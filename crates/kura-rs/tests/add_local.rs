//! 本物のバイナリを、ローカルのディレクトリを取得元にして動かすテスト。
//! `cargo add` は偽物（引数を記録するだけのシェルスクリプト）に差し替えるので、Unix だけで動かす。
#![cfg(unix)]

mod common;

use common::{Project, Registry, kura, kura_bin, repo_root, run};

fn minhash_files() -> [(&'static str, Vec<u8>); 2] {
    let root = repo_root();
    [
        (
            "src/parts/minhash/mod.rs",
            std::fs::read(root.join("parts/minhash/mod.rs")).unwrap(),
        ),
        (
            "src/parts/minhash/permutation.rs",
            std::fs::read(root.join("parts/minhash/permutation.rs")).unwrap(),
        ),
    ]
}

fn add(project: &Project, registry: &Registry, extra: &[&str]) -> common::Run {
    run(kura(project)
        .args(["add"])
        .args(extra)
        .args(["--registry", &registry.arg()]))
}

// ---------------------------------------------------------------- 正常系

#[test]
fn adds_minhash_into_empty_project() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    let r = add(&project, &reg, &["minhash"]);
    r.assert_ok()
        .stdout_has("create    src/parts/minhash/mod.rs")
        .stdout_has("create    src/parts/minhash/permutation.rs")
        .stdout_has("cargo add sha1@0.11")
        .stdout_has("Add `pub mod minhash;` to src/parts/mod.rs")
        .stdout_has("Add `mod parts;` to src/main.rs or src/lib.rs")
        .stdout_has("use crate::parts::minhash::MinHasher;");
    for (rel, expected) in minhash_files() {
        assert_eq!(
            std::fs::read(project.path().join(rel)).unwrap(),
            expected,
            "{rel}"
        );
    }
    let calls = project.cargo_calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let manifest = project.path().canonicalize().unwrap().join("Cargo.toml");
    assert_eq!(
        calls[0],
        format!("add --manifest-path {} sha1@0.11", manifest.display())
    );
    // Cargo.toml と main.rs には手を付けない（依存の追記は cargo add の仕事）
    assert_eq!(project.read("Cargo.toml"), common::EMPTY_MANIFEST);
    assert_eq!(project.read("src/main.rs"), "fn main() {}\n");
}

#[test]
fn kura_rs_binary_behaves_the_same() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    run(kura_bin(env!("CARGO_BIN_EXE_kura-rs"), &project).args([
        "add",
        "minhash",
        "--no-deps",
        "--registry",
        &reg.arg(),
    ]))
    .assert_ok();
    assert!(project.exists("src/parts/minhash/mod.rs"));
}

#[test]
fn registry_from_environment_variable() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    run(kura(&project)
        .args(["add", "minhash", "--no-deps"])
        .env("KURA_REGISTRY", reg.path()))
    .assert_ok();
    assert!(project.exists("src/parts/minhash/mod.rs"));
}

#[test]
fn registry_flag_wins_over_environment_variable() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    run(kura(&project)
        .args(["add", "minhash", "--no-deps", "--registry", &reg.arg()])
        .env("KURA_REGISTRY", "/definitely/not/here"))
    .assert_ok();
}

#[test]
fn works_from_a_subdirectory_of_the_project() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    project.write("src/deep/keep.txt", "");
    run(kura(&project)
        .current_dir(project.path().join("src/deep"))
        .args(["add", "minhash", "--registry", &reg.arg()]))
    .assert_ok();
    // 書き込み先は Cargo.toml の隣の src/parts/
    assert!(project.exists("src/parts/minhash/mod.rs"));
    assert!(!project.exists("src/deep/src"));
    assert_eq!(project.cargo_calls().len(), 1);
}

#[test]
fn skips_workspace_root_without_package() {
    let reg = Registry::copy_of_repo();
    let project = Project::bare();
    project.write("Cargo.toml", "[workspace]\nmembers = [\"app\"]\n");
    project.write("app/Cargo.toml", common::EMPTY_MANIFEST);
    project.write("app/src/main.rs", "fn main() {}\n");
    run(kura(&project)
        .current_dir(project.path().join("app"))
        .args(["add", "minhash", "--registry", &reg.arg()]))
    .assert_ok();
    assert!(project.exists("app/src/parts/minhash/mod.rs"));
    assert!(project.cargo_calls()[0].contains("app/Cargo.toml"));
}

