//! パスの安全確認の性質テスト（proptest）。
//! 「受け入れたパスは、どんな入力でも書き込み先の外に出ない」ことを、ランダムな入力で確かめる。

use std::path::{Component, Path};

use kura_rs::manifest::{is_crate_name, is_feature_name, is_version_req};
use kura_rs::paths::{SafePath, install_layout, is_kebab_case};
use proptest::prelude::*;

/// パスを作るのに危ない文字を多めに混ぜた文字列。
fn nasty_path() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("/".to_owned()),
            Just("..".to_owned()),
            Just(".".to_owned()),
            Just("\\".to_owned()),
            Just(":".to_owned()),
            Just("~".to_owned()),
            Just("\0".to_owned()),
            Just("C:".to_owned()),
            Just("%2e".to_owned()),
            Just("nul".to_owned()),
            "[a-z]{1,4}",
            "\\PC{1,3}",
        ],
        0..10,
    )
    .prop_map(|parts| parts.concat())
}

/// 安全なはずの区切り 1 つ。
fn good_segment() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_][a-zA-Z0-9_.-]{0,15}[a-zA-Z0-9_]".prop_filter("not a Windows device name", |s| {
        let stem = s.split('.').next().unwrap().to_ascii_lowercase();
        !matches!(stem.as_str(), "con" | "prn" | "aux" | "nul")
            && !(stem.len() == 4
                && (stem.starts_with("com") || stem.starts_with("lpt"))
                && stem.as_bytes()[3].is_ascii_digit()
                && stem.as_bytes()[3] != b'0')
    })
}

fn assert_inside(path: &SafePath) {
    let root = Path::new("/project/src/parts/x");
    let joined = path.under(root);
    assert!(joined.starts_with(root), "{joined:?}");
    let rel = joined.strip_prefix(root).unwrap();
    assert!(
        rel.components().all(|c| matches!(c, Component::Normal(_))),
        "{rel:?}"
    );
    assert_eq!(rel.components().count(), path.segments().len());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES").ok().and_then(|s| s.parse().ok()).unwrap_or(2000)
    ))]

    /// どんな文字列でも、受け入れたなら書き込み先の中に収まる。
    #[test]
    fn accepted_arbitrary_strings_stay_inside(s in "\\PC{0,40}") {
        if let Ok(p) = SafePath::parse(&s) {
            assert_inside(&p);
            prop_assert!(!s.contains(".."));
            prop_assert!(!s.starts_with('/'));
        }
    }

    /// 危ない部品を組み合わせた文字列でも同じ。
    #[test]
    fn accepted_nasty_strings_stay_inside(s in nasty_path()) {
        if let Ok(p) = SafePath::parse(&s) {
            assert_inside(&p);
            prop_assert!(p.segments().iter().all(|seg| !seg.starts_with('.')));
        }
    }

    /// `..` の区切りが 1 つでもあれば拒む。
    #[test]
    fn parent_segment_is_always_rejected(
        before in prop::collection::vec(good_segment(), 0..4),
        after in prop::collection::vec(good_segment(), 0..4),
    ) {
        let mut segs = before;
        segs.push("..".into());
        segs.extend(after);
        prop_assert!(SafePath::parse(&segs.join("/")).is_err());
    }

    /// `/` で始まるものは拒む。
    #[test]
    fn absolute_paths_are_rejected(s in "\\PC{0,30}") {
        let absolute = format!("/{s}");
        prop_assert!(SafePath::parse(&absolute).is_err());
    }

    /// バックスラッシュを含むものは拒む。
    #[test]
    fn backslashes_are_rejected(a in "\\PC{0,15}", b in "\\PC{0,15}") {
        let with_backslash = format!("{a}\\{b}");
        prop_assert!(SafePath::parse(&with_backslash).is_err());
    }

    /// ASCII 以外の文字や制御文字を含むものは拒む。
    #[test]
    fn non_ascii_is_rejected(a in "[a-z]{0,5}", c in "[^\\x21-\\x7e]", b in "[a-z]{0,5}") {
        let with_odd_char = format!("{a}{c}{b}");
        prop_assert!(SafePath::parse(&with_odd_char).is_err());
    }

    /// 普通の相対パスは受け入れ、文字列に戻すと元どおりになる。
    #[test]
    fn good_paths_round_trip(segs in prop::collection::vec(good_segment(), 1..6)) {
        let s = segs.join("/");
        let p = SafePath::parse(&s).unwrap();
        prop_assert_eq!(p.as_slash_str(), s);
        assert_inside(&p);
    }

    /// 書き込み先の配置も、すべて書き込み先の中に収まり、行き先が重ならない。
    #[test]
    fn install_layout_stays_inside_and_is_unique(
        files in prop::collection::vec(prop::collection::vec(good_segment(), 1..5), 1..8)
    ) {
        let parsed: Vec<SafePath> = files
            .iter()
            .map(|segs| SafePath::parse(&segs.join("/")).unwrap())
            .collect();
        if let Ok(layout) = install_layout(&parsed) {
            prop_assert_eq!(layout.len(), parsed.len());
            let mut seen = std::collections::HashSet::new();
            for (src, rel) in parsed.iter().zip(&layout) {
                assert_inside(rel);
                // 元のパスの末尾と一致する（ファイル名は変えない）
                prop_assert!(src.as_slash_str().ends_with(&rel.as_slash_str()));
                prop_assert!(seen.insert(rel.as_slash_str().to_ascii_lowercase()));
            }
        }
    }

    /// kebab-case の名前は受け入れる。
    #[test]
    fn kebab_names_are_accepted(segs in prop::collection::vec("[a-z0-9]{1,8}", 1..5)) {
        let name = segs.join("-");
        prop_assume!(name.len() <= 64);
        prop_assert!(is_kebab_case(&name));
    }

    /// 受け入れた部品名は、そのままパスの 1 区切りとして安全に使える。
    #[test]
    fn accepted_names_are_safe_segments(s in "\\PC{0,20}") {
        if is_kebab_case(&s) {
            let path = format!("registry/{s}.json");
            let p = SafePath::parse(&path);
            prop_assert!(p.is_ok(), "{:?}", path);
        }
    }

    /// `-` で始まる値（cargo のオプションに見えるもの）は、クレート名・feature として受け入れない。
    #[test]
    fn option_like_values_are_rejected(s in "-\\PC{0,20}") {
        prop_assert!(!is_crate_name(&s));
        prop_assert!(!is_feature_name(&s));
    }

    /// バージョン指定は空白で始まらず、区切りに使える文字を含まない。
    #[test]
    fn accepted_version_reqs_are_plain(s in "\\PC{0,20}") {
        if is_version_req(&s) {
            prop_assert!(!s.starts_with(' ') && !s.ends_with(' '));
            prop_assert!(!s.contains(['\n', '\0', ';', '"', '\'', '@', '/', '\\']));
        }
    }
}
