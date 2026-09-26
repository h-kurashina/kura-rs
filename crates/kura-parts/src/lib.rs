//! parts/ にある部品（ユーザーがコピーするソース）を、1つの crate としてビルド・テストするためのもの。
//! 部品のソースはここに置かず、parts/<name>/ を #[path] で読み込む。

#[path = "../../../parts/minhash/mod.rs"]
pub mod minhash;

#[path = "../../../parts/file_hash/mod.rs"]
pub mod file_hash;
