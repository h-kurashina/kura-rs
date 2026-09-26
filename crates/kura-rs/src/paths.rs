//! 名前とパスの安全確認。
//!
//! レジストリの JSON は（既定では）ネット越しに取ってくる信用できない入力で、
//! その中のパスを使ってディスクに書き込む。なので「書いてよい形」だけを許す
//! 許可リスト方式にしている（禁止リスト方式は抜けが出やすい）。

use std::fmt;
use std::path::{Path, PathBuf};

/// パス 1 区切り（ディレクトリ名・ファイル名）の最大長。多くのファイルシステムの上限に合わせる。
pub const MAX_SEGMENT_LEN: usize = 255;
/// パス全体の最大長。
pub const MAX_PATH_LEN: usize = 1024;
/// 部品名の最大長。
pub const MAX_NAME_LEN: usize = 64;

/// Windows で予約されているデバイス名。拡張子を付けても使えない（`nul.rs` など）。
const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// 部品名が kebab-case（a-z, 0-9, 区切りは `-` 1 つ）かどうか。kura-schema の規則と同じ。
pub fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_NAME_LEN
        && s.split('-').all(|seg| {
            !seg.is_empty()
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// 部品名が使えないときのエラー。
pub fn check_part_name(name: &str) -> Result<(), String> {
    if is_kebab_case(name) {
        Ok(())
    } else {
        Err(format!(
            "invalid part name {name:?}: must be kebab-case (a-z, 0-9 and single '-', e.g. `minhash` or `file-hash`)"
        ))
    }
}

/// 安全と確認できた相対パス。`/` 区切りの部品（segment）の並び。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SafePath(Vec<String>);

/// パスが安全でない理由。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsafePath {
    pub path: String,
    pub reason: &'static str,
}

impl fmt::Display for UnsafePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsafe path {:?}: {}", self.path, self.reason)
    }
}

impl std::error::Error for UnsafePath {}

impl SafePath {
    /// レジストリに書かれた相対パス（例: `parts/minhash/mod.rs`）を確認する。
    ///
    /// 許すのは `/` で区切った、英数字・`_`・`-`・`.` だけからなる区切りの並び。
    /// 絶対パス、`..` や `.`、空の区切り、バックスラッシュ、ドライブ名（`C:`）、
    /// `.` で始まる隠しファイル、Windows の予約名は認めない。
    pub fn parse(path: &str) -> Result<Self, UnsafePath> {
        let err = |reason| {
            Err(UnsafePath {
                path: path.to_owned(),
                reason,
            })
        };
        if path.is_empty() {
            return err("empty path");
        }
        if path.len() > MAX_PATH_LEN {
            return err("path is too long");
        }
        if path.starts_with('/') {
            return err("absolute paths are not allowed");
        }
        if path.contains('\\') {
            return err("backslashes are not allowed");
        }
        let mut segments = Vec::new();
        for seg in path.split('/') {
            if seg.is_empty() {
                return err("empty path segment (leading, trailing or double '/')");
            }
            if seg == "." || seg == ".." {
                return err("'.' and '..' segments are not allowed");
            }
            if seg.len() > MAX_SEGMENT_LEN {
                return err("path segment is too long");
            }
            if !seg
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            {
                return err("only ASCII letters, digits, '_', '-', '.' and '/' are allowed");
            }
            if seg.starts_with('.') {
                return err("hidden files (starting with '.') are not allowed");
            }
            if seg.ends_with('.') {
                return err("segments ending with '.' are not allowed");
            }
            let stem = seg.split('.').next().unwrap_or(seg).to_ascii_lowercase();
            if WINDOWS_RESERVED.contains(&stem.as_str()) {
                return err("reserved device name on Windows");
            }
            segments.push(seg.to_owned());
        }
        Ok(Self(segments))
    }

    pub fn segments(&self) -> &[String] {
        &self.0
    }

    /// `root` の下に置いたパス。区切りは確認済みなので `root` の外には出ない。
    pub fn under(&self, root: &Path) -> PathBuf {
        let mut p = root.to_path_buf();
        for seg in &self.0 {
            p.push(seg);
        }
        p
    }

    /// URL などに使う `/` 区切りの文字列。
    pub fn as_slash_str(&self) -> String {
        self.0.join("/")
    }

    /// 先頭から `n` 区切りを除いたパス。区切りが残らないなら None。
    fn strip_prefix(&self, n: usize) -> Option<SafePath> {
        (n < self.0.len()).then(|| SafePath(self.0[n..].to_vec()))
    }
}

impl fmt::Display for SafePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.as_slash_str())
    }
}

