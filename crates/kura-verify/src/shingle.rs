//! 実文書を MinHash に入れる前のシングル化（Rust 側）。
//!
//! Python 側（verify/minhash/shingle.py）と同じ規則。規則の説明もそちらにある。
//! 両者が同じシングル列を作ることは、次の2つで確かめる。
//! - このファイルのテスト: verify/minhash/shingle_golden.json（Python が作った期待値）と突き合わせる
//! - verify/run.py minhash: コーパスの全文書について、シングル列の件数と SHA-1 を突き合わせる

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::Deserialize;
use sha1::{Digest, Sha1};

/// Unicode の White_Space 属性を持つ文字（char::is_whitespace と同じ集合。テストで確かめる）。
pub const WHITESPACE: &str = "\u{0009}\u{000a}\u{000b}\u{000c}\u{000d}\u{0020}\u{0085}\u{00a0}\u{1680}\
\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\
\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}";

/// 句読点・記号。Python 側の PUNCTUATION と同じ並び。
pub const PUNCTUATION: &str = concat!(
    "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~",
    "\u{00a1}\u{00a7}\u{00ab}\u{00b6}\u{00b7}\u{00bb}\u{00bf}",
    "\u{2010}\u{2011}\u{2012}\u{2013}\u{2014}\u{2015}",
    "\u{2018}\u{2019}\u{201a}\u{201b}\u{201c}\u{201d}\u{201e}\u{201f}",
    "\u{2020}\u{2021}\u{2022}\u{2026}\u{2030}\u{2032}\u{2033}\u{2039}\u{203a}\u{203b}",
    "\u{3001}\u{3002}\u{3003}\u{3008}\u{3009}\u{300a}\u{300b}\u{300c}\u{300d}\u{300e}\u{300f}",
    "\u{3010}\u{3011}\u{3014}\u{3015}\u{3016}\u{3017}\u{301c}\u{301d}\u{301e}\u{301f}\u{30fb}",
    "\u{ff01}\u{ff02}\u{ff03}\u{ff05}\u{ff06}\u{ff07}\u{ff08}\u{ff09}\u{ff0a}\u{ff0c}\u{ff0d}\u{ff0e}\u{ff0f}",
    "\u{ff1a}\u{ff1b}\u{ff1f}\u{ff20}\u{ff3b}\u{ff3c}\u{ff3d}\u{ff3f}\u{ff5b}\u{ff5d}\u{ff5e}\u{ff61}\u{ff64}",
);

/// シングルの作り方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// 単語 3-shingle（英語など、空白で単語を区切る言語）
    Word,
    /// 文字 3-gram（日本語・中国語）
    Char,
}

/// 区切り文字の一覧（小さい順・重複なし）。
pub fn separators() -> &'static [char] {
    static SEPARATORS: OnceLock<Vec<char>> = OnceLock::new();
    SEPARATORS.get_or_init(|| {
        let mut all: Vec<char> = WHITESPACE.chars().chain(PUNCTUATION.chars()).collect();
        all.sort_unstable();
        all.dedup();
        all
    })
}

pub fn is_separator(c: char) -> bool {
    separators().binary_search(&c).is_ok()
}

/// 小文字にしてから、区切り文字で切った単語の列。
pub fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(is_separator)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

fn grams(units: &[String], joiner: &str) -> Vec<String> {
    match units.len() {
        0 => Vec::new(),
        1 | 2 => vec![units.join(joiner)],
        _ => units.windows(3).map(|w| w.join(joiner)).collect(),
    }
}

/// シングル列（最初に出た順・重複なし）。
pub fn shingles(text: &str, mode: Mode) -> Vec<String> {
    let all = match mode {
        Mode::Word => grams(&words(text), " "),
        Mode::Char => {
            let chars: Vec<String> = text
                .to_lowercase()
                .chars()
                .filter(|&c| !is_separator(c))
                .map(String::from)
                .collect();
            grams(&chars, "")
        }
    };
    let mut seen = HashSet::with_capacity(all.len());
    all.into_iter().filter(|g| seen.insert(g.clone())).collect()
}

