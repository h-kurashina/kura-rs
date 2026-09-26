//! `kura list` をローカルの取得元で動かすテスト。

mod common;

use common::{Project, Registry, kura, run};

#[test]
fn lists_repository_parts() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    let r = run(kura(&project).args(["list", "--registry", &reg.arg()]));
    r.assert_ok()
        .stdout_has("NAME")
        .stdout_has("TITLE")
        .stdout_has("SHELVES")
        .stdout_has("minhash")
        .stdout_has("MinHash")
        .stdout_has("kura add <name>");
    let minhash = r
        .stdout
        .lines()
        .find(|l| l.starts_with("minhash "))
        .unwrap();
    assert!(minhash.contains("ai"), "{minhash}");
    assert!(
        !minhash.contains("(sample)"),
        "minhash is measured: {minhash}"
    );
    // 見本の部品には印が付く
    assert!(r.stdout.lines().any(|l| l.ends_with("(sample)")), "{r:?}");
}

#[test]
fn list_is_sorted_by_name() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    let r = run(kura(&project).args(["list", "--registry", &reg.arg()]));
    let names: Vec<&str> = r
        .stdout
        .lines()
        .skip(1)
        .take_while(|l| !l.is_empty())
        .map(|l| l.split_whitespace().next().unwrap())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    assert!(names.len() >= 2);
}

#[test]
fn list_skips_broken_entries_with_warning() {
    let reg = Registry::empty();
    reg.write_part("good", &["parts/good/mod.rs"], "");
    reg.write("registry/broken.json", "{ nope");
    reg.write("registry/Bad_Name.json", "{}");
    reg.write("registry/index.json", r#"{"parts": []}"#);
    reg.write("registry/README.md", "not a part");
    reg.write(
        "registry/evil.json",
        common::part_json("evil", &["../x"], ""),
    );
    let project = Project::new();
    let r = run(kura(&project).args(["list", "--registry", &reg.arg()]));
    r.assert_ok()
        .stdout_has("good")
        .stderr_has("skipping registry/broken.json")
        .stderr_has("skipping registry/Bad_Name.json")
        .stderr_has("skipping registry/evil.json");
    assert!(!r.stdout.contains("broken"));
    assert!(!r.stdout.contains("evil"));
    assert!(
        !r.stderr.contains("index"),
        "index.json is not a part: {r:?}"
    );
    assert!(r.stdout.contains("1 part(s)"), "{r:?}");
}

#[test]
fn list_of_empty_registry_fails() {
    let reg = Registry::empty();
    let project = Project::new();
    run(kura(&project).args(["list", "--registry", &reg.arg()]))
        .assert_fail()
        .stderr_has("no parts found");
}

#[test]
fn list_without_registry_directory_fails() {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::new();
    run(kura(&project).args(["list", "--registry", dir.path().to_str().unwrap()]))
        .assert_fail()
        .stderr_has("cannot read");
}