#[test]
fn next_steps_skip_what_is_already_declared() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    project.write("src/main.rs", "mod parts;\nfn main() {}\n");
    project.write("src/parts/mod.rs", "pub mod minhash;\n");
    let r = add(&project, &reg, &["minhash", "--no-deps"]);
    r.assert_ok();
    assert!(!r.stdout.contains("Add `pub mod minhash;`"), "{r:?}");
    assert!(!r.stdout.contains("Add `mod parts;`"), "{r:?}");
    r.stdout_has("1. Use it:");
}

#[test]
fn sample_parts_are_marked() {
    let reg = Registry::empty();
    reg.write_part("demo", &["parts/demo/mod.rs"], "");
    reg.write("parts/demo/mod.rs", "pub fn run() {}\n");
    let project = Project::new();
    add(&project, &reg, &["demo"])
        .assert_ok()
        .stdout_has("marked as sample data");
}

#[test]
fn kebab_name_becomes_snake_case_module_directory() {
    let reg = Registry::empty();
    reg.write_part("file-hash", &["parts/file_hash/mod.rs"], "");
    reg.write("parts/file_hash/mod.rs", "pub fn run() {}\n");
    let project = Project::new();
    add(&project, &reg, &["file-hash"])
        .assert_ok()
        .stdout_has("Add `pub mod file_hash;`");
    assert_eq!(
        project.files(),
        ["Cargo.toml", "src/main.rs", "src/parts/file_hash/mod.rs"]
    );
}

#[test]
fn keeps_subdirectories_of_the_part() {
    let reg = Registry::empty();
    reg.write_part(
        "demo",
        &["parts/demo/mod.rs", "parts/demo/inner/deep.rs"],
        "",
    );
    reg.write("parts/demo/mod.rs", "mod inner;\n");
    reg.write("parts/demo/inner/deep.rs", "pub fn f() {}\n");
    let project = Project::new();
    add(&project, &reg, &["demo"]).assert_ok();
    assert_eq!(
        project.read("src/parts/demo/inner/deep.rs"),
        "pub fn f() {}\n"
    );
}

#[test]
fn dependencies_with_features_are_passed_to_cargo_add() {
    let reg = Registry::empty();
    reg.write_part(
        "demo",
        &["parts/demo/mod.rs"],
        r#"{"name": "serde", "version": "1", "features": ["derive", "rc"]},
           {"name": "memchr", "version": "2"}"#,
    );
    reg.write("parts/demo/mod.rs", "pub fn run() {}\n");
    let project = Project::new();
    add(&project, &reg, &["demo"]).assert_ok();
    let calls = project.cargo_calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert!(
        calls[0].ends_with("serde@1 --features derive,rc"),
        "{calls:?}"
    );
    assert!(calls[1].ends_with("memchr@2"), "{calls:?}");
}

#[test]
fn part_without_dependencies_does_not_call_cargo() {
    let reg = Registry::empty();
    reg.write_part("demo", &["parts/demo/mod.rs"], "");
    reg.write("parts/demo/mod.rs", "pub fn run() {}\n");
    let project = Project::new();
    let r = add(&project, &reg, &["demo"]);
    r.assert_ok();
    assert!(project.cargo_calls().is_empty());
    assert!(!r.stdout.contains("Dependencies"), "{r:?}");
}

// ---------------------------------------------------------------- --dry-run

#[test]
fn dry_run_writes_nothing_and_runs_nothing() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    let before = project.files();
    add(&project, &reg, &["minhash", "--dry-run"])
        .assert_ok()
        .stdout_has("Would add MinHash")
        .stdout_has("create    src/parts/minhash/mod.rs")
        .stdout_has("would run: cargo add sha1@0.11")
        .stdout_has("Dry run: nothing was written");
    assert_eq!(project.files(), before);
    assert!(project.cargo_calls().is_empty());
}

#[test]
fn dry_run_reports_conflicts_and_fails() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    project.write("src/parts/minhash/mod.rs", "// mine\n");
    add(&project, &reg, &["minhash", "--dry-run"])
        .assert_fail()
        .stderr_has("src/parts/minhash/mod.rs");
    assert_eq!(project.read("src/parts/minhash/mod.rs"), "// mine\n");
}

