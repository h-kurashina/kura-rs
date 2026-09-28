//! unicode_normalize の例によるテスト。Python 3.14 の unicodedata で確かめた答えと比べる。
//! 公式の NormalizationTest.txt と、すべての符号位置での Python との突き合わせは verify/run.py で行う。

use std::borrow::Cow;

use kura_parts::unicode_normalize::{
    Form, INVISIBLE, Options, UNICODE_VERSION, WHITESPACE, fold_whitespace, is_invisible,
    is_normalized, is_whitespace, normalize, normalize_form, strip_invisible,
};
use unicode_normalization::UnicodeNormalization;

fn nf(form: Form, s: &str) -> String {
    normalize_form(s, form).into_owned()
}

#[test]
fn unicode_version_matches_python_3_14() {
    // Python 3.14 の unicodedata.unidata_version は "16.0.0"。版を上げるときは verify/run.py も通すこと
    assert_eq!(UNICODE_VERSION, (16, 0, 0));
}

#[test]
fn form_names_round_trip() {
    for form in Form::ALL {
        assert_eq!(form.name().parse::<Form>(), Ok(form));
        assert_eq!(form.name().to_lowercase().parse::<Form>(), Ok(form));
        assert_eq!(form.to_string(), form.name());
    }
    for bad in ["", "NF", "NFKCX", "nfc ", "NFKC_CF"] {
        assert!(bad.parse::<Form>().is_err(), "{bad:?}");
    }
}

#[test]
fn known_normalizations() {
    // (入力, NFC, NFD, NFKC, NFKD)。答えは unicodedata.normalize で確かめたもの
    let cases: &[(&str, &str, &str, &str, &str)] = &[
        ("", "", "", "", ""),
        ("abc", "abc", "abc", "abc", "abc"),
        ("e\u{301}", "\u{E9}", "e\u{301}", "\u{E9}", "e\u{301}"),
        ("\u{E9}", "\u{E9}", "e\u{301}", "\u{E9}", "e\u{301}"),
        // ANGSTROM SIGN は単独の分解（singleton）
        ("\u{212B}", "\u{C5}", "A\u{30A}", "\u{C5}", "A\u{30A}"),
        // 合字と全角・半角
        ("\u{FB01}", "\u{FB01}", "\u{FB01}", "fi", "fi"),
        ("ＡＩ１２３", "ＡＩ１２３", "ＡＩ１２３", "AI123", "AI123"),
        ("ｶﾞ", "ｶﾞ", "ｶﾞ", "ガ", "カ\u{3099}"),
        ("が", "が", "か\u{3099}", "が", "か\u{3099}"),
        // ハングル音節と字母
        (
            "한",
            "한",
            "\u{1112}\u{1161}\u{11AB}",
            "한",
            "\u{1112}\u{1161}\u{11AB}",
        ),
        (
            "\u{1112}\u{1161}\u{11AB}",
            "한",
            "\u{1112}\u{1161}\u{11AB}",
            "한",
            "\u{1112}\u{1161}\u{11AB}",
        ),
        // CJK 互換漢字は NFC でも統合漢字になる
        ("\u{F900}", "\u{8C48}", "\u{8C48}", "\u{8C48}", "\u{8C48}"),
        // 結合文字の並べ替え（下の点 ccc=220 と上の点 ccc=230）
        (
            "q\u{307}\u{323}",
            "q\u{323}\u{307}",
            "q\u{323}\u{307}",
            "q\u{323}\u{307}",
            "q\u{323}\u{307}",
        ),
        (
            "\u{1E0B}\u{323}",
            "\u{1E0D}\u{307}",
            "d\u{323}\u{307}",
            "\u{1E0D}\u{307}",
            "d\u{323}\u{307}",
        ),
        // 先頭の結合文字（土台がない）
        ("\u{301}a", "\u{301}a", "\u{301}a", "\u{301}a", "\u{301}a"),
        // 丸数字・組文字
        ("①㈱㌔", "①㈱㌔", "①㈱㌔", "1(株)キロ", "1(株)キロ"),
        // ヘブライ文字の点（niqqud）と合成除外の表示形
        (
            "\u{FB2C}",
            "\u{5E9}\u{5BC}\u{5C1}",
            "\u{5E9}\u{5BC}\u{5C1}",
            "\u{5E9}\u{5BC}\u{5C1}",
            "\u{5E9}\u{5BC}\u{5C1}",
        ),
        // アラビア文字の表示形（NFKC で基本の文字に戻る）
        (
            "\u{FEFB}",
            "\u{FEFB}",
            "\u{FEFB}",
            "\u{644}\u{627}",
            "\u{644}\u{627}",
        ),
        // 絵文字の ZWJ 連結は正規化では変わらない
        (
            "👨\u{200D}👩\u{200D}👧",
            "👨\u{200D}👩\u{200D}👧",
            "👨\u{200D}👩\u{200D}👧",
            "👨\u{200D}👩\u{200D}👧",
            "👨\u{200D}👩\u{200D}👧",
        ),
        // NFKC でスペースと結合文字になる（U+00A8 DIAERESIS）
        ("\u{A8}", "\u{A8}", "\u{A8}", " \u{308}", " \u{308}"),
        // 未割り当ての符号位置と私用領域はそのまま
        (
            "\u{378}\u{E000}\u{10FFFF}",
            "\u{378}\u{E000}\u{10FFFF}",
            "\u{378}\u{E000}\u{10FFFF}",
            "\u{378}\u{E000}\u{10FFFF}",
            "\u{378}\u{E000}\u{10FFFF}",
        ),
    ];
    for &(input, nfc, nfd, nfkc, nfkd) in cases {
        assert_eq!(nf(Form::Nfc, input), nfc, "NFC {input:?}");
        assert_eq!(nf(Form::Nfd, input), nfd, "NFD {input:?}");
        assert_eq!(nf(Form::Nfkc, input), nfkc, "NFKC {input:?}");
        assert_eq!(nf(Form::Nfkd, input), nfkd, "NFKD {input:?}");
        for (form, expected) in [
            (Form::Nfc, nfc),
            (Form::Nfd, nfd),
            (Form::Nfkc, nfkc),
            (Form::Nfkd, nfkd),
        ] {
            assert_eq!(
                is_normalized(input, form),
                input == expected,
                "{form} {input:?}"
            );
            assert!(is_normalized(expected, form), "{form} {expected:?}");
        }
    }
}

