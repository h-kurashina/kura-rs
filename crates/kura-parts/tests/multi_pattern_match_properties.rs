//! multi_pattern_match の性質テスト。ランダムなパターンと入力を大量に作り、
//! テストの中に書いた素朴な O(n·m) の実装（全位置 × 全パターンを比べる）と完全に一致するかを見る。
//! 1つの性質につき既定で 2,000 通り。PROPTEST_CASES=100000 のように環境変数で増やせる。

use std::collections::BTreeSet;

use kura_parts::multi_pattern_match::{DFA_MAX_PATTERN_BYTES, Match, Matcher};
use proptest::prelude::*;

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

/// 大きな automaton を毎回作る性質は重いので、回数を 1/10 にする
fn large_config() -> ProptestConfig {
    let mut c = config();
    c.cases = (c.cases / 10).max(50);
    c
}

/// 空と重複を除いたパターン（id は最初に出てきた位置）
fn distinct(patterns: &[Vec<u8>]) -> Vec<(usize, &[u8])> {
    let mut seen = BTreeSet::new();
    patterns
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.is_empty() && seen.insert(p.as_slice()))
        .map(|(id, p)| (id, p.as_slice()))
        .collect()
}

/// 素朴な実装：終わりの位置ごとに、そこで終わるパターンを長い順に並べる
fn naive_overlapping(patterns: &[Vec<u8>], haystack: &[u8]) -> Vec<Match> {
    let mut by_length = distinct(patterns);
    by_length.sort_by_key(|(_, p)| std::cmp::Reverse(p.len()));
    let mut out = Vec::new();
    for end in 1..=haystack.len() {
        for &(id, p) in &by_length {
            if haystack[..end].ends_with(p) {
                out.push(Match {
                    pattern: id,
                    start: end - p.len(),
                    end,
                });
            }
        }
    }
    out
}

/// 素朴な実装：いちばん左で始まる一致のうち最も長いものを取り、その後ろから続ける
fn naive_longest(patterns: &[Vec<u8>], haystack: &[u8]) -> Vec<Match> {
    let patterns = distinct(patterns);
    let mut out = Vec::new();
    let mut start = 0;
    while start < haystack.len() {
        let best = patterns
            .iter()
            .filter(|(_, p)| haystack[start..].starts_with(p))
            .max_by_key(|(_, p)| p.len());
        match best {
            Some(&(id, p)) => {
                out.push(Match {
                    pattern: id,
                    start,
                    end: start + p.len(),
                });
                start += p.len();
            }
            None => start += 1,
        }
    }
    out
}

/// 一致が密になる小さなアルファベット（2〜4 文字）のバイト列
fn small_alphabet(max_len: usize) -> impl Strategy<Value = Vec<u8>> {
    (2u8..=4).prop_flat_map(move |k| prop::collection::vec(0..k, 0..max_len))
}

/// パターンの集合：小さなアルファベット・互いの接頭辞/接尾辞・重複・空を混ぜる
fn patterns() -> impl Strategy<Value = Vec<Vec<u8>>> {
    prop_oneof![
        3 => prop::collection::vec(small_alphabet(6), 0..20),
        1 => prop::collection::vec(prop::collection::vec(any::<u8>(), 0..5), 0..30),
        // 1本の文字列の接頭辞と接尾辞をすべて（互いに接頭辞・接尾辞の関係になる）
        1 => small_alphabet(12).prop_map(|s| {
            let mut out: Vec<Vec<u8>> = (0..=s.len()).map(|i| s[..i].to_vec()).collect();
            out.extend((0..=s.len()).map(|i| s[i..].to_vec()));
            out
        }),
        // 同じ文字の繰り返し（a, aa, aaa, ...）
        1 => (0u8..3, 1usize..12).prop_map(|(b, k)| (1..=k).map(|n| vec![b; n]).collect()),
    ]
}

