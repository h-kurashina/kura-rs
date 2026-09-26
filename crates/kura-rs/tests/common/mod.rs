//! 統合テストの共通部品。
//!
//! - [`Registry`] — このリポジトリの registry/ と parts/ を一時ディレクトリに写した取得元
//! - [`Project`] — 空の Cargo プロジェクト
//! - [`kura`] — 本物のバイナリを動かすコマンド（偽の `cargo` を使う）
//! - [`Server`] — std::net だけで書いた小さな HTTP サーバー

#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use tempfile::TempDir;

/// リポジトリのルート（crates/kura-rs の 2 つ上）。
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// 一時ディレクトリに作ったレジストリ（取得元）。
pub struct Registry {
    pub dir: TempDir,
}

impl Registry {
    /// リポジトリの registry/ と parts/ をそのまま写す。
    pub fn copy_of_repo() -> Self {
        let dir = tempfile::tempdir().unwrap();
        copy_dir(&repo_root().join("registry"), &dir.path().join("registry"));
        copy_dir(&repo_root().join("parts"), &dir.path().join("parts"));
        Self { dir }
    }

    /// 空のレジストリ（registry/ だけある）。
    pub fn empty() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("registry")).unwrap();
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn arg(&self) -> String {
        self.path().to_str().unwrap().to_owned()
    }

    /// 取得元にファイルを置く（ディレクトリも作る）。
    pub fn write(&self, rel: &str, contents: impl AsRef<[u8]>) {
        let path = self.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// 部品の JSON を置く。`files` と `deps` 以外は妥当な値で埋める。
    pub fn write_part(&self, name: &str, files: &[&str], deps: &str) {
        self.write(
            &format!("registry/{name}.json"),
            part_json(name, files, deps),
        );
    }

    pub fn read(&self, rel: &str) -> Vec<u8> {
        fs::read(self.path().join(rel)).unwrap()
    }
}

/// テスト用の部品 JSON。`deps` は JSON 配列の中身（例: `{"name":"x","version":"1"}`）。
pub fn part_json(name: &str, files: &[&str], deps: &str) -> String {
    let files = serde_json::to_string(files).unwrap();
    format!(
        r#"{{
  "name": {name},
  "title": "Test part",
  "description": "A part for tests.",
  "version": "0.1.0",
  "added": "2026-09-26",
  "sample": true,
  "shelves": ["ai"],
  "rust": {{ "files": {files}, "dependencies": [{deps}] }},
  "reference": {{ "name": "ref", "language": "Python", "version": "1" }},
  "verification": {{ "method": "golden", "cases": 1, "passed": 1, "last_run": "2026-09-26T00:00:00Z" }},
  "benchmarks": [{{ "input_size": 1, "rust_ms": 1.0, "reference_ms": 2.0 }}],
  "usage": {{ "rust": "use crate::parts::demo::run;" }}
}}"#,
        name = serde_json::to_string(name).unwrap()
    )
}

/// 空の Cargo プロジェクト（Cargo.toml と src/main.rs）。
pub struct Project {
    pub dir: TempDir,
    /// 偽の cargo とその記録を置く場所（プロジェクトの中を汚さないよう別にする）。
    pub aux: TempDir,
}

pub const EMPTY_MANIFEST: &str =
    "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n";

impl Project {
    pub fn new() -> Self {
        let p = Self::bare();
        p.write("Cargo.toml", EMPTY_MANIFEST);
        p.write("src/main.rs", "fn main() {}\n");
        p
    }

    /// Cargo.toml のないディレクトリ。
    pub fn bare() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            aux: tempfile::tempdir().unwrap(),
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn write(&self, rel: &str, contents: impl AsRef<[u8]>) {
        let path = self.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path().join(rel)).unwrap()
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.path().join(rel).exists()
    }

    /// ディレクトリ以下の全ファイル（相対パス、並べ替え済み）。
    pub fn files(&self) -> Vec<String> {
        list_files(self.path())
    }

    /// 偽の cargo が受け取った引数（1 回の呼び出しが 1 行）。
    pub fn cargo_calls(&self) -> Vec<String> {
        fs::read_to_string(self.aux.path().join(CARGO_LOG))
            .map(|s| s.lines().map(str::to_owned).collect())
            .unwrap_or_default()
    }
}

pub fn list_files(root: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &path, out);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    if root.exists() {
        walk(root, root, &mut out);
    }
    out.sort();
    out
}

/// 偽の cargo が呼び出しを書き残すファイル（`Project::aux` の中）。
pub const CARGO_LOG: &str = "cargo-calls.log";