/// シングル列の指紋。各シングルの UTF-8 の後ろに `\n` を付けてつないだものの SHA-1（16進）。
pub fn digest(shingles: &[String]) -> String {
    let mut h = Sha1::new();
    for g in shingles {
        h.update(g.as_bytes());
        h.update(b"\n");
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct Golden {
        separators: Vec<u32>,
        cases: Vec<GoldenCase>,
    }

    #[derive(Deserialize)]
    struct GoldenCase {
        mode: Mode,
        text: String,
        shingles: Vec<String>,
        digest: String,
    }

    fn golden() -> Golden {
        serde_json::from_str(include_str!("../../../verify/minhash/shingle_golden.json"))
            .expect("shingle_golden.json")
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn separators_match_python() {
        let rust: Vec<u32> = separators().iter().map(|&c| c as u32).collect();
        assert_eq!(rust, golden().separators);
    }

    #[test]
    fn golden_cases_match_python() {
        let golden = golden();
        assert!(golden.cases.len() >= 90);
        for case in &golden.cases {
            let got = shingles(&case.text, case.mode);
            assert_eq!(
                got, case.shingles,
                "{:?} {:?}: shingles differ from Python",
                case.mode, case.text
            );
            assert_eq!(digest(&got), case.digest, "{:?} {:?}", case.mode, case.text);
        }
    }

    #[test]
    fn whitespace_is_exactly_unicode_white_space() {
        let listed: HashSet<char> = WHITESPACE.chars().collect();
        assert_eq!(listed.len(), WHITESPACE.chars().count(), "no duplicates");
        for c in (0..=0x10ffff).filter_map(char::from_u32) {
            assert_eq!(listed.contains(&c), c.is_whitespace(), "U+{:04X}", c as u32);
        }
    }

    #[test]
    fn punctuation_has_no_duplicates_and_no_whitespace() {
        let listed: HashSet<char> = PUNCTUATION.chars().collect();
        assert_eq!(listed.len(), PUNCTUATION.chars().count());
        assert!(PUNCTUATION.chars().all(|c| !c.is_whitespace()));
        assert!(PUNCTUATION.chars().all(|c| !c.is_alphanumeric()));
    }

    #[test]
    fn all_ascii_punctuation_is_a_separator() {
        for c in (0u8..128).map(char::from) {
            assert_eq!(
                is_separator(c),
                c.is_ascii_punctuation() || c.is_whitespace(),
                "{c:?}"
            );
        }
    }

    #[test]
    fn letters_digits_and_ideographs_are_not_separators() {
        for c in "azAZ09éßΣσжあア漢한ｱＡ０🦀\u{200b}\u{0301}".chars() {
            assert!(!is_separator(c), "{c:?}");
        }
    }

    #[test]
    fn word_shingles() {
        assert_eq!(
            shingles("The quick brown fox", Mode::Word),
            s(&["the quick brown", "quick brown fox"])
        );
        assert_eq!(shingles("", Mode::Word), Vec::<String>::new());
        assert_eq!(shingles("  ,.  ", Mode::Word), Vec::<String>::new());
        assert_eq!(shingles("one", Mode::Word), s(&["one"]));
        assert_eq!(shingles("one two", Mode::Word), s(&["one two"]));
        assert_eq!(
            shingles("one, two; three", Mode::Word),
            s(&["one two three"])
        );
    }

    #[test]
    fn char_shingles() {
        assert_eq!(
            shingles("日本語の文", Mode::Char),
            s(&["日本語", "本語の", "語の文"])
        );
        assert_eq!(shingles("日本", Mode::Char), s(&["日本"]));
        assert_eq!(shingles("、。", Mode::Char), Vec::<String>::new());
        // 句読点と空白は取り除いてからつなぐ
        assert_eq!(
            shingles("東京、 大阪。", Mode::Char),
            s(&["東京大", "京大阪"])
        );
    }

    #[test]
    fn duplicates_are_removed_in_first_seen_order() {
        assert_eq!(
            shingles("a b c a b c a b c", Mode::Word),
            s(&["a b c", "b c a", "c a b"])
        );
        assert_eq!(shingles("ああああああ", Mode::Char), s(&["あああ"]));
    }

    #[test]
    fn lowercase_uses_full_unicode_rules() {
        // 語末のシグマは ς になる（Python の str.lower() と同じ）
        assert_eq!(words("ΟΔΟΣ ΣΑΣ"), s(&["οδος", "σας"]));
        // İ は2文字（i + 結合ドット）になる
        assert_eq!(words("İ"), s(&["i\u{307}"]));
        assert_eq!(words("STRASSE Straße"), s(&["strasse", "straße"]));
    }

    #[test]
    fn case_only_edits_give_the_same_shingles() {
        let a = shingles("The Quick Brown Fox Jumps", Mode::Word);
        let b = shingles("THE QUICK BROWN FOX JUMPS", Mode::Word);
        assert_eq!(a, b);
        assert_eq!(digest(&a), digest(&b));
    }

    #[test]
    fn digest_of_nothing_is_sha1_of_empty_input() {
        assert_eq!(digest(&[]), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        // "abc\n" の SHA-1
        assert_eq!(
            digest(&s(&["abc"])),
            "03cfd743661f07975fa2f1220c5194cbaff48451"
        );
    }

    #[test]
    fn digest_depends_on_order_and_boundaries() {
        assert_ne!(digest(&s(&["ab", "c"])), digest(&s(&["a", "bc"])));
        assert_ne!(digest(&s(&["a", "b"])), digest(&s(&["b", "a"])));
    }

    #[test]
    fn mode_is_read_from_lowercase_json() {
        assert_eq!(
            serde_json::from_str::<Mode>("\"word\"").unwrap(),
            Mode::Word
        );
        assert_eq!(
            serde_json::from_str::<Mode>("\"char\"").unwrap(),
            Mode::Char
        );
        assert!(serde_json::from_str::<Mode>("\"Word\"").is_err());
    }
}