#[test]
fn borrows_when_nothing_changes() {
    let clean = "Hello, 世界. データ AI";
    assert!(matches!(normalize_form(clean, Form::Nfc), Cow::Borrowed(_)));
    assert!(matches!(
        normalize_form(clean, Form::Nfkc),
        Cow::Borrowed(_)
    ));
    assert!(matches!(fold_whitespace(clean), Cow::Borrowed(_)));
    assert!(matches!(strip_invisible(clean), Cow::Borrowed(_)));
    assert!(matches!(
        normalize(clean, Options::nfkc().fold_whitespace().strip_invisible()),
        Cow::Borrowed(_)
    ));
    assert!(matches!(
        normalize_form("e\u{301}", Form::Nfc),
        Cow::Owned(_)
    ));
}

#[test]
fn pipeline_example() {
    let raw = "\u{FEFF}ﾃﾞｰﾀ\u{3000}\u{3000}ＡＩ\u{200B}  ";
    assert_eq!(
        normalize(raw, Options::nfkc().fold_whitespace().strip_invisible()),
        "データ AI"
    );
    assert_eq!(normalize(raw, Options::new()), raw);
    // 見えない文字を先に消すので、空白の間にあってもまとまる
    assert_eq!(
        normalize(
            "a \u{200B} b",
            Options::new().fold_whitespace().strip_invisible()
        ),
        "a b"
    );
    // 空白は正規化の後にまとめる（NFKC で NBSP が U+0020 になる）
    assert_eq!(
        normalize("a\u{A0} b", Options::nfkc().fold_whitespace()),
        "a b"
    );
    assert_eq!(
        normalize("a\u{A0} b", Options::nfc().fold_whitespace()),
        "a b"
    );
    // 見えない文字を消すと、文字と結合文字がつながって合成される
    assert_eq!(
        normalize("e\u{200B}\u{301}", Options::nfc().strip_invisible()),
        "\u{E9}"
    );
}

#[test]
fn fold_whitespace_examples() {
    assert_eq!(fold_whitespace(""), "");
    assert_eq!(fold_whitespace(" \t\r\n "), "");
    assert_eq!(fold_whitespace("  a  b\n\nc\t"), "a b c");
    assert_eq!(fold_whitespace("a\u{3000}b\u{85}c\u{1C}d"), "a b c d");
    // ZWSP と BOM は空白ではない（見えない文字として消す側）
    assert_eq!(fold_whitespace("a\u{200B}b\u{FEFF}"), "a\u{200B}b\u{FEFF}");
}

