//! minhash の差分テスト（verify）とベンチマーク（bench）。
//!
//! verify:      標準入力の各ケースについて、datasketch の hashvalues・jaccard とビット単位で一致するかを見る。
//! verify-text: 実文書（Wikipedia）のケース。Rust 側でシングル化し、シングル列の件数と指紋が
//!              Python 側と同じか、署名と jaccard が datasketch と同じかを見る。
//! bench:       合成トークンの数ごとに署名を作る時間を計る（置換の生成は計測に含めない）。
//! bench-text:  実文書ごとに署名を作る時間を計る（シングル化も計測に含めない）。
//! bench-many:  たくさんの段落をまとめて署名する時間を計る（置換の生成も含める。datasketch の MinHash.bulk と同じ）。

use std::hint::black_box;

use kura_parts::minhash::MinHasher;
use kura_verify::shingle::{Mode, digest, shingles};
use kura_verify::{median_ms, print_json, run_cases};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Case {
    num_perm: usize,
    seed: u32,
    /// 16進文字列で表したトークン（任意のバイト列を扱うため）
    a: Vec<String>,
    b: Vec<String>,
    expected_a: Vec<u32>,
    expected_b: Vec<u32>,
    expected_jaccard: f64,
    expected_perm_a: Vec<u32>,
    expected_perm_b: Vec<u32>,
}

/// 実文書のケース。b があるときは組として jaccard も比べる。
#[derive(Deserialize)]
struct TextCase {
    id: String,
    mode: Mode,
    num_perm: usize,
    seed: u32,
    a: String,
    shingles_a: usize,
    digest_a: String,
    expected_a: Vec<u32>,
    b: Option<String>,
    shingles_b: Option<usize>,
    digest_b: Option<String>,
    expected_b: Option<Vec<u32>>,
    expected_jaccard: Option<f64>,
}

/// ベンチマークに使う実文書（verify/run.py が書き出し、Python 側と同じファイルを読む）。
#[derive(Deserialize)]
struct Document {
    mode: Mode,
    text: String,
}

#[derive(Serialize)]
struct ManyResult {
    documents: usize,
    shingles: usize,
    rust_ms: f64,
}

#[derive(Serialize)]
struct BenchPoint {
    input_size: u64,
    rust_ms: f64,
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("verify") => print_json(&run_cases(check)),
        Some("verify-text") => print_json(&run_cases(check_text)),
        Some("bench") => bench(),
        Some("bench-text") => bench_text(),
        Some("bench-many") => bench_many(),
        _ => {
            eprintln!(
                "usage: minhash verify < cases.jsonl | minhash verify-text < cases.jsonl | \
                 minhash bench <num_perm> <size>... | minhash bench-text <num_perm> <docs.json> | \
                 minhash bench-many <num_perm> <docs.json>"
            );
            std::process::exit(2);
        }
    }
}

fn check(case: &Case) -> Result<(), String> {
    let hasher = MinHasher::new(case.num_perm, case.seed);
    let (perm_a, perm_b) = hasher.permutations();
    if perm_a != case.expected_perm_a || perm_b != case.expected_perm_b {
        return Err(format!(
            "permutations differ (num_perm={}, seed={})",
            case.num_perm, case.seed
        ));
    }
    let a = hasher.signature(case.a.iter().map(|t| hex(t)));
    let b = hasher.signature(case.b.iter().map(|t| hex(t)));
    if a.values() != case.expected_a {
        return Err(format!(
            "signature a differs (num_perm={}, seed={})",
            case.num_perm, case.seed
        ));
    }
    if b.values() != case.expected_b {
        return Err(format!(
            "signature b differs (num_perm={}, seed={})",
            case.num_perm, case.seed
        ));
    }
    let jaccard = a.jaccard(&b);
    if jaccard.to_bits() != case.expected_jaccard.to_bits() {
        return Err(format!("jaccard {jaccard} != {}", case.expected_jaccard));
    }
    Ok(())
}

