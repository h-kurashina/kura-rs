//! 生成物（JSON Schema と TypeScript の型）を作る。

use std::path::Path;

use ts_rs::{Config, TS};

use crate::index::PartIndex;
use crate::part::Part;

/// JSON Schema を出力するファイル名と、その中身。
pub fn json_schemas() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "part.schema.json",
            serde_json::to_value(schemars::schema_for!(Part)).expect("schema serializes"),
        ),
        (
            "index.schema.json",
            serde_json::to_value(schemars::schema_for!(PartIndex)).expect("schema serializes"),
        ),
    ]
}

/// TypeScript の型を `out_dir` に書き出す（依存する型も含む）。
pub fn export_typescript(out_dir: &Path) -> Result<(), ts_rs::ExportError> {
    // u64 などは JS の number で表す（部品の数値は 2^53 を超えない）
    let cfg = Config::new().with_out_dir(out_dir).with_large_int("number");
    Part::export_all(&cfg)?;
    PartIndex::export_all(&cfg)?;
    Ok(())
}
