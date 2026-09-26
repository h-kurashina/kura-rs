//! 部品の取得元。リポジトリと同じ並び（`registry/<name>.json` と `parts/...`）を持つ場所。
//!
//! - URL（既定は GitHub の raw。サイトのドメインに依存しない）
//! - ローカルのディレクトリ（リポジトリを clone したもの。オフラインやテスト用）

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::paths::SafePath;

/// 既定の取得元。main ブランチの中身をそのまま読む。
pub const DEFAULT_REGISTRY: &str = "https://raw.githubusercontent.com/h-kurashina/kura-rs/main/";
/// 取得元を変える環境変数。
pub const REGISTRY_ENV: &str = "KURA_REGISTRY";
/// 1 ファイルの最大サイズ。これより大きい応答は途中で打ち切ってエラーにする。
pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// GitHub の API（`kura list` で raw の取得元を一覧するときに使う）。
pub const GITHUB_API: &str = "https://api.github.com";

/// 取得に失敗した理由。
#[derive(Debug)]
pub enum FetchError {
    /// 取得元にそのファイルがない（HTTP 404 やファイルなし）。
    NotFound(String),
    Other(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::NotFound(what) => write!(f, "{what}: not found"),
            FetchError::Other(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for FetchError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// 末尾が `/` の URL。
    Http(String),
    Local(PathBuf),
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Http(url) => f.write_str(url),
            Source::Local(dir) => write!(f, "{}", dir.display()),
        }
    }
}

impl Source {
    /// `--registry`、環境変数 `KURA_REGISTRY`、既定値の順に決める。
    pub fn resolve(flag: Option<&str>) -> Result<Self, String> {
        let env = std::env::var(REGISTRY_ENV).ok().filter(|s| !s.is_empty());
        Self::parse(flag.or(env.as_deref()).unwrap_or(DEFAULT_REGISTRY))
    }

    /// `http://` か `https://` で始まれば URL、それ以外の `scheme://` はエラー、残りはディレクトリ。
    pub fn parse(s: &str) -> Result<Self, String> {
        let lower = s.to_ascii_lowercase();
        if lower.starts_with("https://") || lower.starts_with("http://") {
            let mut url = s.to_owned();
            if url.contains(['?', '#']) {
                return Err(format!("registry URL must not contain '?' or '#': {s}"));
            }
            if !url.ends_with('/') {
                url.push('/');
            }
            return Ok(Source::Http(url));
        }
        if s.contains("://") {
            return Err(format!(
                "unsupported registry {s:?}: use an http(s):// URL or a local directory"
            ));
        }
        let dir = PathBuf::from(s);
        if !dir.is_dir() {
            return Err(format!(
                "registry directory {} does not exist (expected a copy of the kura repository with registry/ and parts/)",
                dir.display()
            ));
        }
        Ok(Source::Local(dir))
    }

    /// 取得元のルートからの相対パスでファイルを読む。
    pub fn fetch(&self, path: &SafePath) -> Result<Vec<u8>, FetchError> {
        match self {
            Source::Http(base) => http_get(&format!("{base}{}", path.as_slash_str()), None),
            Source::Local(root) => read_local(root, path),
        }
    }

    /// `registry/<name>.json` を読む。
    pub fn fetch_part_json(&self, name: &str) -> Result<Vec<u8>, FetchError> {
        let path = SafePath::parse(&format!("registry/{name}.json"))
            .map_err(|e| FetchError::Other(e.to_string()))?;
        self.fetch(&path)
    }
}