fn check_text(case: &TextCase) -> Result<(), String> {
    let hasher = MinHasher::new(case.num_perm, case.seed);
    let a = shingles(&case.a, case.mode);
    compare_shingles(&case.id, "a", &a, case.shingles_a, &case.digest_a)?;
    let sig_a = hasher.signature(&a);
    if sig_a.values() != case.expected_a {
        return Err(format!("{}: signature a differs", case.id));
    }
    let (Some(text_b), Some(count_b), Some(digest_b), Some(expected_b), Some(expected_jaccard)) = (
        &case.b,
        case.shingles_b,
        &case.digest_b,
        &case.expected_b,
        case.expected_jaccard,
    ) else {
        return match case.b {
            None => Ok(()),
            Some(_) => Err(format!("{}: incomplete pair", case.id)),
        };
    };
    let b = shingles(text_b, case.mode);
    compare_shingles(&case.id, "b", &b, count_b, digest_b)?;
    let sig_b = hasher.signature(&b);
    if sig_b.values() != expected_b {
        return Err(format!("{}: signature b differs", case.id));
    }
    let jaccard = sig_a.jaccard(&sig_b);
    if jaccard.to_bits() != expected_jaccard.to_bits() {
        return Err(format!(
            "{}: jaccard {jaccard} != {expected_jaccard}",
            case.id
        ));
    }
    Ok(())
}

