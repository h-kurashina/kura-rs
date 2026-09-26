//! URL の取得元のテスト。テストの中で小さな HTTP サーバーを立てて、本物のバイナリから取りに行く。
#![cfg(unix)]

mod common;

use std::collections::HashMap;

use common::{Project, Registry, Reply, Server, kura, repo_root, run};
use kura_rs::source::{self, Listing, Source};

fn serve_repo(overrides: HashMap<String, Reply>) -> (Registry, Server) {
    let reg = Registry::copy_of_repo();
    let server = Server::serve_dir(reg.path(), overrides);
    (reg, server)
}

fn one(path: &str, reply: Reply) -> HashMap<String, Reply> {
    HashMap::from([(path.to_owned(), reply)])
}

fn add_from(server: &Server, project: &Project, extra: &[&str]) -> common::Run {
    run(kura(project)
        .args(["add"])
        .args(extra)
        .args(["--registry", &server.url]))
}

#[test]
fn adds_minhash_over_http() {
    let (_reg, server) = serve_repo(HashMap::new());
    let project = Project::new();
    add_from(&server, &project, &["minhash"])
        .assert_ok()
        .stdout_has(&format!("from {}", server.url))
        .stdout_has("cargo add sha1@0.11");
    assert_eq!(
        project.read("src/parts/minhash/mod.rs"),
        std::fs::read_to_string(repo_root().join("parts/minhash/mod.rs")).unwrap()
    );
    assert_eq!(
        server.requests(),
        [
            "/registry/minhash.json",
            "/parts/minhash/mod.rs",
            "/parts/minhash/permutation.rs"
        ]
    );
}

#[test]
fn base_url_with_path_and_without_trailing_slash() {
    let reg = Registry::copy_of_repo();
    let root = reg.path().to_path_buf();
    let server = Server::start(move |path| match path.strip_prefix("/mirror/kura/") {
        Some(rel) => match std::fs::read(root.join(rel)) {
            Ok(body) => Reply::Body(200, body),
            Err(_) => Reply::Body(404, vec![]),
        },
        None => Reply::Body(404, vec![]),
    });
    let project = Project::new();
    let url = format!("{}mirror/kura", server.url);
    run(kura(&project).args(["add", "minhash", "--no-deps", "--registry", &url])).assert_ok();
    assert!(project.exists("src/parts/minhash/permutation.rs"));
}

#[test]
fn registry_url_from_environment_variable() {
    let (_reg, server) = serve_repo(HashMap::new());
    let project = Project::new();
    run(kura(&project)
        .args(["add", "minhash", "--no-deps"])
        .env("KURA_REGISTRY", &server.url))
    .assert_ok();
    assert!(project.exists("src/parts/minhash/mod.rs"));
}

#[test]
fn dry_run_over_http_fetches_but_writes_nothing() {
    let (_reg, server) = serve_repo(HashMap::new());
    let project = Project::new();
    add_from(&server, &project, &["minhash", "--dry-run"]).assert_ok();
    assert_eq!(project.files(), ["Cargo.toml", "src/main.rs"]);
    assert!(project.cargo_calls().is_empty());
}

#[test]
fn unknown_part_is_404() {
    let (_reg, server) = serve_repo(HashMap::new());
    let project = Project::new();
    add_from(&server, &project, &["no-such-part"])
        .assert_fail()
        .stderr_has("part `no-such-part` not found");
}

#[test]
fn missing_source_file_is_404_and_nothing_is_written() {
    let (_reg, server) = serve_repo(one(
        "/parts/minhash/permutation.rs",
        Reply::Body(404, b"404: Not Found".to_vec()),
    ));
    let project = Project::new();
    add_from(&server, &project, &["minhash"])
        .assert_fail()
        .stderr_has("file parts/minhash/permutation.rs listed by part `minhash` is missing");
    assert_eq!(project.files(), ["Cargo.toml", "src/main.rs"]);
    assert!(project.cargo_calls().is_empty());
}