/// ローカルのファイルを読む。シンボリックリンクで取得元の外に出るものは読まない。
fn read_local(root: &Path, path: &SafePath) -> Result<Vec<u8>, FetchError> {
    let file = path.under(root);
    let shown = path.as_slash_str();
    let canonical_root = root
        .canonicalize()
        .map_err(|e| FetchError::Other(format!("{}: {e}", root.display())))?;
    let canonical = match file.canonicalize() {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FetchError::NotFound(shown));
        }
        Err(e) => return Err(FetchError::Other(format!("{shown}: {e}"))),
    };
    if !canonical.starts_with(&canonical_root) {
        return Err(FetchError::Other(format!(
            "{shown}: resolves outside the registry directory"
        )));
    }
    if !canonical.is_file() {
        return Err(FetchError::Other(format!("{shown}: not a regular file")));
    }
    let len = canonical
        .metadata()
        .map_err(|e| FetchError::Other(format!("{shown}: {e}")))?
        .len();
    if len > MAX_FILE_BYTES {
        return Err(FetchError::Other(format!(
            "{shown}: larger than {MAX_FILE_BYTES} bytes"
        )));
    }
    std::fs::read(&canonical).map_err(|e| FetchError::Other(format!("{shown}: {e}")))
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .http_status_as_error(false)
        .user_agent(concat!("kura-rs/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// GET して本文を返す。404 は `NotFound`、それ以外の 2xx 以外はエラー。
pub fn http_get(url: &str, accept: Option<&str>) -> Result<Vec<u8>, FetchError> {
    let mut req = agent().get(url);
    if let Some(accept) = accept {
        req = req.header("Accept", accept);
    }
    let mut resp = req
        .call()
        .map_err(|e| FetchError::Other(format!("GET {url}: {e}")))?;
    let status = resp.status().as_u16();
    if status == 404 {
        return Err(FetchError::NotFound(url.to_owned()));
    }
    if !(200..300).contains(&status) {
        return Err(FetchError::Other(format!("GET {url}: HTTP {status}")));
    }
    resp.body_mut()
        .with_config()
        .limit(MAX_FILE_BYTES)
        .read_to_vec()
        .map_err(|e| FetchError::Other(format!("GET {url}: {e}")))
}

/// 一覧に出す 1 部品の要約。
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Summary {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub shelves: Vec<String>,
    #[serde(default)]
    pub sample: bool,
}

/// `registry/index.json`（`kura-registry build` が作る `/r/index.json` と同じ形）。
#[derive(Debug, Deserialize)]
struct Index {
    parts: Vec<Summary>,
}

/// `https://raw.githubusercontent.com/<owner>/<repo>/<ref>/` なら (owner, repo, ref)。
pub fn github_raw(url: &str) -> Option<(String, String, String)> {
    let rest = url.strip_prefix("https://raw.githubusercontent.com/")?;
    let mut it = rest.trim_end_matches('/').splitn(3, '/');
    let (owner, repo, reference) = (it.next()?, it.next()?, it.next()?);
    if [owner, repo, reference].iter().any(|s| s.is_empty()) {
        return None;
    }
    Some((owner.into(), repo.into(), reference.into()))
}

/// 部品名の一覧を得る。
///
/// - ローカル: `registry/*.json` のファイル名
/// - GitHub の raw: GitHub の contents API で `registry/` の中身を見る
///   （`index.json` は生成物でリポジトリにないため）
/// - その他の URL: `registry/index.json` を読む（kura-registry build の出力を置いたミラー向け）
///
/// 一覧のほかに、すでに読めた要約（index.json の場合）も返す。
pub fn list_names(source: &Source, github_api: &str) -> Result<Listing, String> {
    match source {
        Source::Local(root) => {
            let dir = root.join("registry");
            let entries = std::fs::read_dir(&dir)
                .map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
            let mut names = Vec::new();
            for entry in entries {
                let entry = entry.map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
                if let Some(name) = entry
                    .file_name()
                    .to_str()
                    .and_then(|f| f.strip_suffix(".json"))
                {
                    names.push(name.to_owned());
                }
            }
            Ok(Listing::Names(names))
        }
        Source::Http(base) => {
            if let Some((owner, repo, reference)) = github_raw(base) {
                let url = format!(
                    "{}/repos/{owner}/{repo}/contents/registry?ref={reference}",
                    github_api.trim_end_matches('/')
                );
                let body = http_get(&url, Some("application/vnd.github+json"))
                    .map_err(|e| e.to_string())?;
                Ok(Listing::Names(parse_contents_listing(&body)?))
            } else {
                let path = SafePath::parse("registry/index.json").expect("static path");
                let body = source.fetch(&path).map_err(|e| e.to_string())?;
                let index: Index = serde_json::from_slice(&body)
                    .map_err(|e| format!("registry/index.json is not a valid index: {e}"))?;
                Ok(Listing::Summaries(index.parts))
            }
        }
    }
}

/// 一覧の結果。名前だけか、要約まで読めたか。
#[derive(Debug)]
pub enum Listing {
    Names(Vec<String>),
    Summaries(Vec<Summary>),
}

/// GitHub の contents API の応答（配列）から `*.json` のファイル名を取り出す。
pub fn parse_contents_listing(body: &[u8]) -> Result<Vec<String>, String> {
    #[derive(Deserialize)]
    struct Entry {
        name: String,
        #[serde(rename = "type")]
        kind: String,
    }
    let entries: Vec<Entry> = serde_json::from_slice(body)
        .map_err(|e| format!("unexpected response from the GitHub API: {e}"))?;
    Ok(entries
        .into_iter()
        .filter(|e| e.kind == "file")
        .filter_map(|e| e.name.strip_suffix(".json").map(str::to_owned))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_urls() {
        assert_eq!(
            Source::parse("https://example.com/kura").unwrap(),
            Source::Http("https://example.com/kura/".into())
        );
        assert_eq!(
            Source::parse("http://127.0.0.1:8080/").unwrap(),
            Source::Http("http://127.0.0.1:8080/".into())
        );
        assert!(Source::parse("ftp://example.com/").is_err());
        assert!(Source::parse("file:///etc").is_err());
        assert!(Source::parse("https://example.com/?x=1").is_err());
        assert!(Source::parse("/definitely/not/a/dir/kura").is_err());
        assert!(matches!(Source::parse(".").unwrap(), Source::Local(_)));
    }

    #[test]
    fn default_is_github_raw() {
        assert_eq!(
            github_raw(DEFAULT_REGISTRY),
            Some(("h-kurashina".into(), "kura-rs".into(), "main".into()))
        );
        assert_eq!(github_raw("https://example.com/x/"), None);
        assert_eq!(github_raw("https://raw.githubusercontent.com/a/"), None);
        assert_eq!(
            github_raw("https://raw.githubusercontent.com/a/b/feat/x/"),
            Some(("a".into(), "b".into(), "feat/x".into()))
        );
    }

    #[test]
    fn contents_listing() {
        let body = br#"[
            {"name": "minhash.json", "type": "file", "size": 10},
            {"name": "README.md", "type": "file"},
            {"name": "sub", "type": "dir"},
            {"name": "dir.json", "type": "dir"}
        ]"#;
        assert_eq!(parse_contents_listing(body).unwrap(), vec!["minhash"]);
        assert!(parse_contents_listing(br#"{"message": "API rate limit exceeded"}"#).is_err());
        assert!(parse_contents_listing(b"\xff").is_err());
    }
}
