//! file-hash の差分テスト（verify）とベンチマーク（bench）。
//!
//! verify: 標準入力の各ケースについて、hashlib.sha256 / blake3 パッケージの答えと一致するかを、
//!         すべての入口（hash_bytes・Hasher への分割入力・io::Write・短い読み込みの hash_reader・hash_file）で見る。
//! bench:  入力の大きさ（バイト）ごとに、メモリ上のバイト列を hash_bytes にかける時間を計る。

use std::hint::black_box;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use kura_parts::file_hash::{Algorithm, Hasher, hash_bytes, hash_file, hash_reader};
use kura_verify::{median_ms, print_json, run_cases};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Case {
    algorithm: String,
    source: Source,
    len: usize,
    /// Hasher::update に渡す大きさの列（残りは最後にまとめて渡す）
    chunks: Vec<usize>,
    /// Reader が1回の read で返す大きさの列
    reads: Vec<usize>,
    expected: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Source {
    Hex { hex: String },
    Fill { byte: u8, len: usize },
    Cycle251 { len: usize },
    File { path: PathBuf },
}

#[derive(Serialize)]
struct BenchPoint {
    input_size: u64,
    rust_ms: f64,
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("verify") => print_json(&run_cases(check)),
        Some("bench") => bench(),
        _ => {
            eprintln!(
                "usage: file-hash verify < cases.jsonl | file-hash bench <sha256|blake3> <size>..."
            );
            std::process::exit(2);
        }
    }
}

fn cycle251(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn check(case: &Case) -> Result<(), String> {
    let algorithm: Algorithm = case.algorithm.parse().map_err(|e| format!("{e}"))?;
    let data = match &case.source {
        Source::Hex { hex } => unhex(hex)?,
        Source::Fill { byte, len } => vec![*byte; *len],
        Source::Cycle251 { len } => cycle251(*len),
        Source::File { path } => std::fs::read(path).map_err(|e| format!("{path:?}: {e}"))?,
    };
    if data.len() != case.len {
        return Err(format!(
            "input has {} bytes, expected {}",
            data.len(),
            case.len
        ));
    }
    let what = format!("{algorithm}, {} bytes", data.len());
    let compare = |path: &str, got: String| {
        if got == case.expected {
            Ok(())
        } else {
            Err(format!(
                "{path} differs ({what}): {got} != {}",
                case.expected
            ))
        }
    };

    compare("hash_bytes", hash_bytes(algorithm, &data).to_hex())?;

    let mut hasher = Hasher::new(algorithm);
    let mut rest = data.as_slice();
    for &size in &case.chunks {
        let n = size.min(rest.len());
        hasher.update(&rest[..n]);
        rest = &rest[n..];
    }
    hasher.update(rest);
    compare("Hasher::update (split)", hasher.finalize().to_hex())?;

    let mut writer = Hasher::new(algorithm);
    writer.write_all(&data).map_err(|e| e.to_string())?;
    compare("io::Write", writer.finalize().to_hex())?;

    let reader = ShortReader {
        data: &data,
        sizes: &case.reads,
        next: 0,
    };
    let digest = hash_reader(algorithm, reader).map_err(|e| e.to_string())?;
    compare("hash_reader (short reads)", digest.to_hex())?;

    if let Source::File { path } = &case.source {
        let digest = hash_file(algorithm, path).map_err(|e| format!("{path:?}: {e}"))?;
        compare("hash_file", digest.to_hex())?;
    }
    Ok(())
}

/// 決められた大きさずつしか返さない Reader（ファイルやソケットの短い読み込みを再現する）。
struct ShortReader<'a> {
    data: &'a [u8],
    sizes: &'a [usize],
    next: usize,
}

impl Read for ShortReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let size = match self.sizes.get(self.next) {
            // 0 を返すと終わりの合図になるので、残りがあるうちは最低 1 バイト返す
            Some(&s) => s.max(1),
            None => usize::MAX,
        };
        self.next += 1;
        let n = size.min(buf.len()).min(self.data.len());
        buf[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];
        Ok(n)
    }
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn bench() {
    let mut args = std::env::args().skip(2);
    let algorithm: Algorithm = args
        .next()
        .expect("algorithm")
        .parse()
        .expect("sha256 or blake3");
    let points: Vec<BenchPoint> = args
        .map(|a| a.parse::<u64>().expect("size"))
        .map(|size| {
            // Python 側と同じバイト列（中身は速さに関係しない）
            let data = cycle251(size as usize);
            let rust_ms = median_ms(
                || {
                    black_box(hash_bytes(algorithm, black_box(&data)));
                },
                5,
                300.0,
                1000,
            );
            BenchPoint {
                input_size: size,
                rust_ms,
            }
        })
        .collect();
    print_json(&points);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn case(algorithm: &str, hex: &str, expected: &str) -> Case {
        Case {
            algorithm: algorithm.into(),
            source: Source::Hex { hex: hex.into() },
            len: hex.len() / 2,
            chunks: vec![1, 0, 1],
            reads: vec![1, 1],
            expected: expected.into(),
        }
    }

    #[test]
    fn accepts_the_right_answer() {
        assert_eq!(check(&case("sha256", "616263", ABC_SHA256)), Ok(()));
    }

    // 以下は「検証がわざと壊した答えを見逃さない」ことの確認

    #[test]
    fn rejects_a_tampered_expected_value() {
        for i in 0..ABC_SHA256.len() {
            let mut tampered = ABC_SHA256.as_bytes().to_vec();
            tampered[i] = if tampered[i] == b'0' { b'1' } else { b'0' };
            let tampered = String::from_utf8(tampered).unwrap();
            assert!(check(&case("sha256", "616263", &tampered)).is_err(), "{i}");
        }
    }

    #[test]
    fn rejects_tampered_input() {
        assert!(check(&case("sha256", "616264", ABC_SHA256)).is_err());
        assert!(check(&case("sha256", "61626300", ABC_SHA256)).is_err());
        assert!(check(&case("sha256", "", ABC_SHA256)).is_err());
    }

    #[test]
    fn rejects_the_wrong_algorithm() {
        assert!(check(&case("blake3", "616263", ABC_SHA256)).is_err());
        assert!(check(&case("md5", "616263", ABC_SHA256)).is_err());
    }

    #[test]
    fn rejects_a_wrong_length_and_missing_file() {
        let mut c = case("sha256", "616263", ABC_SHA256);
        c.len = 4;
        assert!(check(&c).is_err());
        let mut c = case("sha256", "", ABC_SHA256);
        c.source = Source::File {
            path: "/nonexistent/kura-file-hash".into(),
        };
        assert!(check(&c).is_err());
    }

    #[test]
    fn rejects_an_uppercase_expected_value() {
        // Python の hexdigest() は小文字なので、大文字は別物として扱う（比べ方がゆるくなっていないか）
        assert!(check(&case("sha256", "616263", &ABC_SHA256.to_uppercase())).is_err());
    }
}