#[test]
fn server_error_is_reported() {
    for status in [500, 502, 503, 403, 401, 410] {
        let (_reg, server) = serve_repo(one(
            "/registry/minhash.json",
            Reply::Body(status, b"oops".to_vec()),
        ));
        let project = Project::new();
        add_from(&server, &project, &["minhash"])
            .assert_fail()
            .stderr_has(&format!("HTTP {status}"));
    }
}

#[test]
fn server_error_on_source_file() {
    let (_reg, server) = serve_repo(one(
        "/parts/minhash/mod.rs",
        Reply::Body(500, b"oops".to_vec()),
    ));
    let project = Project::new();
    add_from(&server, &project, &["minhash"])
        .assert_fail()
        .stderr_has("cannot fetch parts/minhash/mod.rs");
    assert!(!project.exists("src/parts"));
}

#[test]
fn garbage_json_responses_are_rejected() {
    for body in [
        b"\xff\xfe\xfd\xfc\x00\x01".to_vec(),
        b"<!DOCTYPE html><title>Login</title>".to_vec(),
        b"{\"name\": \"minhash\"".to_vec(),
        vec![0u8; 1000],
        Vec::new(),
    ] {
        let (_reg, server) = serve_repo(one("/registry/minhash.json", Reply::Body(200, body)));
        let project = Project::new();
        add_from(&server, &project, &["minhash"])
            .assert_fail()
            .stderr_has("registry/minhash.json is invalid");
        assert!(!project.exists("src/parts"));
    }
}

#[test]
fn non_utf8_source_file_response_is_rejected() {
    let (_reg, server) = serve_repo(one(
        "/parts/minhash/mod.rs",
        Reply::Body(200, b"\x7fELF\x02\x01\x01\x00\xff\xff".to_vec()),
    ));
    let project = Project::new();
    add_from(&server, &project, &["minhash"])
        .assert_fail()
        .stderr_has("not valid UTF-8");
    assert!(!project.exists("src/parts"));
}

#[test]
fn oversized_response_is_cut_off() {
    let big = vec![b'/'; (source::MAX_FILE_BYTES + 1) as usize];
    let (_reg, server) = serve_repo(one("/parts/minhash/mod.rs", Reply::Body(200, big)));
    let project = Project::new();
    add_from(&server, &project, &["minhash"])
        .assert_fail()
        .stderr_has("cannot fetch parts/minhash/mod.rs");
    assert!(!project.exists("src/parts"));
}

#[test]
fn connection_dropped_without_response() {
    let (_reg, server) = serve_repo(one("/registry/minhash.json", Reply::Hangup));
    let project = Project::new();
    add_from(&server, &project, &["minhash"])
        .assert_fail()
        .stderr_has("cannot fetch part `minhash`");
}

#[test]
fn truncated_body_is_an_error() {
    let (_reg, server) = serve_repo(one(
        "/parts/minhash/mod.rs",
        Reply::Truncated(10_000, b"pub fn half".to_vec()),
    ));
    let project = Project::new();
    add_from(&server, &project, &["minhash"])
        .assert_fail()
        .stderr_has("cannot fetch parts/minhash/mod.rs");
    assert!(!project.exists("src/parts"));
}

#[test]
fn nothing_listening_is_an_error() {
    // 使ったあと閉じたポート
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let project = Project::new();
    run(kura(&project).args([
        "add",
        "minhash",
        "--registry",
        &format!("http://127.0.0.1:{port}/"),
    ]))
    .assert_fail()
    .stderr_has("cannot fetch part `minhash`");
}

#[test]
fn malicious_paths_are_never_requested() {
    for bad in ["../x", "/etc/passwd", "a/../../b", "parts\\..\\x", ""] {
        let reg = Registry::empty();
        reg.write_part("evil", &[bad], "");
        let server = Server::serve_dir(reg.path(), HashMap::new());
        let project = Project::new();
        add_from(&server, &project, &["evil"])
            .assert_fail()
            .stderr_has("rust.files[0]");
        // 部品の JSON 以外は取りに行っていない
        assert_eq!(server.requests(), ["/registry/evil.json"], "{bad:?}");
        assert_eq!(project.files(), ["Cargo.toml", "src/main.rs"]);
    }
}