/// Rust 側で作ったシングル列が、Python 側のものと件数・指紋とも同じか。
fn compare_shingles(
    id: &str,
    side: &str,
    got: &[String],
    count: usize,
    expected: &str,
) -> Result<(), String> {
    if got.len() != count || digest(got) != expected {
        return Err(format!(
            "{id}: shingles {side} differ from Python ({} vs {count})",
            got.len()
        ));
    }
    Ok(())
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

fn bench() {
    let args: Vec<u64> = std::env::args()
        .skip(2)
        .map(|a| a.parse().expect("number"))
        .collect();
    let (&num_perm, sizes) = args.split_first().expect("num_perm and sizes");
    let hasher = MinHasher::new(num_perm as usize, 1);
    let points: Vec<BenchPoint> = sizes
        .iter()
        .map(|&size| {
            // Python 側と同じトークン（"token-<i>" の UTF-8）
            let tokens: Vec<Vec<u8>> = (0..size)
                .map(|i| format!("token-{i}").into_bytes())
                .collect();
            let rust_ms = median_ms(
                || drop(black_box(hasher.signature(black_box(&tokens)))),
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

/// 引数の num_perm と、文書ファイルを読んでシングル化したもの（バイト列）。
fn documents_from_args() -> (usize, Vec<Vec<Vec<u8>>>) {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let [num_perm, path] = args.as_slice() else {
        panic!("expected: <num_perm> <docs.json>");
    };
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let docs: Vec<Document> = serde_json::from_str(&text).expect("docs.json");
    let shingled = docs
        .iter()
        .map(|d| {
            shingles(&d.text, d.mode)
                .into_iter()
                .map(String::into_bytes)
                .collect()
        })
        .collect();
    (num_perm.parse().expect("num_perm"), shingled)
}

fn bench_text() {
    let (num_perm, docs) = documents_from_args();
    let hasher = MinHasher::new(num_perm, 1);
    let points: Vec<BenchPoint> = docs
        .iter()
        .map(|grams| BenchPoint {
            input_size: grams.len() as u64,
            rust_ms: median_ms(
                || drop(black_box(hasher.signature(black_box(grams)))),
                5,
                300.0,
                1000,
            ),
        })
        .collect();
    print_json(&points);
}

fn bench_many() {
    let (num_perm, docs) = documents_from_args();
    let rust_ms = median_ms(
        || {
            let hasher = MinHasher::new(num_perm, 1);
            let signatures: Vec<_> = docs.iter().map(|d| hasher.signature(d)).collect();
            drop(black_box(signatures));
        },
        5,
        1000.0,
        1000,
    );
    print_json(&ManyResult {
        documents: docs.len(),
        shingles: docs.iter().map(Vec::len).sum(),
        rust_ms,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rust 自身の結果から「正しい」ケースを作る（照合の仕組みを確かめるため。datasketch との一致は run.py で見る）。
    fn case(mode: Mode, a: &str, b: Option<&str>) -> TextCase {
        let hasher = MinHasher::new(128, 1);
        let ga = shingles(a, mode);
        let sa = hasher.signature(&ga);
        let mut case = TextCase {
            id: "test".into(),
            mode,
            num_perm: 128,
            seed: 1,
            a: a.into(),
            shingles_a: ga.len(),
            digest_a: digest(&ga),
            expected_a: sa.values().to_vec(),
            b: None,
            shingles_b: None,
            digest_b: None,
            expected_b: None,
            expected_jaccard: None,
        };
        if let Some(b) = b {
            let gb = shingles(b, mode);
            let sb = hasher.signature(&gb);
            case.b = Some(b.into());
            case.shingles_b = Some(gb.len());
            case.digest_b = Some(digest(&gb));
            case.expected_b = Some(sb.values().to_vec());
            case.expected_jaccard = Some(sa.jaccard(&sb));
        }
        case
    }

    const A: &str = "The quick brown fox jumps over the lazy dog.";
    const B: &str = "The quick brown fox leaps over the lazy dog.";

    #[test]
    fn accepts_matching_single_and_pair() {
        assert_eq!(check_text(&case(Mode::Word, A, None)), Ok(()));
        assert_eq!(check_text(&case(Mode::Word, A, Some(B))), Ok(()));
        assert_eq!(
            check_text(&case(Mode::Char, "日本語の文章。", Some("日本語の文書。"))),
            Ok(())
        );
        assert_eq!(check_text(&case(Mode::Word, "", Some(""))), Ok(()));
    }

    #[test]
    fn rejects_wrong_shingle_count_or_digest() {
        let mut c = case(Mode::Word, A, Some(B));
        c.shingles_a += 1;
        assert!(check_text(&c).unwrap_err().contains("shingles a"));
        let mut c = case(Mode::Word, A, Some(B));
        c.digest_b = Some("0".repeat(40));
        assert!(check_text(&c).unwrap_err().contains("shingles b"));
    }

    #[test]
    fn rejects_the_other_mode() {
        let mut c = case(Mode::Word, A, None);
        c.mode = Mode::Char;
        assert!(check_text(&c).is_err());
    }

    #[test]
    fn rejects_wrong_signature_or_jaccard() {
        let mut c = case(Mode::Word, A, Some(B));
        c.expected_a[0] ^= 1;
        assert!(check_text(&c).unwrap_err().contains("signature a"));
        let mut c = case(Mode::Word, A, Some(B));
        c.expected_b.as_mut().unwrap()[127] ^= 1;
        assert!(check_text(&c).unwrap_err().contains("signature b"));
        let mut c = case(Mode::Word, A, Some(B));
        c.expected_jaccard = Some(f64::from_bits(c.expected_jaccard.unwrap().to_bits() + 1));
        assert!(check_text(&c).unwrap_err().contains("jaccard"));
    }

    #[test]
    fn rejects_incomplete_pair() {
        let mut c = case(Mode::Word, A, Some(B));
        c.expected_jaccard = None;
        assert!(check_text(&c).unwrap_err().contains("incomplete"));
    }

    #[test]
    fn reads_the_json_written_by_reference_py() {
        let line = r#"{"id": "en:1:p0", "mode": "word", "num_perm": 2, "seed": 1, "a": "x", "shingles_a": 1, "digest_a": "d", "expected_a": [1, 2], "b": null, "shingles_b": null, "digest_b": null, "expected_b": null, "expected_jaccard": null}"#;
        let c: TextCase = serde_json::from_str(line).unwrap();
        assert_eq!(c.mode, Mode::Word);
        assert!(c.b.is_none());
        let docs: Vec<Document> =
            serde_json::from_str(r#"[{"id": "x", "mode": "char", "text": "日本"}]"#).unwrap();
        assert_eq!(docs[0].mode, Mode::Char);
        assert_eq!(docs[0].text, "日本");
    }
}
