//! unicode-normalize の差分テスト（verify）とベンチマーク（bench）。
//!
//! verify: 標準入力の各ケースについて、unicodedata.normalize・Python の参照実装・NormalizationTest.txt の答えと
//!         一致するかを見る。正規化だけのケースでは is_normalized と「変わらなければ借用で返る」ことも確かめる。
//! bench:  verify/unicode-normalize/bench_fragments.json から Python 側と同じ文章を作り、
//!         入力の大きさ（UTF-8 のバイト数）ごとに NFKC だけ・パイプライン全体の時間を計る。

use std::borrow::Cow;
use std::hint::black_box;

use kura_parts::unicode_normalize::{
    Form, Options, UNICODE_VERSION, fold_whitespace, is_normalized, normalize, normalize_form,
    strip_invisible,
};
use kura_verify::{median_ms, print_json, run_cases};
use serde::{Deserialize, Serialize};
use sha1::{Digest as _, Sha1};

#[derive(Deserialize)]
struct Case {
    source: String,
    /// 入力の文字列（range がないとき）
    #[serde(default)]
    text: String,
    /// lo..=hi のすべての文字（サロゲートを除く）を1文字ずつ処理する。このとき expected は結果の SHA-1（16 進）
    range: Option<(u32, u32)>,
    form: Option<String>,
    fold_whitespace: bool,
    strip_invisible: bool,
    /// 参照実装（Python の unicodedata）の Unicode の版
    unicode_version: String,
    expected: String,
}

#[derive(Serialize)]
struct BenchPoint {
    input_size: u64,
    rust_ms: f64,
    text_sha1: String,
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("verify") => print_json(&run_cases(check)),
        Some("bench") => bench(),
        _ => {
            eprintln!(
                "usage: unicode-normalize verify < cases.jsonl | unicode-normalize bench <NFKC|pipeline> <size>..."
            );
            std::process::exit(2);
        }
    }
}

fn unicode_version() -> String {
    let (major, minor, update) = UNICODE_VERSION;
    format!("{major}.{minor}.{update}")
}

fn sha1_hex(s: &str) -> String {
    Sha1::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn check(case: &Case) -> Result<(), String> {
    let source = &case.source;
    if case.unicode_version != unicode_version() {
        return Err(format!(
            "{source}: the reference implements Unicode {}, the part implements Unicode {}",
            case.unicode_version,
            unicode_version()
        ));
    }
    let form = case
        .form
        .as_deref()
        .map(|f| f.parse::<Form>().map_err(|e| e.to_string()))
        .transpose()?;
    let options = Options {
        form,
        fold_whitespace: case.fold_whitespace,
        strip_invisible: case.strip_invisible,
    };

    if let Some((lo, hi)) = case.range {
        let mut out = String::new();
        for c in (lo..=hi).filter_map(char::from_u32) {
            out.push_str(&normalize(c.encode_utf8(&mut [0; 4]), options));
        }
        let got = sha1_hex(&out);
        return if got == case.expected {
            Ok(())
        } else {
            Err(format!(
                "{source} ({options:?}): SHA-1 of the output is {got}, expected {}",
                case.expected
            ))
        };
    }

    let text = case.text.as_str();
    let what = |path: &str, got: &str| {
        format!(
            "{source}, {path} ({options:?}): {text:?} -> {got:?}, expected {:?}",
            case.expected
        )
    };
    let got = normalize(text, options);
    if got != case.expected.as_str() {
        return Err(what("normalize", &got));
    }
    // 変わらないときだけ借用で返る
    if matches!(got, Cow::Borrowed(_)) != (case.expected == text) {
        return Err(what("normalize (borrowed only when unchanged)", &got));
    }
    // 各段を順に呼んでも同じ
    let mut step = text.to_string();
    if case.strip_invisible {
        step = strip_invisible(&step).into_owned();
    }
    if let Some(form) = form {
        step = normalize_form(&step, form).into_owned();
    }
    if case.fold_whitespace {
        step = fold_whitespace(&step).into_owned();
    }
    if step != case.expected {
        return Err(what("each step in turn", &step));
    }
    if let Some(form) = form {
        if !is_normalized(&case.expected, form) {
            return Err(what("is_normalized(expected)", &got));
        }
        if !case.fold_whitespace
            && !case.strip_invisible
            && is_normalized(text, form) != (text == case.expected)
        {
            return Err(what("is_normalized(input)", &got));
        }
    }
    Ok(())
}

/// Python 側（verify/unicode-normalize/reference.py の bench_text）と同じ手順で、max_size バイト以上の文章を作る。
fn bench_text(max_size: usize) -> String {
    #[derive(Deserialize)]
    struct Spec {
        seed: u64,
        fragments: Vec<String>,
    }
    let path = "verify/unicode-normalize/bench_fragments.json";
    let spec: Spec = serde_json::from_str(
        &std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}")),
    )
    .expect("bench_fragments.json");
    let mut state = spec.seed;
    let mut text = String::with_capacity(max_size + 256);
    while text.len() < max_size {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        text.push_str(&spec.fragments[((state >> 33) % spec.fragments.len() as u64) as usize]);
    }
    text
}