/// 偽の `cargo`。引数をログに書き、`KURA_TEST_CARGO_EXIT` の終了コードで終わる。
#[cfg(unix)]
fn fake_cargo(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("fake-cargo.sh");
    fs::write(
        &path,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$KURA_TEST_CARGO_LOG\"\nexit \"${KURA_TEST_CARGO_EXIT:-0}\"\n",
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// バイナリ `kura` を `project` の中で動かす準備をする。
/// `KURA_REGISTRY` は消し、`cargo` は偽物に差し替える（ネットにつながないため）。
pub fn kura(project: &Project) -> Command {
    kura_bin(env!("CARGO_BIN_EXE_kura"), project)
}

pub fn kura_bin(bin: &str, project: &Project) -> Command {
    let mut cmd = Command::new(bin);
    cmd.current_dir(project.path())
        .env_remove("KURA_REGISTRY")
        .env("KURA_TEST_CARGO_LOG", project.aux.path().join(CARGO_LOG));
    #[cfg(unix)]
    cmd.env("CARGO", fake_cargo(project.aux.path()));
    cmd
}

/// 実行結果を読みやすくしたもの。
pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn ok(&self) -> bool {
        self.code == Some(0)
    }

    pub fn assert_ok(&self) -> &Self {
        assert!(self.ok(), "expected success, got {self:?}");
        self
    }

    pub fn assert_fail(&self) -> &Self {
        assert_eq!(self.code, Some(1), "expected exit code 1, got {self:?}");
        self
    }

    pub fn stderr_has(&self, needle: &str) -> &Self {
        assert!(
            self.stderr.contains(needle),
            "stderr should contain {needle:?}: {self:?}"
        );
        self
    }

    pub fn stdout_has(&self, needle: &str) -> &Self {
        assert!(
            self.stdout.contains(needle),
            "stdout should contain {needle:?}: {self:?}"
        );
        self
    }
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "exit {:?}\n--- stdout\n{}\n--- stderr\n{}",
            self.code, self.stdout, self.stderr
        )
    }
}

pub fn run(cmd: &mut Command) -> Run {
    let Output {
        status,
        stdout,
        stderr,
    } = cmd.output().unwrap();
    Run {
        code: status.code(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    }
}

/// サーバーが返す応答。
#[derive(Clone)]
pub enum Reply {
    /// ステータスと本文。
    Body(u16, Vec<u8>),
    /// 何も返さずに接続を切る。
    Hangup,
    /// ヘッダーで長さを宣言しておきながら、途中で切る。
    Truncated(usize, Vec<u8>),
}

/// std::net だけで書いた HTTP/1.1 サーバー。テストの間だけ動く。
pub struct Server {
    pub url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

type Handler = dyn Fn(&str) -> Reply + Send + Sync;

impl Server {
    pub fn start(handler: impl Fn(&str) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&requests);
        let handler: Arc<Handler> = Arc::new(handler);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let handler = Arc::clone(&handler);
                let log = Arc::clone(&log);
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).is_err() {
                        return;
                    }
                    // ヘッダーを読み捨てる
                    loop {
                        let mut line = String::new();
                        match reader.read_line(&mut line) {
                            Ok(0) | Err(_) => break,
                            Ok(_) if line == "\r\n" || line == "\n" => break,
                            Ok(_) => {}
                        }
                    }
                    let path = request_line
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("/")
                        .to_owned();
                    log.lock().unwrap().push(path.clone());
                    let reply = handler(&path);
                    let _ = match reply {
                        Reply::Hangup => Ok(()),
                        Reply::Body(status, body) => {
                            write_response(&mut stream, status, body.len(), &body)
                        }
                        Reply::Truncated(declared, body) => {
                            write_response(&mut stream, 200, declared, &body)
                        }
                    };
                    let _ = stream.flush();
                    // 残りを読み捨ててから閉じる（RST で応答が消えないように）
                    let _ = stream.shutdown(std::net::Shutdown::Write);
                    let mut sink = [0u8; 1024];
                    let _ = reader.get_mut().read(&mut sink);
                });
            }
        });
        Self { url, requests }
    }

    /// ディレクトリの中身をそのまま配る。`overrides` のパスはその応答を優先する。
    pub fn serve_dir(root: &Path, overrides: HashMap<String, Reply>) -> Self {
        let root = root.to_path_buf();
        Self::start(move |path| {
            if let Some(reply) = overrides.get(path) {
                return reply.clone();
            }
            let rel = path.trim_start_matches('/');
            if rel.split('/').any(|s| s == ".." || s.is_empty()) {
                return Reply::Body(400, b"bad path".to_vec());
            }
            match fs::read(root.join(rel)) {
                Ok(body) => Reply::Body(200, body),
                Err(_) => Reply::Body(404, b"404: Not Found".to_vec()),
            }
        })
    }

    /// 受け取ったリクエストのパス。
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

fn write_response(
    stream: &mut impl Write,
    status: u16,
    declared_len: usize,
    body: &[u8],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} X\r\nContent-Length: {declared_len}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n"
    )?;
    stream.write_all(body)
}
