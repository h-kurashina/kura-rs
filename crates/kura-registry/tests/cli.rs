use std::fs;
use std::path::{Path, PathBuf};

use kura_registry::Paths;
use kura_schema::{Part, PartIndex};

/// 一時ディレクトリにリポジトリの縮小版を作る。
fn temp_repo(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("kura-registry-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("registry")).unwrap();
    let repo = Paths::from_manifest();
    fs::copy(
        repo.registry().join("minhash.json"),
        root.join("registry/minhash.json"),
    )
    .unwrap();
    root
}

#[test]
fn repository_registry_is_valid() {
    let parts =
        kura_registry::check(&Paths::from_manifest()).expect("registry/*.json must be valid");
    assert!(parts.iter().any(|p| p.name == "minhash"));
}

#[test]
fn generated_files_are_up_to_date() {
    kura_registry::codegen_check(&Paths::from_manifest()).unwrap();
}

#[test]
fn build_writes_parts_index_and_schema() {
    let root = temp_repo("build");
    let paths = Paths::new(&root);
    fs::create_dir_all(paths.public().join("r")).unwrap();
    fs::write(paths.public().join("r/stale.json"), "{}").unwrap();

    kura_registry::build(&paths).unwrap();

    let out = paths.public().join("r");
    assert!(
        !out.join("stale.json").exists(),
        "stale output must be removed"
    );
    let part: Part = read(&out.join("minhash.json"));
    let source: Part = read(&paths.registry().join("minhash.json"));
    assert_eq!(part, source, "served JSON must match the registry");

    let index: PartIndex = read(&out.join("index.json"));
    assert_eq!(index.parts.len(), 1);
    assert_eq!(index.parts[0].path, "minhash.json");
    assert!(paths.public().join("schema/part.schema.json").exists());
    fs::remove_dir_all(root).ok();
}

#[test]
fn build_fails_on_invalid_registry_and_reports_every_problem() {
    let root = temp_repo("invalid");
    let file = root.join("registry/minhash.json");
    let mut json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let cases = json["verification"]["cases"].as_u64().unwrap();
    json["verification"]["passed"] = (cases + 1).into();
    json["version"] = "one".into();
    fs::write(&file, serde_json::to_string_pretty(&json).unwrap()).unwrap();

    let err = kura_registry::build(&Paths::new(&root))
        .unwrap_err()
        .to_string();
    assert!(err.contains("verification.passed"), "{err}");
    assert!(err.contains("version"), "{err}");
    assert!(
        !root.join("site/public/r").exists(),
        "nothing is written when validation fails"
    );
    fs::remove_dir_all(root).ok();
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}