#[test]
fn dry_run_with_overwrite_shows_overwrite_but_keeps_file() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    project.write("src/parts/minhash/mod.rs", "// mine\n");
    add(&project, &reg, &["minhash", "--dry-run", "--overwrite"])
        .assert_ok()
        .stdout_has("overwrite src/parts/minhash/mod.rs")
        .stdout_has("create    src/parts/minhash/permutation.rs");
    assert_eq!(project.read("src/parts/minhash/mod.rs"), "// mine\n");
    assert!(!project.exists("src/parts/minhash/permutation.rs"));
}

// ---------------------------------------------------------------- 衝突と --overwrite

#[test]
fn conflict_without_overwrite_fails_and_writes_nothing() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    project.write("src/parts/minhash/permutation.rs", "// my own file\n");
    add(&project, &reg, &["minhash"])
        .assert_fail()
        .stderr_has("already exist")
        .stderr_has("src/parts/minhash/permutation.rs")
        .stderr_has("--overwrite");
    assert_eq!(
        project.read("src/parts/minhash/permutation.rs"),
        "// my own file\n"
    );
    // ぶつからないファイルも書かない（中途半端な状態を残さない）
    assert!(!project.exists("src/parts/minhash/mod.rs"));
    assert!(project.cargo_calls().is_empty());
}

#[test]
fn conflict_lists_every_conflicting_file() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    project.write("src/parts/minhash/mod.rs", "a");
    project.write("src/parts/minhash/permutation.rs", "b");
    let r = add(&project, &reg, &["minhash"]);
    r.assert_fail()
        .stderr_has("src/parts/minhash/mod.rs")
        .stderr_has("src/parts/minhash/permutation.rs");
}

#[test]
fn overwrite_replaces_changed_files() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    project.write("src/parts/minhash/mod.rs", "// old\n");
    add(&project, &reg, &["minhash", "--overwrite"])
        .assert_ok()
        .stdout_has("overwrite src/parts/minhash/mod.rs");
    for (rel, expected) in minhash_files() {
        assert_eq!(std::fs::read(project.path().join(rel)).unwrap(), expected);
    }
}

#[test]
fn identical_files_are_not_conflicts() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    add(&project, &reg, &["minhash", "--no-deps"]).assert_ok();
    add(&project, &reg, &["minhash", "--no-deps"])
        .assert_ok()
        .stdout_has("unchanged src/parts/minhash/mod.rs")
        .stdout_has("unchanged src/parts/minhash/permutation.rs");
}

#[test]
fn refuses_to_write_through_symlink_even_with_overwrite() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("victim.rs");
    std::fs::write(&target, "precious\n").unwrap();
    std::fs::create_dir_all(project.path().join("src/parts/minhash")).unwrap();
    std::os::unix::fs::symlink(&target, project.path().join("src/parts/minhash/mod.rs")).unwrap();
    add(&project, &reg, &["minhash", "--overwrite"])
        .assert_fail()
        .stderr_has("symbolic link");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "precious\n");
}

#[test]
fn refuses_when_destination_is_a_directory() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    std::fs::create_dir_all(project.path().join("src/parts/minhash/mod.rs")).unwrap();
    add(&project, &reg, &["minhash", "--overwrite"])
        .assert_fail()
        .stderr_has("not a regular file");
}

// ---------------------------------------------------------------- --no-deps と Cargo.toml なし

#[test]
fn no_deps_prints_lines_instead_of_running_cargo() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    add(&project, &reg, &["minhash", "--no-deps"])
        .assert_ok()
        .stdout_has("(--no-deps)")
        .stdout_has("[dependencies]")
        .stdout_has("sha1 = \"0.11\"");
    assert!(project.cargo_calls().is_empty());
    assert_eq!(project.read("Cargo.toml"), common::EMPTY_MANIFEST);
}

#[test]
fn without_cargo_toml_prints_dependency_lines() {
    let reg = Registry::copy_of_repo();
    let project = Project::bare();
    add(&project, &reg, &["minhash"])
        .assert_ok()
        .stdout_has("no Cargo.toml with [package] found")
        .stdout_has("sha1 = \"0.11\"");
    assert!(project.exists("src/parts/minhash/mod.rs"));
    assert!(project.cargo_calls().is_empty());
}

