//! CLI が写し持っている型（`kura_rs::manifest`）と、正本のスキーマ（kura-schema）がずれていないかを確かめる。
//! kura-schema は crates.io に出さないので、CLI はここでしか kura-schema に依存しない（dev-dependency）。

mod common;

use kura_rs::manifest::Manifest;

fn registry_parts() -> Vec<(String, String)> {
    let dir = common::repo_root().join("registry");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            let file = path.file_name().unwrap().to_string_lossy().into_owned();
            out.push((file, std::fs::read_to_string(&path).unwrap()));
        }
    }
    out.sort();
    assert!(!out.is_empty());
    out
}

/// kura-schema が妥当とする部品は、CLI も受け入れ、同じ値として読む。
#[test]
fn every_valid_registry_part_is_accepted_by_the_cli() {
    for (file, json) in registry_parts() {
        let Ok(part) = kura_schema::parse_part(&file, &json) else {
            continue; // kura-registry check が落とすので、ここでは見ない
        };
        let checked = Manifest::from_json(json.as_bytes())
            .unwrap_or_else(|e| panic!("{file}: {e}"))
            .check(Some(&part.name))
            .unwrap_or_else(|e| panic!("{file}: {e:?}"));
        let m = &checked.manifest;
        assert_eq!(m.name, part.name);
        assert_eq!(m.title, part.title);
        assert_eq!(m.description, part.description);
        assert_eq!(m.version, part.version);
        assert_eq!(m.sample, part.sample);
        assert_eq!(
            m.shelves,
            part.shelves.iter().map(|s| s.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(m.rust.files, part.rust.files);
        assert_eq!(m.rust.dependencies.len(), part.rust.dependencies.len());
        for (a, b) in m.rust.dependencies.iter().zip(&part.rust.dependencies) {
            assert_eq!(
                (&a.name, &a.version, &a.features),
                (&b.name, &b.version, &b.features)
            );
        }
        assert_eq!(
            m.usage.as_ref().and_then(|u| u.rust.as_deref()),
            Some(part.usage.rust.as_str())
        );
        assert_eq!(
            checked
                .files
                .iter()
                .map(|p| p.as_slash_str())
                .collect::<Vec<_>>(),
            part.rust.files
        );
    }
}

/// 配信用の JSON（`/r/<name>.json`、`$schema` 付き）も読める。
#[test]
fn published_json_is_accepted() {
    for (file, json) in registry_parts() {
        let Ok(part) = kura_schema::parse_part(&file, &json) else {
            continue;
        };
        let published = kura_schema::to_published_json(&part.published()).unwrap();
        let m = Manifest::from_json(published.as_bytes()).unwrap();
        assert_eq!(m.name, part.name);
    }
}

/// 部品名の規則は kura-schema と同じ（kura-schema が kebab-case でないと言うものは CLI も拒む）。
#[test]
fn name_rule_matches_schema() {
    let (file, json) = registry_parts()
        .into_iter()
        .find(|(f, _)| f == "minhash.json")
        .unwrap();
    for name in [
        "minhash",
        "file-hash",
        "MinHash",
        "min_hash",
        "a--b",
        "-a",
        "a-",
        "",
        "a1-2b",
    ] {
        let edited = json.replacen("\"name\": \"minhash\"", &format!("\"name\": {name:?}"), 1);
        let schema_ok = kura_schema::parse_part(&file.replace("minhash", name), &edited).is_ok();
        let cli_ok = kura_rs::paths::is_kebab_case(name);
        assert_eq!(schema_ok, cli_ok, "{name:?}");
    }
}

/// kura-schema が受け入れるパスのうち、CLI が拒むものがリポジトリにない。
/// （CLI の方が厳しいので、ここが落ちたら registry のパスを直すか、CLI の規則を見直す）
#[test]
fn repository_paths_pass_the_stricter_cli_check() {
    for (file, json) in registry_parts() {
        if let Ok(part) = kura_schema::parse_part(&file, &json) {
            for f in &part.rust.files {
                assert!(
                    kura_rs::paths::SafePath::parse(f).is_ok(),
                    "{file}: {f} is rejected by the CLI"
                );
            }
        }
    }
}

/// 既定の取得元はこのリポジトリを指す。
#[test]
fn default_registry_points_at_this_repository() {
    let repo = env!("CARGO_PKG_REPOSITORY"); // https://github.com/h-kurashina/kura-rs
    let owner_repo = repo.strip_prefix("https://github.com/").unwrap();
    assert_eq!(
        kura_rs::source::DEFAULT_REGISTRY,
        format!("https://raw.githubusercontent.com/{owner_repo}/main/")
    );
}
