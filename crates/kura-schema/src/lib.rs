//! kura レジストリのスキーマ。
//!
//! - [`part`] — 部品のデータ構造（唯一の正本）
//! - [`index`] — `/r/index.json` の形
//! - [`validate`] — 型で表せない規則の検証
//! - [`publish`] — 配信する JSON の形（静的サイトとサーバーで共通）
//! - [`registry`] — `registry/*.json` の読み込み
//! - [`codegen`] — JSON Schema と TypeScript の型の生成

pub mod codegen;
pub mod index;
pub mod part;
pub mod publish;
pub mod registry;
pub mod validate;

pub use index::{PartIndex, PartSummary};
pub use part::*;
pub use publish::{PART_SCHEMA_REF, to_published_json};
pub use registry::{LoadError, RegistryError, load_dir, parse_part};
pub use validate::Issue;
