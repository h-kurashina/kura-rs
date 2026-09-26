//! 部品（Part）のスキーマ。JSON Schema と TypeScript の型はここから生成する。
//!
//! フィールドを増やす・変えるときはこのファイルを編集し、
//! `cargo run -p kura-registry -- codegen` で生成物を更新する。
//! JSON 上のフィールド名は snake_case に統一する（Rust の名前がそのまま出る）。

use chrono::{DateTime, NaiveDate, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A kura-rs part. The content of `registry/<name>.json` and `/r/<name>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(optional_fields)]
pub struct Part {
    /// Reference to the JSON Schema, for editor completion.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Unique identifier in kebab-case. Also used as the file name and URL slug.
    pub name: String,
    pub title: String,
    pub description: String,
    /// Semantic version, e.g. `0.1.0`.
    pub version: String,
    /// Date the part was added to the registry (YYYY-MM-DD). The site features the newest part.
    pub added: NaiveDate,
    /// When true, verification and benchmark numbers are placeholders, not measurements.
    pub sample: bool,
    /// Shelves the part belongs to. At least one, no duplicates.
    pub shelves: Vec<Shelf>,
    pub rust: RustSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonPackage>,
    pub reference: Reference,
    pub verification: Verification,
    /// Sorted by strictly increasing input size.
    pub benchmarks: Vec<BenchmarkPoint>,
    /// Where the benchmarks were measured. Written by `verify/run.py`; required when `sample` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
    /// What `input_size` counts, e.g. "tokens per document".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_unit: Option<String>,
    pub usage: Usage,
    /// Translations of the human-facing text. English (the fields above) is the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translations: Option<Translations>,
}

/// Translated text per locale. Add a field here to support a new locale.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(optional_fields)]
pub struct Translations {
    /// Japanese.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ja: Option<PartTranslation>,
    /// Simplified Chinese.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zh: Option<PartTranslation>,
    /// Korean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko: Option<PartTranslation>,
    /// Spanish.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub es: Option<PartTranslation>,
    /// French.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fr: Option<PartTranslation>,
    /// German.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub de: Option<PartTranslation>,
    /// Brazilian Portuguese.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pt: Option<PartTranslation>,
}

impl Translations {
    /// 訳の欄がある言語コード。欠けている訳の報告に使う。
    pub const LOCALES: [&'static str; 7] = ["ja", "zh", "ko", "es", "fr", "de", "pt"];

    /// ロケール名と訳文の組。検証で使う。
    pub fn entries(&self) -> impl Iterator<Item = (&'static str, &PartTranslation)> {
        [
            ("ja", self.ja.as_ref()),
            ("zh", self.zh.as_ref()),
            ("ko", self.ko.as_ref()),
            ("es", self.es.as_ref()),
            ("fr", self.fr.as_ref()),
            ("de", self.de.as_ref()),
            ("pt", self.pt.as_ref()),
        ]
        .into_iter()
        .filter_map(|(k, v)| v.map(|v| (k, v)))
    }
}

/// Translated title and description. Omitted fields fall back to English.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(optional_fields)]
pub struct PartTranslation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum Shelf {
    Ai,
    Security,
}

impl Shelf {
    pub const ALL: [Shelf; 2] = [Shelf::Ai, Shelf::Security];

    pub fn as_str(self) -> &'static str {
        match self {
            Shelf::Ai => "ai",
            Shelf::Security => "security",
        }
    }
}

impl std::str::FromStr for Shelf {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Shelf::ALL
            .into_iter()
            .find(|shelf| shelf.as_str() == s)
            .ok_or_else(|| format!("unknown shelf \"{s}\" (expected one of: ai, security)"))
    }
}

/// Source that Rust users copy into their own project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RustSource {
    /// Paths relative to the kura repository root.
    pub files: Vec<String>,
    pub dependencies: Vec<CrateDependency>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CrateDependency {
    pub name: String,
    /// Version requirement, e.g. `"0.8"`.
    pub version: String,
    /// Cargo features to enable. Empty when omitted.
    #[serde(default)]
    pub features: Vec<String>,
}

/// PyO3 package for Python users.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct PythonPackage {
    /// PyPI package name, e.g. `kura-pack`.
    pub package: String,
    /// Import statement, e.g. `from kura_pack import minhash`.
    pub import_path: String,
}

/// The implementation kura is compared against.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(optional_fields)]
pub struct Reference {
    pub name: String,
    pub language: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub method: VerificationMethod,
    pub cases: u32,
    pub passed: u32,
    /// When the verification last ran (RFC 3339).
    pub last_run: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum VerificationMethod {
    /// Same inputs fed to kura and the reference; outputs compared.
    Differential,
    /// Property-based tests.
    Property,
    /// Compared against fixed expected outputs.
    Golden,
}

/// The machine and toolchains a benchmark ran on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    /// CPU, e.g. "Apple M3 Pro".
    pub cpu: String,
    /// Operating system, e.g. "macOS 27.0 (arm64)".
    pub os: String,
    /// Rust compiler version, e.g. "rustc 1.98.1".
    pub rust: String,
    /// Runtime of the reference, e.g. "Python 3.14.6, numpy 2.5.3".
    pub reference_runtime: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkPoint {
    /// Input size (x axis, plotted on a log scale).
    pub input_size: u64,
    /// Median wall time of the kura Rust implementation, in milliseconds.
    pub rust_ms: f64,
    /// Median wall time of the reference implementation, in milliseconds.
    pub reference_ms: f64,
}

impl BenchmarkPoint {
    /// Speedup over the reference. Greater than 1 means kura is faster.
    pub fn speedup(&self) -> f64 {
        self.reference_ms / self.rust_ms
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
#[ts(optional_fields)]
pub struct Usage {
    pub rust: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<String>,
}