#[test]
fn features_are_printed_as_toml() {
    let reg = Registry::empty();
    reg.write_part(
        "demo",
        &["parts/demo/mod.rs"],
        r#"{"name": "serde", "version": "1", "features": ["derive"]}"#,
    );
    reg.write("parts/demo/mod.rs", "pub fn run() {}\n");
    let project = Project::new();
    add(&project, &reg, &["demo", "--no-deps"])
        .assert_ok()
        .stdout_has(r#"serde = { version = "1", features = ["derive"] }"#);
}

#[test]
fn failing_cargo_add_is_reported() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    run(kura(&project)
        .args(["add", "minhash", "--registry", &reg.arg()])
        .env("KURA_TEST_CARGO_EXIT", "101"))
    .assert_fail()
    .stderr_has("`cargo add sha1@0.11` failed")
    .stdout_has("sha1 = \"0.11\"");
    // ファイルはすでにコピーされている
    assert!(project.exists("src/parts/minhash/mod.rs"));
}

#[test]
fn missing_cargo_binary_is_reported() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    run(kura(&project)
        .args(["add", "minhash", "--registry", &reg.arg()])
        .env("CARGO", "/definitely/not/cargo"))
    .assert_fail()
    .stderr_has("cannot run");
}

// ---------------------------------------------------------------- --dir

#[test]
fn dir_option_changes_destination() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    add(
        &project,
        &reg,
        &["minhash", "--no-deps", "--dir", "src/hashing/mh"],
    )
    .assert_ok()
    .stdout_has("create    src/hashing/mh/mod.rs")
    .stdout_has("Declare `mod mh;` in the parent module of src/hashing/mh");
    assert_eq!(
        project.files(),
        [
            "Cargo.toml",
            "src/hashing/mh/mod.rs",
            "src/hashing/mh/permutation.rs",
            "src/main.rs"
        ]
    );
}

#[test]
fn dir_option_accepts_absolute_path() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    let elsewhere = tempfile::tempdir().unwrap();
    let dest = elsewhere.path().join("mh");
    add(
        &project,
        &reg,
        &["minhash", "--no-deps", "--dir", dest.to_str().unwrap()],
    )
    .assert_ok();
    assert!(dest.join("mod.rs").is_file());
    assert!(!project.exists("src/parts"));
}

#[test]
fn dir_option_conflicts_are_detected_too() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    project.write("vendor/mh/mod.rs", "x");
    add(&project, &reg, &["minhash", "--dir", "vendor/mh"])
        .assert_fail()
        .stderr_has("vendor/mh/mod.rs");
}

// ---------------------------------------------------------------- 取得元の問題

#[test]
fn unknown_part_fails_with_hint() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    add(&project, &reg, &["does-not-exist"])
        .assert_fail()
        .stderr_has("part `does-not-exist` not found")
        .stderr_has("kura list");
    assert_eq!(project.files(), ["Cargo.toml", "src/main.rs"]);
}

#[test]
fn missing_registry_directory_fails() {
    let project = Project::new();
    run(kura(&project).args(["add", "minhash", "--registry", "/definitely/not/here"]))
        .assert_fail()
        .stderr_has("does not exist");
}

#[test]
fn unsupported_registry_scheme_fails() {
    let project = Project::new();
    for scheme in [
        "ftp://example.com/",
        "file:///etc/",
        "ssh://git@github.com/x",
    ] {
        run(kura(&project).args(["add", "minhash", "--registry", scheme]))
            .assert_fail()
            .stderr_has("unsupported registry");
    }
}

#[test]
fn invalid_json_fails() {
    let project = Project::new();
    for bad in [
        "".as_bytes(),
        b"{",
        b"not json",
        b"[]",
        b"null",
        b"{\"name\": \"broken\"}",
        b"\xff\xfe\xfd garbage",
        b"<!DOCTYPE html><html>404</html>",
    ] {
        let reg = Registry::empty();
        reg.write("registry/broken.json", bad);
        add(&project, &reg, &["broken"])
            .assert_fail()
            .stderr_has("registry/broken.json is invalid");
    }
    assert_eq!(project.files(), ["Cargo.toml", "src/main.rs"]);
}

#[test]
fn name_mismatch_is_rejected() {
    let reg = Registry::empty();
    reg.write(
        "registry/alias.json",
        common::part_json("minhash", &["parts/x.rs"], ""),
    );
    reg.write("parts/x.rs", "x");
    let project = Project::new();
    add(&project, &reg, &["alias"])
        .assert_fail()
        .stderr_has("declares name \"minhash\"");
}

#[test]
fn part_with_missing_files_fails_and_writes_nothing() {
    let reg = Registry::empty();
    reg.write_part("demo", &["parts/demo/mod.rs", "parts/demo/missing.rs"], "");
    reg.write("parts/demo/mod.rs", "mod missing;\n");
    let project = Project::new();
    add(&project, &reg, &["demo"])
        .assert_fail()
        .stderr_has("file parts/demo/missing.rs listed by part `demo` is missing");
    assert_eq!(project.files(), ["Cargo.toml", "src/main.rs"]);
    assert!(project.cargo_calls().is_empty());
}