#[test]
fn invalid_name_is_never_requested() {
    let (_reg, server) = serve_repo(HashMap::new());
    let project = Project::new();
    add_from(&server, &project, &["../registry/minhash"])
        .assert_fail()
        .stderr_has("kebab-case");
    assert!(server.requests().is_empty());
}

// ---------------------------------------------------------------- list

fn index_json() -> Vec<u8> {
    let parts = kura_schema::load_dir(&repo_root().join("registry")).unwrap();
    kura_schema::to_published_json(&kura_schema::PartIndex::new(&parts))
        .unwrap()
        .into_bytes()
}

#[test]
fn list_over_http_reads_index_json() {
    let (_reg, server) = serve_repo(one("/registry/index.json", Reply::Body(200, index_json())));
    let project = Project::new();
    let r = run(kura(&project).args(["list", "--registry", &server.url]));
    r.assert_ok().stdout_has("minhash").stdout_has("MinHash");
    assert_eq!(server.requests(), ["/registry/index.json"]);
}

#[test]
fn list_over_http_without_index_fails() {
    let (_reg, server) = serve_repo(HashMap::new());
    let project = Project::new();
    run(kura(&project).args(["list", "--registry", &server.url]))
        .assert_fail()
        .stderr_has("not found");
}

#[test]
fn list_over_http_with_garbage_index_fails() {
    let (_reg, server) = serve_repo(one(
        "/registry/index.json",
        Reply::Body(200, b"\xff\x00garbage".to_vec()),
    ));
    let project = Project::new();
    run(kura(&project).args(["list", "--registry", &server.url]))
        .assert_fail()
        .stderr_has("not a valid index");
}

#[test]
fn list_skips_invalid_names_in_index() {
    let index = br#"{"parts": [
        {"name": "ok-part", "title": "OK", "shelves": ["ai"], "sample": false},
        {"name": "../evil", "title": "Evil", "shelves": [], "sample": false}
    ]}"#;
    let (_reg, server) = serve_repo(one(
        "/registry/index.json",
        Reply::Body(200, index.to_vec()),
    ));
    let project = Project::new();
    let r = run(kura(&project).args(["list", "--registry", &server.url]));
    r.assert_ok()
        .stdout_has("ok-part")
        .stderr_has("skipping invalid part name");
    assert!(!r.stdout.contains("evil"), "{r:?}");
}

/// GitHub の raw の取得元では contents API で一覧を得る。API の場所をテスト用サーバーに向ける。
#[test]
fn list_for_github_raw_uses_contents_api() {
    let contents = br#"[
        {"name": "minhash.json", "path": "registry/minhash.json", "type": "file"},
        {"name": "file-hash.json", "path": "registry/file-hash.json", "type": "file"},
        {"name": "notes.md", "path": "registry/notes.md", "type": "file"}
    ]"#;
    let server = Server::start(move |_| Reply::Body(200, contents.to_vec()));
    let src = Source::parse("https://raw.githubusercontent.com/h-kurashina/kura-rs/main/").unwrap();
    let listing = source::list_names(&src, server.url.trim_end_matches('/')).unwrap();
    let Listing::Names(mut names) = listing else {
        panic!("expected names")
    };
    names.sort();
    assert_eq!(names, ["file-hash", "minhash"]);
    assert_eq!(
        server.requests(),
        ["/repos/h-kurashina/kura-rs/contents/registry?ref=main"]
    );
}

#[test]
fn list_for_github_raw_reports_api_errors() {
    let server =
        Server::start(|_| Reply::Body(403, br#"{"message": "API rate limit exceeded"}"#.to_vec()));
    let src = Source::parse("https://raw.githubusercontent.com/h-kurashina/kura-rs/main/").unwrap();
    let err = source::list_names(&src, &server.url).unwrap_err();
    assert!(err.contains("HTTP 403"), "{err}");
}

#[test]
fn list_for_github_raw_rejects_garbage() {
    let server = Server::start(|_| Reply::Body(200, b"\xff\xfe".to_vec()));
    let src = Source::parse("https://raw.githubusercontent.com/h-kurashina/kura-rs/main/").unwrap();
    assert!(source::list_names(&src, &server.url).is_err());
}