fn haystack() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        3 => small_alphabet(200),
        1 => prop::collection::vec(any::<u8>(), 0..300),
        1 => (0u8..3, 0usize..300).prop_map(|(b, n)| vec![b; n]),
    ]
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn overlapping_equals_naive(patterns in patterns(), haystack in haystack()) {
        let matcher = Matcher::new(&patterns).unwrap();
        let expected = naive_overlapping(&patterns, &haystack);
        prop_assert_eq!(matcher.find_overlapping(&haystack), expected.clone());
        prop_assert_eq!(matcher.count_overlapping(&haystack), expected.len());
        prop_assert_eq!(matcher.is_match(&haystack), !expected.is_empty());
    }

    #[test]
    fn longest_equals_naive(patterns in patterns(), haystack in haystack()) {
        let matcher = Matcher::new(&patterns).unwrap();
        prop_assert_eq!(matcher.find_longest(&haystack), naive_longest(&patterns, &haystack));
    }

    #[test]
    fn independent_of_pattern_order(
        patterns in patterns(),
        haystack in haystack(),
        seed in any::<u64>(),
    ) {
        // パターンの並びを変えても、見つかる (位置, パターンの中身) は同じ。id だけが並びに合わせて変わる
        let mut shuffled = patterns.clone();
        let mut state = seed | 1;
        for i in (1..shuffled.len()).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            shuffled.swap(i, (state % (i as u64 + 1)) as usize);
        }
        let spans = |ps: &[Vec<u8>], matches: Vec<Match>| -> Vec<(usize, usize, Vec<u8>)> {
            matches.into_iter().map(|m| (m.start, m.end, ps[m.pattern].clone())).collect()
        };
        let a = Matcher::new(&patterns).unwrap();
        let b = Matcher::new(&shuffled).unwrap();
        // 重なりありは順番（end, 長い順）も並びに依存しない
        prop_assert_eq!(
            spans(&patterns, a.find_overlapping(&haystack)),
            spans(&shuffled, b.find_overlapping(&haystack))
        );
        prop_assert_eq!(
            spans(&patterns, a.find_longest(&haystack)),
            spans(&shuffled, b.find_longest(&haystack))
        );
    }

    #[test]
    fn shifts_with_a_prefix_that_cannot_match(
        patterns in prop::collection::vec(small_alphabet(6), 0..20),
        haystack in small_alphabet(200),
        pad in 0usize..50,
    ) {
        // どのパターンにも含まれないバイト（0xFF）を前に付けると、一致は同じだけずれる
        let matcher = Matcher::new(&patterns).unwrap();
        let mut padded = vec![0xFF; pad];
        padded.extend_from_slice(&haystack);
        let shifted: Vec<Match> = matcher
            .find_overlapping(&haystack)
            .into_iter()
            .map(|m| Match { start: m.start + pad, end: m.end + pad, ..m })
            .collect();
        prop_assert_eq!(matcher.find_overlapping(&padded), shifted);
    }

    #[test]
    fn longest_is_a_non_overlapping_subset(patterns in patterns(), haystack in haystack()) {
        let matcher = Matcher::new(&patterns).unwrap();
        let all: BTreeSet<Match> = matcher.find_overlapping(&haystack).into_iter().collect();
        let longest = matcher.find_longest(&haystack);
        for w in longest.windows(2) {
            prop_assert!(w[0].end <= w[1].start);
        }
        for m in &longest {
            prop_assert!(all.contains(m));
        }
        // 何か一致があれば、最左最長も少なくとも1つ見つける
        prop_assert_eq!(longest.is_empty(), all.is_empty());
    }

    #[test]
    fn duplicates_and_empties_do_not_change_results(
        patterns in patterns(),
        haystack in haystack(),
        extra in prop::collection::vec(any::<prop::sample::Index>(), 0..10),
    ) {
        // 既にあるパターン（と空パターン）を後ろに足しても、結果は変わらない（id は最初のもの）
        let mut more = patterns.clone();
        more.push(Vec::new());
        if !patterns.is_empty() {
            more.extend(extra.iter().map(|i| patterns[i.index(patterns.len())].clone()));
        }
        let a = Matcher::new(&patterns).unwrap();
        let b = Matcher::new(&more).unwrap();
        prop_assert_eq!(a.find_overlapping(&haystack), b.find_overlapping(&haystack));
        prop_assert_eq!(a.find_longest(&haystack), b.find_longest(&haystack));
    }
}

proptest! {
    // 大きな automaton（任意のバイトの DFA や 32 KiB を超える NFA）を毎回作るので重い。回数を 1/10 にする
    #![proptest_config(large_config())]

    #[test]
    fn arbitrary_bytes_never_panic(
        patterns in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..40), 0..60),
        haystack in prop::collection::vec(any::<u8>(), 0..2000),
    ) {
        let matcher = Matcher::new(&patterns).unwrap();
        prop_assert_eq!(matcher.pattern_count(), patterns.len());
        for m in matcher.find_overlapping(&haystack).iter().chain(&matcher.find_longest(&haystack)) {
            // 報告された位置には、報告されたパターンがそのままある
            prop_assert_eq!(&haystack[m.start..m.end], patterns[m.pattern].as_slice());
            prop_assert!(!m.is_empty());
        }
    }

    #[test]
    fn large_pattern_sets_equal_naive(patterns in patterns(), haystack in haystack()) {
        // 入力より長い（一致しえない）パターンを最後に足し、パターンの合計を DFA の上限より大きくする。
        // こうすると DFA ではなく NFA で探すが、結果は同じでなければならない
        let mut large = patterns.clone();
        large.push(vec![0xAB; DFA_MAX_PATTERN_BYTES + 1]);
        let matcher = Matcher::new(&large).unwrap();
        prop_assert_eq!(matcher.find_overlapping(&haystack), naive_overlapping(&patterns, &haystack));
        prop_assert_eq!(matcher.find_longest(&haystack), naive_longest(&patterns, &haystack));
    }
}