#[test]
fn registry_parts_without_source_fail_cleanly() {
    // 見本の部品（ソースがまだない）を入れようとしても、何も書かずに失敗する
    let reg = Registry::empty();
    reg.write_part("sample-only", &["parts/sample_only/mod.rs"], "");
    let project = Project::new();
    add(&project, &reg, &["sample-only"])
        .assert_fail()
        .stderr_has("is missing");
    assert!(!project.exists("src/parts"));
}

#[test]
fn non_utf8_source_file_is_rejected() {
    let reg = Registry::empty();
    reg.write_part("demo", &["parts/demo/mod.rs"], "");
    reg.write("parts/demo/mod.rs", b"\xff\xfe\x00\x01binary");
    let project = Project::new();
    add(&project, &reg, &["demo"])
        .assert_fail()
        .stderr_has("not valid UTF-8");
    assert!(!project.exists("src/parts"));
}

#[test]
fn source_directory_instead_of_file_is_rejected() {
    let reg = Registry::empty();
    reg.write_part("demo", &["parts/demo"], "");
    reg.write("parts/demo/mod.rs", "x");
    let project = Project::new();
    add(&project, &reg, &["demo"])
        .assert_fail()
        .stderr_has("not a regular file");
}

#[test]
fn symlink_in_registry_pointing_outside_is_rejected() {
    let reg = Registry::empty();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret"), "secret\n").unwrap();
    reg.write_part("demo", &["parts/demo/mod.rs"], "");
    std::fs::create_dir_all(reg.path().join("parts/demo")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret"),
        reg.path().join("parts/demo/mod.rs"),
    )
    .unwrap();
    let project = Project::new();
    add(&project, &reg, &["demo"])
        .assert_fail()
        .stderr_has("outside the registry directory");
    assert!(!project.exists("src/parts"));
}

#[test]
fn oversized_source_file_is_rejected() {
    let reg = Registry::empty();
    reg.write_part("demo", &["parts/demo/mod.rs"], "");
    reg.write("parts/demo/mod.rs", vec![b'a'; 4 * 1024 * 1024 + 1]);
    let project = Project::new();
    add(&project, &reg, &["demo"])
        .assert_fail()
        .stderr_has("larger than");
}

// ---------------------------------------------------------------- 悪意のある rust.files

/// 取得元に悪意のあるパスを書き、何も書かれず・何も読まれずに失敗することを確かめる。
fn assert_malicious_files_rejected(files: &[&str]) {
    let reg = Registry::empty();
    reg.write_part("evil", files, "");
    // もし読みに行ってしまったら分かるよう、ありそうな場所に置いておく
    reg.write("parts/evil/mod.rs", "pub fn ok() {}\n");
    reg.write("x", "escaped\n");
    let project = Project::new();
    let r = add(&project, &reg, &["evil"]);
    r.assert_fail().stderr_has("registry/evil.json is invalid");
    assert_eq!(project.files(), ["Cargo.toml", "src/main.rs"], "{files:?}");
    assert!(project.cargo_calls().is_empty());
}

#[test]
fn rejects_parent_directory_escape() {
    assert_malicious_files_rejected(&["../x"]);
}

#[test]
fn rejects_absolute_path() {
    assert_malicious_files_rejected(&["/etc/passwd"]);
}

#[test]
fn rejects_escape_in_the_middle() {
    assert_malicious_files_rejected(&["a/../../b"]);
}

#[test]
fn rejects_backslashes() {
    assert_malicious_files_rejected(&["parts\\..\\..\\x"]);
    assert_malicious_files_rejected(&["parts\\evil\\mod.rs"]);
}

#[test]
fn rejects_empty_path() {
    assert_malicious_files_rejected(&[""]);
    assert_malicious_files_rejected(&[]);
}

#[test]
fn rejects_many_other_unsafe_paths() {
    for bad in [
        "..",
        ".",
        "./parts/evil/mod.rs",
        "parts/evil/../../../x",
        "parts//evil/mod.rs",
        "parts/evil/",
        "C:/Windows/win.ini",
        "C:\\Windows\\win.ini",
        "\\\\server\\share\\x",
        "//server/share/x",
        "parts/evil/mod.rs\0.txt",
        "parts/evil/\nmod.rs",
        "parts/.git/config",
        "~/.ssh/authorized_keys",
        "%2e%2e/x",
        "parts/evil/nul.rs",
        "parts/evil/mod.rs:stream",
    ] {
        assert_malicious_files_rejected(&[bad]);
    }
}

