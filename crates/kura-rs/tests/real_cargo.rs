//! 本物の cargo を使う、端から端までのテスト。
//! `kura add minhash` → 本物の `cargo add` → 案内どおりにモジュールを宣言 → `cargo run` で動く、まで確かめる。
//!
//! ネットにはつながない（`CARGO_NET_OFFLINE`）。このワークスペース自体が sha1 0.11 に依存しているので、
//! テストを走らせる時点で必要なクレートは手元のキャッシュにある。

mod common;

use std::process::Command;

use common::{Project, Registry, run};

#[test]
fn added_minhash_builds_and_runs_with_real_cargo() {
    let reg = Registry::copy_of_repo();
    let project = Project::new();
    let target = tempfile::tempdir().unwrap();
    let cargo = env!("CARGO");

    let r = run(Command::new(env!("CARGO_BIN_EXE_kura"))
        .current_dir(project.path())
        .env_remove("KURA_REGISTRY")
        .env("CARGO", cargo)
        .env("CARGO_NET_OFFLINE", "true")
        .args(["add", "minhash", "--registry", &reg.arg()]));
    r.assert_ok();
    assert!(
        project.read("Cargo.toml").contains("sha1 = \"0.11\""),
        "{}",
        project.read("Cargo.toml")
    );

    // 案内に出るとおりにモジュールを宣言し、使い方の例を動かす
    project.write("src/parts/mod.rs", "pub mod minhash;\n");
    project.write(
        "src/main.rs",
        r#"mod parts;

use crate::parts::minhash::MinHasher;

fn main() {
    let hasher = MinHasher::new(128, 1);
    let a = hasher.signature("the quick brown fox jumps".split_whitespace());
    let b = hasher.signature("the quick brown fox jumps".split_whitespace());
    println!("jaccard={}", a.jaccard(&b));
}
"#,
    );
    let out = Command::new(cargo)
        .current_dir(project.path())
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", target.path())
        .args(["run", "--quiet"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cargo run failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "jaccard=1");
}
