//! unicode_normalize の性質テスト。ランダムな文字列を大量に作り、どんな入力でも成り立つべき性質を確かめる。
//! 1つの性質につき既定で 2,000 通り。PROPTEST_CASES=100000 のように環境変数で増やせる。

use std::borrow::Cow;

use kura_parts::unicode_normalize::{
    Form, Options, fold_whitespace, is_invisible, is_normalized, is_whitespace, normalize,
    normalize_form, strip_invisible,
};
use proptest::prelude::*;
use unicode_normalization::UnicodeNormalization;

fn config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000);
    ProptestConfig {
        cases,
        ..ProptestConfig::default()
    }
}

/// 正規化で何かが起きやすい文字の範囲（結合文字・ハングル・互換文字・合成除外・空白・見えない文字など）
const TRICKY: &[(char, char)] = &[
    ('\u{0}', '\u{7F}'),        // ASCII
    ('\u{A0}', '\u{24F}'),      // ラテン文字（合成済み文字・NBSP・¨ など）
    ('\u{300}', '\u{36F}'),     // 結合用のダイアクリティカルマーク
    ('\u{370}', '\u{3FF}'),     // ギリシャ文字（単独の分解を含む）
    ('\u{591}', '\u{5F4}'),     // ヘブライ文字と niqqud
    ('\u{600}', '\u{6FF}'),     // アラビア文字
    ('\u{900}', '\u{97F}'),     // デーバナーガリー（ヌクタの合成除外）
    ('\u{9BC}', '\u{9D7}'),     // ベンガル文字（2 つに分かれる母音記号）
    ('\u{B3C}', '\u{B57}'),     // オリヤー文字（starter 同士の合成）
    ('\u{BBE}', '\u{BD7}'),     // タミル文字
    ('\u{D3E}', '\u{D57}'),     // マラヤーラム文字
    ('\u{F71}', '\u{F81}'),     // チベット文字（分解が非 starter で始まる）
    ('\u{1100}', '\u{11FF}'),   // ハングル字母（L・V・T）
    ('\u{1E00}', '\u{1FFF}'),   // ラテン拡張追加・ギリシャ拡張
    ('\u{2000}', '\u{206F}'),   // 一般句読点（空白・ZWSP・ZWJ・双方向制御）
    ('\u{2460}', '\u{24FF}'),   // 丸数字
    ('\u{3000}', '\u{30FF}'),   // 和文の空白・かな・濁点
    ('\u{3131}', '\u{318E}'),   // ハングル互換字母
    ('\u{3200}', '\u{33FF}'),   // 組文字・単位記号
    ('\u{AC00}', '\u{AC40}'),   // ハングル音節（LV と LVT）
    ('\u{D7A0}', '\u{D7FF}'),   // ハングル音節の終わりと字母拡張 B
    ('\u{F900}', '\u{FAFF}'),   // CJK 互換漢字
    ('\u{FB00}', '\u{FB4F}'),   // 合字・ヘブライ文字の表示形
    ('\u{FE00}', '\u{FE0F}'),   // 異体字セレクタ
    ('\u{FE20}', '\u{FE2F}'),   // 結合用半記号
    ('\u{FE70}', '\u{FEFF}'),   // アラビア文字の表示形・BOM
    ('\u{FF00}', '\u{FFEF}'),   // 全角・半角
    ('\u{110BA}', '\u{110BA}'), // カイティー文字のヌクタ
    ('\u{11099}', '\u{110AB}'), // カイティー文字（合成あり）
    ('\u{1133B}', '\u{1134D}'), // グランタ文字
    ('\u{1D15E}', '\u{1D17A}'), // 音楽記号（合成除外・書式制御）
    ('\u{1F3FB}', '\u{1F3FF}'), // 肌の色の修飾子
    ('\u{1F468}', '\u{1F469}'), // 絵文字（ZWJ 連結に使う）
    ('\u{2F800}', '\u{2FA1D}'), // CJK 互換漢字補助
    ('\u{E0000}', '\u{E007F}'), // タグ文字
];

fn tricky_char() -> impl Strategy<Value = char> {
    prop::sample::select(TRICKY.to_vec()).prop_flat_map(|(lo, hi)| prop::char::range(lo, hi))
}

/// 空・短い・長い文字列。偏った文字を多めに、すべての面の任意の文字も混ぜる
fn text() -> impl Strategy<Value = String> {
    let ch = prop_oneof![
        6 => tricky_char(),
        2 => any::<char>(),
        1 => prop::sample::select(vec![' ', ' ', '\u{3000}', '\t', '\n', '\u{200B}', '\u{200D}', '\u{FEFF}', '\u{301}']),
    ];
    let string = |chars: Vec<char>| chars.into_iter().collect::<String>();
    prop_oneof![
        4 => prop::collection::vec(ch.clone(), 0..20).prop_map(string),
        3 => prop::collection::vec(ch.clone(), 0..300).prop_map(string),
        1 => prop::collection::vec(ch, 0..3000).prop_map(string),
        1 => any::<String>(),
    ]
}

fn form() -> impl Strategy<Value = Form> {
    prop::sample::select(Form::ALL.to_vec())
}

fn plain(form: Form, s: &str) -> String {
    match form {
        Form::Nfc => s.chars().nfc().collect(),
        Form::Nfd => s.chars().nfd().collect(),
        Form::Nfkc => s.chars().nfkc().collect(),
        Form::Nfkd => s.chars().nfkd().collect(),
    }
}

fn nf(form: Form, s: &str) -> String {
    normalize_form(s, form).into_owned()
}

