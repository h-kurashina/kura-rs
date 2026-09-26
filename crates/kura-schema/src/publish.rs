//! 配信する JSON の形を 1 か所で決める。静的サイト（kura-registry build）と
//! API サーバー（kura-server）はどちらもここを通すので、返す JSON はバイト単位で一致する。

use serde::Serialize;

use crate::part::Part;

/// 配信先（`/r/<name>.json`）から見た JSON Schema の場所。
pub const PART_SCHEMA_REF: &str = "../schema/part.schema.json";

impl Part {
    /// 配信用に `$schema` を配信先からの相対パスにそろえたもの。
    pub fn published(&self) -> Part {
        Part {
            schema: Some(PART_SCHEMA_REF.to_owned()),
            ..self.clone()
        }
    }
}

/// 配信用の JSON 文字列（整形済み、末尾改行あり）。
pub fn to_published_json(value: &impl Serialize) -> serde_json::Result<String> {
    let mut json = serde_json::to_string_pretty(value)?;
    json.push('\n');
    Ok(json)
}
