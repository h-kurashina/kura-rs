//! 型だけでは表せない規則の検証。サイト・CLI・サーバーはすべてここを通す。

use std::collections::HashSet;

use crate::part::Part;

/// 検証で見つかった問題 1 件。`field` は JSON 上のパス（例: `verification.passed`）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field}: {message}")]
pub struct Issue {
    pub field: String,
    pub message: String,
}

/// 部品名として使えない名前（`/r/index.json` と衝突するため）。
pub const RESERVED_NAMES: &[&str] = &["index"];

impl Part {
    /// 規則違反をすべて集めて返す。空なら妥当。
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        let mut push = |field: &str, message: String| {
            issues.push(Issue {
                field: field.to_owned(),
                message,
            });
        };

        if !is_kebab_case(&self.name) {
            push(
                "name",
                format!("\"{}\" must be kebab-case (a-z, 0-9, -)", self.name),
            );
        }
        if RESERVED_NAMES.contains(&self.name.as_str()) {
            push("name", format!("\"{}\" is reserved", self.name));
        }
        for (field, value) in [
            ("title", &self.title),
            ("description", &self.description),
            ("reference.name", &self.reference.name),
            ("reference.language", &self.reference.language),
            ("reference.version", &self.reference.version),
            ("usage.rust", &self.usage.rust),
        ] {
            if value.trim().is_empty() {
                push(field, "must not be empty".into());
            }
        }
        for (locale, t) in self.translations.iter().flat_map(|t| t.entries()) {
            for (name, value) in [("title", &t.title), ("description", &t.description)] {
                if value.as_deref().is_some_and(|v| v.trim().is_empty()) {
                    push(
                        &format!("translations.{locale}.{name}"),
                        "must not be empty when set".into(),
                    );
                }
            }
        }
        if let Err(e) = semver::Version::parse(&self.version) {
            push(
                "version",
                format!("\"{}\" is not a semantic version ({e})", self.version),
            );
        }

        if self.shelves.is_empty() {
            push("shelves", "must contain at least one shelf".into());
        }
        if self.shelves.iter().collect::<HashSet<_>>().len() != self.shelves.len() {
            push("shelves", "must not contain duplicates".into());
        }

        if self.rust.files.is_empty() {
            push("rust.files", "must list at least one file".into());
        }
        for (i, file) in self.rust.files.iter().enumerate() {
            if file.trim().is_empty()
                || file.starts_with('/')
                || file.split('/').any(|seg| seg == "..")
            {
                push(
                    &format!("rust.files[{i}]"),
                    format!("\"{file}\" must be a relative path inside the repository"),
                );
            }
        }
        for (i, dep) in self.rust.dependencies.iter().enumerate() {
            if dep.name.trim().is_empty() || dep.version.trim().is_empty() {
                push(
                    &format!("rust.dependencies[{i}]"),
                    "name and version must not be empty".into(),
                );
            }
        }

        match (&self.python, &self.usage.python) {
            (Some(_), None) => push("usage.python", "is required when python is set".into()),
            (None, Some(_)) => push("python", "is required when usage.python is set".into()),
            _ => {}
        }

        if !self.sample && self.environment.is_none() {
            push(
                "environment",
                "is required for measured (non-sample) parts".into(),
            );
        }
        if self.verification.passed > self.verification.cases {
            push(
                "verification.passed",
                format!(
                    "{} must be <= cases ({})",
                    self.verification.passed, self.verification.cases
                ),
            );
        }

        if self.benchmarks.is_empty() {
            push("benchmarks", "must contain at least one point".into());
        }
        for (i, point) in self.benchmarks.iter().enumerate() {
            if point.input_size == 0 {
                push(
                    &format!("benchmarks[{i}].input_size"),
                    "must be > 0 (plotted on a log axis)".into(),
                );
            }
            for (name, ms) in [
                ("rust_ms", point.rust_ms),
                ("reference_ms", point.reference_ms),
            ] {
                if !(ms.is_finite() && ms > 0.0) {
                    push(
                        &format!("benchmarks[{i}].{name}"),
                        format!("{ms} must be a positive number"),
                    );
                }
            }
            if let Some(prev) = i.checked_sub(1).and_then(|j| self.benchmarks.get(j))
                && point.input_size <= prev.input_size
            {
                push(
                    &format!("benchmarks[{i}].input_size"),
                    "benchmarks must be sorted by strictly increasing input_size".into(),
                );
            }
        }

        issues
    }

    /// 説明文の訳がない言語コード。エラーではなく、報告用。
    pub fn missing_translations(&self) -> Vec<&'static str> {
        crate::part::Translations::LOCALES
            .into_iter()
            .filter(|locale| {
                !self
                    .translations
                    .iter()
                    .flat_map(|t| t.entries())
                    .any(|(l, t)| l == *locale && t.description.is_some())
            })
            .collect()
    }

    /// 計測点の中で最大の速度倍率。計測点がなければ None。
    pub fn max_speedup(&self) -> Option<f64> {
        self.benchmarks.iter().map(|p| p.speedup()).reduce(f64::max)
    }
}

fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|seg| {
            !seg.is_empty()
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