proptest! {
    #![proptest_config(config())]

    /// 区切って必要なところだけ正規化しても、文字列全体を素直に正規化したのと同じ
    #[test]
    fn same_as_normalizing_the_whole_text(s in text(), form in form()) {
        prop_assert_eq!(nf(form, &s), plain(form, &s));
    }

    /// 二度かけても変わらない（冪等）
    #[test]
    fn idempotent(s in text(), form in form()) {
        let once = nf(form, &s);
        prop_assert_eq!(nf(form, &once), once.clone());
        prop_assert!(matches!(normalize_form(&once, form), Cow::Borrowed(_)));
    }

    /// 形どうしの関係（UAX #15）
    #[test]
    fn forms_relate(s in text()) {
        let nfc = nf(Form::Nfc, &s);
        let nfd = nf(Form::Nfd, &s);
        let nfkc = nf(Form::Nfkc, &s);
        let nfkd = nf(Form::Nfkd, &s);
        prop_assert_eq!(nf(Form::Nfc, &nfd), nfc.clone());
        prop_assert_eq!(nf(Form::Nfd, &nfc), nfd.clone());
        prop_assert_eq!(nf(Form::Nfkc, &nfkd), nfkc.clone());
        prop_assert_eq!(nf(Form::Nfkd, &nfkc), nfkd.clone());
        // 互換分解は正準分解を含む：NFKC・NFKD は先に NFC・NFD をかけても同じ
        prop_assert_eq!(nf(Form::Nfkc, &nfc), nfkc.clone());
        prop_assert_eq!(nf(Form::Nfkc, &nfd), nfkc.clone());
        prop_assert_eq!(nf(Form::Nfkd, &nfc), nfkd.clone());
        // NFKC の結果は NFC でもあり、NFKD の結果は NFD でもある
        prop_assert!(is_normalized(&nfkc, Form::Nfc));
        prop_assert!(is_normalized(&nfkd, Form::Nfd));
        prop_assert_eq!(nf(Form::Nfd, &nfkc), nfkd.clone());
        // NFKC で正規化済みなら NFC でも正規化済み
        if is_normalized(&s, Form::Nfkc) {
            prop_assert!(is_normalized(&s, Form::Nfc));
        }
        if is_normalized(&s, Form::Nfkd) {
            prop_assert!(is_normalized(&s, Form::Nfd));
        }
    }

    /// is_normalized は「正規化しても変わらない」ことと同じで、そのときだけ借用で返る
    #[test]
    fn is_normalized_agrees(s in text(), form in form()) {
        let out = normalize_form(&s, form);
        prop_assert_eq!(is_normalized(&s, form), out == s.as_str());
        prop_assert_eq!(matches!(out, Cow::Borrowed(_)), out == s.as_str());
        prop_assert_eq!(is_normalized(&s, form), plain(form, &s) == s);
    }

    /// 空白をまとめた結果：空白は U+0020 だけ、2 つ続かない、両端にない。空白以外の文字は順番どおり残る
    #[test]
    fn fold_whitespace_properties(s in text()) {
        let out = fold_whitespace(&s);
        prop_assert!(!out.contains("  "));
        prop_assert!(!out.starts_with(' ') && !out.ends_with(' '));
        prop_assert!(out.chars().all(|c| c == ' ' || !is_whitespace(c)));
        let words: Vec<&str> = s.split(is_whitespace).filter(|w| !w.is_empty()).collect();
        prop_assert_eq!(out.as_ref(), words.join(" "));
        prop_assert_eq!(fold_whitespace(&out), out.clone());
        prop_assert_eq!(matches!(out, Cow::Borrowed(_)), out == s.as_str());
    }

    /// 見えない文字の除去は、決めた集合をちょうど消し、ほかの文字はそのまま残す
    #[test]
    fn strip_invisible_properties(s in text()) {
        let out = strip_invisible(&s);
        let expected: String = s.chars().filter(|&c| !is_invisible(c)).collect();
        prop_assert_eq!(out.as_ref(), expected.as_str());
        prop_assert!(!out.chars().any(is_invisible));
        prop_assert_eq!(
            s.chars().count() - out.chars().count(),
            s.chars().filter(|&c| is_invisible(c)).count()
        );
        prop_assert_eq!(matches!(out, Cow::Borrowed(_)), out == s.as_str());
    }

    /// パイプライン全体：各段を順にかけたものと同じ。結果は正規化済み・見えない文字なし・空白がまとまっていて、もう一度かけても変わらない
    #[test]
    fn pipeline_properties(
        s in text(),
        form in prop::option::of(form()),
        fold in any::<bool>(),
        strip in any::<bool>(),
    ) {
        let options = Options { form, fold_whitespace: fold, strip_invisible: strip };
        let out = normalize(&s, options);
        let mut step = s.clone();
        if strip {
            step = strip_invisible(&step).into_owned();
        }
        if let Some(form) = form {
            step = plain(form, &step);
        }
        if fold {
            step = fold_whitespace(&step).into_owned();
        }
        prop_assert_eq!(out.as_ref(), step.as_str());
        if let Some(form) = form {
            prop_assert!(is_normalized(&out, form), "not {} after the pipeline: {:?}", form, out);
        }
        if strip {
            prop_assert!(!out.chars().any(is_invisible));
        }
        if fold {
            prop_assert_eq!(fold_whitespace(&out), out.clone());
        }
        prop_assert_eq!(normalize(&out, options), out.clone());
        prop_assert_eq!(matches!(out, Cow::Borrowed(_)), out == s.as_str());
    }

    /// 任意の UTF-8 文字列で panic しない
    #[test]
    fn never_panics(s in any::<String>(), form in prop::option::of(form()), fold in any::<bool>(), strip in any::<bool>()) {
        let _ = normalize(&s, Options { form, fold_whitespace: fold, strip_invisible: strip });
    }
}