#[test]
fn one_bad_path_rejects_the_whole_part() {
    assert_malicious_files_rejected(&["parts/evil/mod.rs", "../x"]);
}

#[test]
fn case_insensitive_duplicate_destinations_are_rejected() {
    let reg = Registry::empty();
    reg.write_part("demo", &["parts/demo/mod.rs", "parts/demo/MOD.rs"], "");
    reg.write("parts/demo/mod.rs", "a");
    let project = Project::new();
    add(&project, &reg, &["demo"])
        .assert_fail()
        .stderr_has("same place");
    assert!(!project.exists("src/parts"));
}

#[test]
fn malicious_dependencies_are_rejected() {
    for deps in [
        r#"{"name": "--config", "version": "1"}"#,
        r#"{"name": "-Zunstable-options", "version": "1"}"#,
        r#"{"name": "x", "version": "1", "features": ["--git=https://evil"]}"#,
        r#"{"name": "x", "version": "1 --path /"}"#,
        r#"{"name": "x y", "version": "1"}"#,
        r#"{"name": "x", "version": ""}"#,
        r#"{"name": "x", "version": "1", "features": ["a,b"]}"#,
    ] {
        let reg = Registry::empty();
        reg.write_part("demo", &["parts/demo/mod.rs"], deps);
        reg.write("parts/demo/mod.rs", "x");
        let project = Project::new();
        add(&project, &reg, &["demo"])
            .assert_fail()
            .stderr_has("rust.dependencies[0]");
        assert!(project.cargo_calls().is_empty(), "{deps}");
        assert!(!project.exists("src/parts"));
    }
}

#[test]
fn control_characters_in_title_are_not_printed() {
    let reg = Registry::empty();
    let json = common::part_json("demo", &["parts/demo/mod.rs"], "")
        .replace("Test part", "Evil\\u001b[2Jtitle");
    reg.write("registry/demo.json", json);
    reg.write("parts/demo/mod.rs", "x");
    let project = Project::new();
    let r = add(&project, &reg, &["demo", "--dry-run"]);
    r.assert_ok().stdout_has("Evil[2Jtitle");
    assert!(!r.stdout.contains('\u{1b}'), "{r:?}");
}

// ---------------------------------------------------------------- 部品名

#[test]
fn non_kebab_names_are_rejected_before_any_fetch() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    for bad in [
        "MinHash",
        "min_hash",
        "min hash",
        "-minhash",
        "minhash-",
        "min--hash",
        "../minhash",
        "minhash/../../x",
        "minhash.json",
        "/etc/passwd",
        "ｍｉｎｈａｓｈ",
    ] {
        run(kura(&project).args(["add", "--registry", &reg.arg(), "--", bad]))
            .assert_fail()
            .stderr_has("must be kebab-case");
    }
    assert_eq!(project.files(), ["Cargo.toml", "src/main.rs"]);
}

#[test]
fn empty_name_is_rejected() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    run(kura(&project).args(["add", "", "--registry", &reg.arg()]))
        .assert_fail()
        .stderr_has("must be kebab-case");
}

#[test]
fn invalid_name_inside_json_is_rejected() {
    let reg = Registry::empty();
    reg.write(
        "registry/demo.json",
        common::part_json("Demo_Part", &["parts/x.rs"], ""),
    );
    let project = Project::new();
    add(&project, &reg, &["demo"])
        .assert_fail()
        .stderr_has("must be kebab-case");
}

// ---------------------------------------------------------------- 引数

#[test]
fn missing_name_is_a_usage_error() {
    let project = Project::new();
    let r = run(kura(&project).args(["add"]));
    assert_eq!(r.code, Some(2), "{r:?}");
}

#[test]
fn help_and_version() {
    let project = Project::new();
    run(kura(&project).args(["--version"]))
        .assert_ok()
        .stdout_has(env!("CARGO_PKG_VERSION"));
    run(kura(&project).args(["add", "--help"]))
        .assert_ok()
        .stdout_has("--overwrite")
        .stdout_has("--dry-run")
        .stdout_has("--no-deps")
        .stdout_has("--dir")
        .stdout_has("--registry");
}
