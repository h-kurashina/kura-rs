//! kura-registry CLI の処理本体。main.rs とテストから使う。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use kura_schema::{Part, PartIndex, codegen, load_dir, to_published_json};

/// リポジトリ内の各パス。
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// このクレートの場所からリポジトリのルートを求める。
    pub fn from_manifest() -> Self {
        Self::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
    }

    pub fn registry(&self) -> PathBuf {
        self.root.join("registry")
    }
    pub fn schema(&self) -> PathBuf {
        self.root.join("schema")
    }
    pub fn generated_ts(&self) -> PathBuf {
        self.root.join("site/src/generated")
    }
    pub fn public(&self) -> PathBuf {
        self.root.join("site/public")
    }
}

/// registry/*.json を読み込んで検証する。
pub fn check(paths: &Paths) -> Result<Vec<Part>> {
    Ok(load_dir(&paths.registry())?)
}

/// 検証に通った部品を site/public/r/ と site/public/schema/ に書き出す。
pub fn build(paths: &Paths) -> Result<Vec<Part>> {
    let parts = check(paths)?;

    let out = paths.public().join("r");
    if out.exists() {
        std::fs::remove_dir_all(&out).with_context(|| format!("cannot clean {}", out.display()))?;
    }
    std::fs::create_dir_all(&out)?;

    for part in &parts {
        write_json(&out.join(format!("{}.json", part.name)), &part.published())?;
    }
    write_json(&out.join("index.json"), &PartIndex::new(&parts))?;

    let schema_out = paths.public().join("schema");
    std::fs::create_dir_all(&schema_out)?;
    for (file, schema) in codegen::json_schemas() {
        write_json(&schema_out.join(file), &schema)?;
    }
    Ok(parts)
}

/// JSON Schema を schema/ に、TypeScript の型を site/src/generated/ に書き出す。
pub fn codegen(paths: &Paths) -> Result<()> {
    write_codegen(&paths.schema(), &paths.generated_ts())
}

/// 生成物がコミット済みのものと一致しているか確かめる（CI やテスト用）。
pub fn codegen_check(paths: &Paths) -> Result<()> {
    let tmp = std::env::temp_dir().join(format!("kura-codegen-{}", std::process::id()));
    let (schema_tmp, ts_tmp) = (tmp.join("schema"), tmp.join("ts"));
    write_codegen(&schema_tmp, &ts_tmp)?;
    let mut stale = Vec::new();
    for (tmp_dir, real_dir) in [
        (&schema_tmp, paths.schema()),
        (&ts_tmp, paths.generated_ts()),
    ] {
        for entry in std::fs::read_dir(tmp_dir)? {
            let name = entry?.file_name();
            let fresh = std::fs::read(tmp_dir.join(&name))?;
            let current = std::fs::read(real_dir.join(&name)).unwrap_or_default();
            if fresh != current {
                stale.push(real_dir.join(&name).display().to_string());
            }
        }
    }
    std::fs::remove_dir_all(&tmp).ok();
    if !stale.is_empty() {
        bail!(
            "generated files are out of date (run `cargo run -p kura-registry -- codegen`):\n  {}",
            stale.join("\n  ")
        );
    }
    Ok(())
}

fn write_codegen(schema_dir: &Path, ts_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(schema_dir)?;
    for (file, schema) in codegen::json_schemas() {
        write_json(&schema_dir.join(file), &schema)?;
    }
    if ts_dir.exists() {
        std::fs::remove_dir_all(ts_dir)?;
    }
    codegen::export_typescript(ts_dir).context("cannot export TypeScript types")?;
    Ok(())
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let json = to_published_json(value)?;
    std::fs::write(path, json).with_context(|| format!("cannot write {}", path.display()))
}