/// 部品のファイル群を、書き込み先ディレクトリからの相対パスに直す。
///
/// 全ファイルに共通するディレクトリ（例: `parts/minhash/`）を取り除き、その下の構成は保つ。
/// `parts/minhash/mod.rs` → `mod.rs`、`parts/x/sub/a.rs` → `sub/a.rs`。
/// ファイルが 1 つだけなら、そのファイル名だけが残る。
///
/// 大文字・小文字だけが違う同じ行き先（macOS や Windows では同じファイル）は拒む。
pub fn install_layout(files: &[SafePath]) -> Result<Vec<SafePath>, String> {
    let Some(first) = files.first() else {
        return Err("the part lists no files".into());
    };
    // 最後の区切り（ファイル名）はディレクトリではないので、共通部分の候補から外す
    let mut common = first.segments().len() - 1;
    for f in &files[1..] {
        let dir = &f.segments()[..f.segments().len() - 1];
        common = common.min(
            first.segments()[..common]
                .iter()
                .zip(dir)
                .take_while(|(a, b)| a == b)
                .count(),
        );
    }
    let mut out = Vec::with_capacity(files.len());
    let mut seen = std::collections::HashSet::new();
    for f in files {
        let rel = f
            .strip_prefix(common)
            .ok_or_else(|| format!("cannot place {f}"))?;
        if !seen.insert(rel.as_slash_str().to_ascii_lowercase()) {
            return Err(format!(
                "two files would be written to the same place: {rel} (names must differ by more than letter case)"
            ));
        }
        out.push(rel);
    }
    Ok(out)
}

/// 部品名から Rust のモジュール名を作る（`file-hash` → `file_hash`）。
pub fn module_name(part: &str) -> String {
    part.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(s: &str) -> SafePath {
        SafePath::parse(s).unwrap()
    }

    #[test]
    fn accepts_normal_paths() {
        for p in [
            "parts/minhash/mod.rs",
            "parts/minhash/permutation.rs",
            "a.rs",
            "a/b/c/d.rs",
            "parts/file_hash/mod.rs",
            "x-y/z_w.v1.rs",
            "README",
        ] {
            assert!(SafePath::parse(p).is_ok(), "{p}");
        }
    }

    #[test]
    fn rejects_unsafe_paths() {
        for p in [
            "",
            "/",
            "/etc/passwd",
            "//server/share",
            "../x",
            "..",
            ".",
            "./x",
            "a/../../b",
            "a/..",
            "a/./b",
            "a//b",
            "a/",
            "parts\\..\\x",
            "..\\x",
            "C:/Windows/x",
            "C:x",
            "a/b:c",
            "a b",
            "a\0b",
            "a\nb",
            ".hidden",
            "a/.git/config",
            "a/...",
            "a/b.",
            "nul",
            "parts/CON.rs",
            "parts/lpt1",
            "héllo.rs",
            "a/~/b",
            "%2e%2e/x",
            "a?b",
            "a*b",
            "a|b",
            "\"a\"",
        ] {
            assert!(SafePath::parse(p).is_err(), "{p:?} should be rejected");
        }
    }

    #[test]
    fn rejects_too_long() {
        assert!(SafePath::parse(&"a".repeat(MAX_SEGMENT_LEN + 1)).is_err());
        assert!(SafePath::parse(&"a".repeat(MAX_SEGMENT_LEN)).is_ok());
        let long = vec!["abcd"; MAX_PATH_LEN / 5 + 2].join("/");
        assert!(SafePath::parse(&long).is_err());
    }

    #[test]
    fn under_stays_inside_root() {
        let root = Path::new("/tmp/project");
        assert_eq!(
            sp("parts/minhash/mod.rs").under(root),
            Path::new("/tmp/project/parts/minhash/mod.rs")
        );
    }

    #[test]
    fn kebab_case() {
        for ok in [
            "minhash",
            "file-hash",
            "a",
            "a1",
            "1a",
            "multi-pattern-match",
        ] {
            assert!(is_kebab_case(ok), "{ok}");
        }
        for bad in [
            "", "-", "a-", "-a", "a--b", "MinHash", "min_hash", "min hash", "../x", "a/b", "a.b",
            "ａ", "é",
        ] {
            assert!(!is_kebab_case(bad), "{bad}");
        }
        assert!(!is_kebab_case(&"a".repeat(MAX_NAME_LEN + 1)));
    }

    #[test]
    fn layout_strips_common_directory() {
        let files = [
            sp("parts/minhash/mod.rs"),
            sp("parts/minhash/permutation.rs"),
        ];
        let layout = install_layout(&files).unwrap();
        assert_eq!(layout, vec![sp("mod.rs"), sp("permutation.rs")]);
    }

    #[test]
    fn layout_keeps_subdirectories() {
        let files = [sp("parts/x/mod.rs"), sp("parts/x/sub/a.rs")];
        assert_eq!(
            install_layout(&files).unwrap(),
            vec![sp("mod.rs"), sp("sub/a.rs")]
        );
    }

    #[test]
    fn layout_single_file_and_root_file() {
        assert_eq!(
            install_layout(&[sp("parts/x/mod.rs")]).unwrap(),
            vec![sp("mod.rs")]
        );
        assert_eq!(install_layout(&[sp("lib.rs")]).unwrap(), vec![sp("lib.rs")]);
        assert_eq!(
            install_layout(&[sp("a/x.rs"), sp("b/y.rs")]).unwrap(),
            vec![sp("a/x.rs"), sp("b/y.rs")]
        );
    }

    #[test]
    fn layout_rejects_duplicates() {
        assert!(install_layout(&[sp("p/mod.rs"), sp("p/mod.rs")]).is_err());
        assert!(install_layout(&[sp("p/mod.rs"), sp("p/MOD.rs")]).is_err());
        assert!(install_layout(&[]).is_err());
    }

    #[test]
    fn module_names() {
        assert_eq!(module_name("file-hash"), "file_hash");
        assert_eq!(module_name("minhash"), "minhash");
    }
}