/// 先頭 size バイト（文字の途中で切れる場合はその文字の前まで）。
fn prefix(text: &str, size: usize) -> &str {
    let mut end = size.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn bench() {
    let mut args = std::env::args().skip(2);
    let what = args.next().expect("NFKC or pipeline");
    let options = match what.as_str() {
        "NFKC" => Options::nfkc(),
        "pipeline" => Options::nfkc().fold_whitespace().strip_invisible(),
        other => panic!("unknown benchmark: {other}"),
    };
    let sizes: Vec<u64> = args.map(|a| a.parse().expect("size")).collect();
    let text = bench_text(sizes.iter().copied().max().unwrap_or(0) as usize);
    let points: Vec<BenchPoint> = sizes
        .iter()
        .map(|&size| {
            let input = prefix(&text, size as usize);
            let rust_ms = median_ms(
                || {
                    black_box(normalize(black_box(input), options));
                },
                5,
                300.0,
                1000,
            );
            BenchPoint {
                input_size: size,
                rust_ms,
                text_sha1: sha1_hex(input),
            }
        })
        .collect();
    print_json(&points);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(text: &str, form: Option<&str>, expected: &str) -> Case {
        Case {
            source: "test".into(),
            text: text.into(),
            range: None,
            form: form.map(Into::into),
            fold_whitespace: false,
            strip_invisible: false,
            unicode_version: unicode_version(),
            expected: expected.into(),
        }
    }

    #[test]
    fn accepts_the_right_answer() {
        assert_eq!(check(&case("e\u{301}", Some("NFC"), "\u{E9}")), Ok(()));
        assert_eq!(check(&case("\u{E9}", Some("NFD"), "e\u{301}")), Ok(()));
        assert_eq!(check(&case("", Some("NFKC"), "")), Ok(()));
        let mut c = case(" a\u{200B}  b ", None, "a b");
        c.fold_whitespace = true;
        c.strip_invisible = true;
        assert_eq!(check(&c), Ok(()));
    }

    // 以下は「検証がわざと壊した答えを見逃さない」ことの確認

    #[test]
    fn rejects_a_tampered_expected_value() {
        assert!(check(&case("e\u{301}", Some("NFC"), "e\u{301}")).is_err());
        assert!(check(&case("e\u{301}", Some("NFC"), "\u{E9} ")).is_err());
        assert!(check(&case("e\u{301}", Some("NFC"), "")).is_err());
        assert!(check(&case("", Some("NFC"), "0")).is_err());
        assert!(check(&case("ﬁ", Some("NFKC"), "ﬁ")).is_err());
    }

    #[test]
    fn rejects_the_wrong_form_or_options() {
        assert!(check(&case("e\u{301}", Some("NFD"), "\u{E9}")).is_err());
        assert!(check(&case("ﬁ", Some("NFC"), "fi")).is_err());
        assert!(check(&case("e\u{301}", Some("NFX"), "\u{E9}")).is_err());
        let mut c = case(" a ", None, "a");
        assert!(check(&c).is_err());
        c.fold_whitespace = true;
        assert_eq!(check(&c), Ok(()));
    }

    #[test]
    fn rejects_another_unicode_version() {
        let mut c = case("a", Some("NFC"), "a");
        c.unicode_version = "17.0.0".into();
        assert!(check(&c).is_err());
    }

    #[test]
    fn per_char_cases_compare_the_digest() {
        let mut c = case("", Some("NFD"), &sha1_hex("A\u{300}A\u{301}"));
        c.range = Some((0xC0, 0xC1));
        assert_eq!(check(&c), Ok(()));
        c.expected = sha1_hex("\u{C0}\u{C1}");
        assert!(check(&c).is_err());
        c.expected = c.expected.to_uppercase();
        assert!(check(&c).is_err());
    }

    #[test]
    fn prefix_stops_at_a_character_boundary() {
        assert_eq!(prefix("aあ", 1), "a");
        assert_eq!(prefix("aあ", 2), "a");
        assert_eq!(prefix("aあ", 4), "aあ");
        assert_eq!(prefix("aあ", 100), "aあ");
    }
}
