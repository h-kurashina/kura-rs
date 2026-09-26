//! `registry/*.json` の読み込みと検証。CLI（kura-registry）とサーバー（kura-server）で共有する。

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::part::Part;
use crate::validate::Issue;

/// 読み込みに失敗した理由。1 ファイルにつき 1 件以上。
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{file}: {source}")]
    Parse {
        file: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("{file}: {issue}")]
    Invalid { file: String, issue: Issue },
    #[error("{file}: file name must be \"{expected}\" to match its name")]
    FileName { file: String, expected: String },
    #[error("{file}: name \"{name}\" is already used by {other}")]
    Duplicate {
        file: String,
        name: String,
        other: String,
    },
    #[error("no registry files found in {0}")]
    Empty(PathBuf),
}

/// 全ファイル分の問題をまとめたもの。1 件でもあれば全体を不正とする。
#[derive(Debug, thiserror::Error)]
pub struct RegistryError(pub Vec<LoadError>);

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "invalid registry ({} problem(s)):", self.0.len())?;
        for e in &self.0 {
            writeln!(f, "  - {e}")?;
        }
        Ok(())
    }
}

/// JSON 文字列 1 件を読み、型とルールの両方で検証する。
pub fn parse_part(file: &str, json: &str) -> Result<Part, Vec<LoadError>> {
    let part: Part = serde_json::from_str(json).map_err(|source| {
        vec![LoadError::Parse {
            file: file.to_owned(),
            source,
        }]
    })?;

    let mut errors: Vec<LoadError> = part
        .validate()
        .into_iter()
        .map(|issue| LoadError::Invalid {
            file: file.to_owned(),
            issue,
        })
        .collect();

    let expected = format!("{}.json", part.name);
    if file != expected {
        errors.push(LoadError::FileName {
            file: file.to_owned(),
            expected,
        });
    }

    if errors.is_empty() {
        Ok(part)
    } else {
        Err(errors)
    }
}

/// ディレクトリ内の `*.json` をすべて読み込む。名前順で返す。
pub fn load_dir(dir: &Path) -> Result<Vec<Part>, RegistryError> {
    let io = |source| {
        RegistryError(vec![LoadError::Io {
            path: dir.to_owned(),
            source,
        }])
    };

    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(io)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(io)?;
    files.retain(|p| p.extension().is_some_and(|ext| ext == "json"));
    files.sort();

    if files.is_empty() {
        return Err(RegistryError(vec![LoadError::Empty(dir.to_owned())]));
    }

    let mut errors = Vec::new();
    let mut parts = Vec::new();
    let mut seen: HashMap<String, String> = HashMap::new();

    for path in files {
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let json = match std::fs::read_to_string(&path) {
            Ok(json) => json,
            Err(source) => {
                errors.push(LoadError::Io { path, source });
                continue;
            }
        };
        match parse_part(&file, &json) {
            Ok(part) => {
                if let Some(other) = seen.insert(part.name.clone(), file.clone()) {
                    errors.push(LoadError::Duplicate {
                        file,
                        name: part.name.clone(),
                        other,
                    });
                } else {
                    parts.push(part);
                }
            }
            Err(mut es) => errors.append(&mut es),
        }
    }

    if !errors.is_empty() {
        return Err(RegistryError(errors));
    }
    parts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(parts)
}
