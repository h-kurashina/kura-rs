//! `/r/index.json` の形。部品一覧のカードや CLI の検索に使う要約。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::part::{Part, Shelf, Translations};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PartIndex {
    /// Sorted by name.
    pub parts: Vec<PartSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(optional_fields)]
pub struct PartSummary {
    pub name: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub added: chrono::NaiveDate,
    pub sample: bool,
    pub shelves: Vec<Shelf>,
    /// Name of the reference implementation, e.g. `datasketch`.
    pub reference: String,
    /// Largest speedup over the reference across all benchmark points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_speedup: Option<f64>,
    /// Path of the part JSON relative to this index, e.g. `minhash.json`.
    pub path: String,
    /// Translated title and description, same as the part's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translations: Option<Translations>,
}

impl From<&Part> for PartSummary {
    fn from(part: &Part) -> Self {
        Self {
            name: part.name.clone(),
            title: part.title.clone(),
            description: part.description.clone(),
            version: part.version.clone(),
            added: part.added,
            sample: part.sample,
            shelves: part.shelves.clone(),
            reference: part.reference.name.clone(),
            max_speedup: part.max_speedup(),
            path: format!("{}.json", part.name),
            translations: part.translations.clone(),
        }
    }
}

impl PartIndex {
    pub fn new<'a>(parts: impl IntoIterator<Item = &'a Part>) -> Self {
        let mut parts: Vec<PartSummary> = parts.into_iter().map(PartSummary::from).collect();
        parts.sort_by(|a, b| a.name.cmp(&b.name));
        Self { parts }
    }
}
