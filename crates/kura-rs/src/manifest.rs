//! `registry/<name>.json` のうち、CLI が使う部分だけの型と検証。
//!
//! 正本のスキーマは `kura-schema`（`crates/kura-schema/src/part.rs`）。kura-schema は
//! crates.io に出していない（JSON Schema や TypeScript の生成のための重い依存を持つ）ので、
//! CLI は必要なフィールドだけをここに写している。
//!
//! - 知らないフィールドは無視する。レジストリにフィールドが増えても、古い CLI が壊れない。
//! - 使うフィールドは kura-schema より厳しく確認する（信用できない入力なので）。
//! - kura-schema とずれていないかは `tests/schema_drift.rs` で確かめる。

use serde::Deserialize;

use crate::paths::{SafePath, check_part_name};

/// 部品の JSON のうち、CLI が使う部分。
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub sample: bool,
    #[serde(default)]
    pub shelves: Vec<String>,
    pub rust: RustSource,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RustSource {
    pub files: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub rust: Option<String>,
}

/// 検証済みの部品。ファイルのパスは安全と確認済み。
#[derive(Debug, Clone, PartialEq)]
pub struct CheckedPart {
    pub manifest: Manifest,
    pub files: Vec<SafePath>,
}

/// 1 部品が持てるファイル数と依存の数の上限（ばかげた JSON で暴れないように）。
pub const MAX_FILES: usize = 256;
pub const MAX_DEPENDENCIES: usize = 64;

impl Manifest {
    /// JSON を読む。UTF-8 でない・JSON でない・形が違うものはエラー。
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(bytes).map_err(|_| "not valid UTF-8".to_owned())?;
        serde_json::from_str(text).map_err(|e| format!("not a valid part JSON: {e}"))
    }

    /// 使うフィールドを確認する。`expected_name` は要求した部品名（JSON の name と一致すること）。
    pub fn check(self, expected_name: Option<&str>) -> Result<CheckedPart, Vec<String>> {
        let mut errors = Vec::new();
        if let Err(e) = check_part_name(&self.name) {
            errors.push(format!("name: {e}"));
        }
        if let Some(expected) = expected_name
            && self.name != expected
        {
            errors.push(format!(
                "name: the file for {expected:?} declares name {:?}",
                self.name
            ));
        }
        if self.title.trim().is_empty() {
            errors.push("title: must not be empty".into());
        }
        if self.version.trim().is_empty() {
            errors.push("version: must not be empty".into());
        }

        if self.rust.files.is_empty() {
            errors.push("rust.files: must list at least one file".into());
        }
        if self.rust.files.len() > MAX_FILES {
            errors.push(format!("rust.files: more than {MAX_FILES} files"));
        }
        let mut files = Vec::new();
        for (i, f) in self.rust.files.iter().enumerate() {
            match SafePath::parse(f) {
                Ok(p) => files.push(p),
                Err(e) => errors.push(format!("rust.files[{i}]: {e}")),
            }
        }

        if self.rust.dependencies.len() > MAX_DEPENDENCIES {
            errors.push(format!(
                "rust.dependencies: more than {MAX_DEPENDENCIES} entries"
            ));
        }
        for (i, dep) in self.rust.dependencies.iter().enumerate() {
            if let Err(e) = dep.check() {
                errors.push(format!("rust.dependencies[{i}]: {e}"));
            }
        }

        if errors.is_empty() {
            Ok(CheckedPart {
                manifest: self,
                files,
            })
        } else {
            Err(errors)
        }
    }
}

/// crates.io のクレート名として正しいか（英字で始まり、英数字・`_`・`-`、64 文字まで）。
pub fn is_crate_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic())
        && s.len() <= 64
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// バージョン指定（`0.11`、`^1.2`、`>=1, <2` など）として許す文字だけか。
pub fn is_version_req(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty()
        && s.len() <= 64
        && t == s
        && s.bytes().any(|b| b.is_ascii_digit() || b == b'*')
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'.' | b'*' | b'^' | b'~' | b'=' | b'<' | b'>' | b',' | b' ' | b'-' | b'+'
                )
        })
}

/// Cargo の feature 名として許す文字だけか（`derive`、`serde/std` など）。`-` では始めない。
pub fn is_feature_name(s: &str) -> bool {
    let mut bytes = s.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphanumeric() || b == b'_')
        && s.len() <= 64
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'+' | b'.' | b'/'))
}

impl Dependency {
    /// `cargo add` に渡すので、オプションとして解釈される形（`-` で始まる等）も含めて弾く。
    pub fn check(&self) -> Result<(), String> {
        if !is_crate_name(&self.name) {
            return Err(format!("invalid crate name {:?}", self.name));
        }
        if !is_version_req(&self.version) {
            return Err(format!(
                "invalid version requirement {:?} for {}",
                self.version, self.name
            ));
        }
        if let Some(f) = self.features.iter().find(|f| !is_feature_name(f)) {
            return Err(format!("invalid feature {f:?} for {}", self.name));
        }
        Ok(())
    }