#[test]
fn strip_invisible_examples() {
    assert_eq!(
        strip_invisible("in\u{AD}vis\u{200B}i\u{2060}ble\u{FEFF}"),
        "invisible"
    );
    assert_eq!(strip_invisible("👨\u{200D}👩\u{200D}👧"), "👨👩👧");
    // タグ文字に隠した文字列（ASCII smuggling）を消す
    assert_eq!(
        strip_invisible("hi\u{E0001}\u{E0069}\u{E0067}\u{E007F}"),
        "hi"
    );
    // 異体字セレクタ・CGJ・ハングルのフィラー・見える書式文字は残す
    let kept = "\u{2764}\u{FE0F}a\u{34F}\u{3164}\u{600}\u{E0100}";
    assert_eq!(strip_invisible(kept), kept);
}

#[test]
fn whitespace_table_is_python_isspace() {
    assert!(WHITESPACE.is_sorted());
    for c in (0..=0x10FFFF).filter_map(char::from_u32) {
        assert_eq!(is_whitespace(c), WHITESPACE.contains(&c), "{c:?}");
        // Rust の char::is_whitespace との違いは U+001C..U+001F だけ
        assert_eq!(
            is_whitespace(c),
            c.is_whitespace() || ('\u{1C}'..='\u{1F}').contains(&c),
            "{c:?}"
        );
    }
}

#[test]
fn invisible_table_is_sorted_and_disjoint_from_whitespace() {
    for w in INVISIBLE.windows(2) {
        assert!(
            w[0].0 <= w[0].1 && (w[0].1 as u32) + 1 < w[1].0 as u32,
            "{w:?}"
        );
    }
    let mut count = 0;
    for c in (0..=0x10FFFF).filter_map(char::from_u32) {
        let listed = INVISIBLE.iter().any(|&(lo, hi)| lo <= c && c <= hi);
        assert_eq!(is_invisible(c), listed, "{c:?}");
        assert!(!(is_invisible(c) && is_whitespace(c)), "{c:?}");
        count += usize::from(listed);
    }
    // U+00AD, U+061C, U+180E, 5 + 5 + 5 + 10, U+FEFF, 4 + 8, U+E0001, 96（Python で数えた Cf かつ Default_Ignorable の数）
    assert_eq!(count, 138);
}

#[test]
fn very_long_combining_runs() {
    // 結合文字が何万個続いても、素直に全体を正規化した結果と同じ（並べ替えも含めて）
    let marks = [
        '\u{301}',
        '\u{323}',
        '\u{334}',
        '\u{5B0}',
        '\u{93C}',
        '\u{F71}',
        '\u{1D165}',
    ];
    let mut s = String::from("a");
    for i in 0..30_000 {
        s.push(marks[(i * 7 + i / 3) % marks.len()]);
    }
    s.push_str("b\u{308}");
    for form in Form::ALL {
        let plain: String = match form {
            Form::Nfc => s.chars().nfc().collect(),
            Form::Nfd => s.chars().nfd().collect(),
            Form::Nfkc => s.chars().nfkc().collect(),
            Form::Nfkd => s.chars().nfkd().collect(),
        };
        assert_eq!(nf(form, &s), plain, "{form}");
        assert!(is_normalized(&plain, form));
    }
}

#[test]
fn every_code_point_alone_matches_the_crate() {
    // 区切り方の工夫（境目の判定）が、1文字だけの入力でも素直な計算と同じになるか
    for c in (0..=0x10FFFF).filter_map(char::from_u32) {
        let s = c.to_string();
        assert_eq!(
            nf(Form::Nfc, &s),
            s.chars().nfc().collect::<String>(),
            "{c:?}"
        );
        assert_eq!(
            nf(Form::Nfd, &s),
            s.chars().nfd().collect::<String>(),
            "{c:?}"
        );
        assert_eq!(
            nf(Form::Nfkc, &s),
            s.chars().nfkc().collect::<String>(),
            "{c:?}"
        );
        assert_eq!(
            nf(Form::Nfkd, &s),
            s.chars().nfkd().collect::<String>(),
            "{c:?}"
        );
    }
}