    /// `cargo add` の引数（`--manifest-path` を除く）。
    pub fn cargo_add_args(&self) -> Vec<String> {
        let mut args = vec![format!("{}@{}", self.name, self.version)];
        if !self.features.is_empty() {
            args.push("--features".into());
            args.push(self.features.join(","));
        }
        args
    }

    /// Cargo.toml の `[dependencies]` に書く 1 行。
    pub fn toml_line(&self) -> String {
        if self.features.is_empty() {
            format!("{} = \"{}\"", self.name, self.version)
        } else {
            let features: Vec<String> = self.features.iter().map(|f| format!("\"{f}\"")).collect();
            format!(
                "{} = {{ version = \"{}\", features = [{}] }}",
                self.name,
                self.version,
                features.join(", ")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dep(name: &str, version: &str, features: &[&str]) -> Dependency {
        Dependency {
            name: name.into(),
            version: version.into(),
            features: features.iter().map(|s| s.to_string()).collect(),
        }
    }

    const MINIMAL: &str = r#"{
        "name": "demo", "title": "Demo", "version": "0.1.0",
        "rust": { "files": ["parts/demo/mod.rs"], "dependencies": [] }
    }"#;

    #[test]
    fn parses_minimal_and_ignores_unknown_fields() {
        let m = Manifest::from_json(MINIMAL.as_bytes()).unwrap();
        assert_eq!(m.name, "demo");
        let with_extra = MINIMAL.replacen('{', r#"{ "future_field": [1, 2, 3], "#, 1);
        assert_eq!(Manifest::from_json(with_extra.as_bytes()).unwrap(), m);
        assert!(m.check(Some("demo")).is_ok());
    }

    #[test]
    fn rejects_garbage() {
        assert!(Manifest::from_json(b"").is_err());
        assert!(Manifest::from_json(b"\xff\xfe\x00").is_err());
        assert!(Manifest::from_json(b"<html>404</html>").is_err());
        assert!(Manifest::from_json(b"[]").is_err());
        assert!(Manifest::from_json(br#"{"name": "x"}"#).is_err());
        assert!(
            Manifest::from_json(
                br#"{"name": 1, "title": "t", "version": "1", "rust": {"files": []}}"#
            )
            .is_err()
        );
    }

    #[test]
    fn name_must_match_request() {
        let m = Manifest::from_json(MINIMAL.as_bytes()).unwrap();
        let errs = m.check(Some("other")).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("declares name")), "{errs:?}");
    }

    #[test]
    fn collects_all_problems() {
        let json = r#"{
            "name": "Bad_Name", "title": " ", "version": "",
            "rust": { "files": ["../x", "/etc/passwd", "ok.rs"],
                      "dependencies": [{"name": "--config", "version": "1"}] }
        }"#;
        let errs = Manifest::from_json(json.as_bytes())
            .unwrap()
            .check(None)
            .unwrap_err();
        assert_eq!(errs.len(), 6, "{errs:#?}");
    }

    #[test]
    fn empty_files_rejected() {
        let json = r#"{"name": "a", "title": "A", "version": "1",
                       "rust": {"files": [], "dependencies": []}}"#;
        assert!(
            Manifest::from_json(json.as_bytes())
                .unwrap()
                .check(None)
                .is_err()
        );
    }

    #[test]
    fn dependency_checks() {
        assert!(dep("sha1", "0.11", &[]).check().is_ok());
        assert!(
            dep("aho-corasick", "1", &["std", "perf-literal"])
                .check()
                .is_ok()
        );
        assert!(
            dep(
                "serde",
                "^1.0.200",
                &["derive", "serde_derive/deserialize_in_place"]
            )
            .check()
            .is_ok()
        );
        assert!(dep("x", ">=1, <2", &[]).check().is_ok());
        assert!(dep("x", "*", &[]).check().is_ok());
        for bad in [
            dep("--config", "1", &[]),
            dep("-x", "1", &[]),
            dep("", "1", &[]),
            dep("1abc", "1", &[]),
            dep("a b", "1", &[]),
            dep("a@1", "1", &[]),
            dep("x", "", &[]),
            dep("x", " 1", &[]),
            dep("x", "latest", &[]),
            dep("x", "1;rm -rf /", &[]),
            dep("x", "1\n", &[]),
            dep("x", "1", &["-Zfoo"]),
            dep("x", "1", &["--offline"]),
            dep("x", "1", &["a,b"]),
            dep("x", "1", &[""]),
            dep("x", "1", &["a b"]),
        ] {
            assert!(bad.check().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn cargo_args_and_toml() {
        let d = dep("sha1", "0.11", &[]);
        assert_eq!(d.cargo_add_args(), vec!["sha1@0.11"]);
        assert_eq!(d.toml_line(), r#"sha1 = "0.11""#);
        let d = dep("serde", "1", &["derive", "rc"]);
        assert_eq!(
            d.cargo_add_args(),
            vec!["serde@1", "--features", "derive,rc"]
        );
        assert_eq!(
            d.toml_line(),
            r#"serde = { version = "1", features = ["derive", "rc"] }"#
        );
    }
}
