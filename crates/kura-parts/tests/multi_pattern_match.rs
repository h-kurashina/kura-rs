//! multi_pattern_match の例によるテスト。期待値は pyahocorasick 2.3.1 の Automaton.iter() で確かめたもの
//! （iter() の (末尾の位置, 値) は、ここでは (end - 1, pattern) にあたる）。

use kura_parts::multi_pattern_match::{Match, Matcher};

fn m(pattern: usize, start: usize, end: usize) -> Match {
    Match {
        pattern,
        start,
        end,
    }
}

/// pyahocorasick の iter() と同じ形 (end_index, id) に直す
fn as_pyahocorasick(matches: &[Match]) -> Vec<(usize, usize)> {
    matches.iter().map(|m| (m.end - 1, m.pattern)).collect()
}

#[test]
fn he_she_his_hers() {
    let matcher = Matcher::new(["he", "she", "his", "hers"]).unwrap();
    assert_eq!(
        matcher.find_overlapping("ushers"),
        vec![m(1, 1, 4), m(0, 2, 4), m(3, 2, 6)]
    );
    assert_eq!(matcher.find_longest("ushers"), vec![m(1, 1, 4)]);
    assert_eq!(matcher.count_overlapping("ushers"), 3);
    assert!(matcher.is_match("ushers"));
    assert!(!matcher.is_match("user"));
}

#[test]
fn same_order_as_pyahocorasick_iter() {
    // pyahocorasick: [(0,0),(1,1),(1,0),(2,2),(2,1),(2,0),(3,2),(3,1),(3,0),(4,4),(4,3),(5,0),(6,5),(6,4),(6,3)]
    let matcher = Matcher::new(["a", "aa", "aaa", "b", "ab", "bab"]).unwrap();
    let got = as_pyahocorasick(&matcher.find_overlapping("aaaabab"));
    assert_eq!(
        got,
        vec![
            (0, 0),
            (1, 1),
            (1, 0),
            (2, 2),
            (2, 1),
            (2, 0),
            (3, 2),
            (3, 1),
            (3, 0),
            (4, 4),
            (4, 3),
            (5, 0),
            (6, 5),
            (6, 4),
            (6, 3)
        ]
    );
    // iter_long: [(2,2),(4,4),(6,4)]
    let got = as_pyahocorasick(&matcher.find_longest("aaaabab"));
    assert_eq!(got, vec![(2, 2), (4, 4), (6, 4)]);
}

#[test]
fn longest_does_not_miss_matches_like_iter_long() {
    // pyahocorasick 2.3.1 の iter_long() はこれらで一致を取りこぼす（[] や [(2, 1)] を返す）
    let matcher = Matcher::new(["b", "abb"]).unwrap();
    assert_eq!(matcher.find_longest("ab"), vec![m(0, 1, 2)]);
    assert_eq!(matcher.find_longest("abbab"), vec![m(1, 0, 3), m(0, 4, 5)]);
    let matcher = Matcher::new(["baa", "a"]).unwrap();
    assert_eq!(matcher.find_longest("ba"), vec![m(1, 1, 2)]);
}

#[test]
fn empty_patterns_never_match() {
    let matcher = Matcher::new(["", "b", ""]).unwrap();
    assert_eq!(matcher.find_overlapping("abc"), vec![m(1, 1, 2)]);
    assert_eq!(matcher.pattern_count(), 3);
    assert_eq!(matcher.distinct_pattern_count(), 1);

    let only_empty = Matcher::new([""]).unwrap();
    assert!(only_empty.find_overlapping("abc").is_empty());
    assert!(only_empty.find_longest("abc").is_empty());
    assert!(!only_empty.is_match("abc"));
}

#[test]
fn no_patterns_and_empty_haystack() {
    let none = Matcher::new(Vec::<Vec<u8>>::new()).unwrap();
    assert_eq!(none.pattern_count(), 0);
    assert!(none.find_overlapping("abc").is_empty());
    assert!(none.find_longest("abc").is_empty());

    let matcher = Matcher::new(["a"]).unwrap();
    assert!(matcher.find_overlapping("").is_empty());
    assert!(matcher.find_longest(b"").is_empty());
    assert_eq!(matcher.count_overlapping(""), 0);
    assert!(!matcher.is_match(""));
}

#[test]
fn duplicates_report_the_first_id() {
    let matcher = Matcher::new(["x", "y", "x", "x"]).unwrap();
    assert_eq!(
        matcher.find_overlapping("xyx"),
        vec![m(0, 0, 1), m(1, 1, 2), m(0, 2, 3)]
    );
    assert_eq!(matcher.find_longest("xyx"), matcher.find_overlapping("xyx"));
    assert_eq!(matcher.distinct_pattern_count(), 2);
}

#[test]
fn prefixes_and_suffixes_of_each_other() {
    // 互いに接頭辞（abc, ab, a）・接尾辞（abc, bc, c）
    let matcher = Matcher::new(["abc", "ab", "a", "bc", "c"]).unwrap();
    assert_eq!(
        matcher.find_overlapping("abc"),
        vec![m(2, 0, 1), m(1, 0, 2), m(0, 0, 3), m(3, 1, 3), m(4, 2, 3)]
    );
    assert_eq!(matcher.find_longest("abcabc"), vec![m(0, 0, 3), m(0, 3, 6)]);
}

#[test]
fn prefix_chain_over_repetitive_haystack() {
    // a, aa, ..., a×k を a×n から探すと、一致は位置ごとに min(end, k) 個
    let k = 50;
    let n = 1000;
    let patterns: Vec<Vec<u8>> = (1..=k).map(|len| vec![b'a'; len]).collect();
    let matcher = Matcher::new(&patterns).unwrap();
    let haystack = vec![b'a'; n];
    let expected: usize = (1..=n).map(|end| end.min(k)).sum();
    assert_eq!(matcher.count_overlapping(&haystack), expected);
    let all = matcher.find_overlapping(&haystack);
    assert_eq!(all.len(), expected);
    for w in all.windows(2) {
        assert!((w[0].end, w[0].start) < (w[1].end, w[1].start));
    }
    // 最左最長は a×k を重ならずに並べ、残りを 1 つ
    let longest = matcher.find_longest(&haystack);
    assert_eq!(longest.len(), n / k + usize::from(!n.is_multiple_of(k)));
    assert!(longest[..n / k].iter().all(|m| m.len() == k));
}

#[test]
fn bytes_and_utf8() {
    let matcher = Matcher::new([
        &b"\x00\xff"[..],
        b"\xff",
        "東京".as_bytes(),
        "😀".as_bytes(),
    ])
    .unwrap();
    assert_eq!(
        matcher.find_overlapping(b"a\x00\xff\x00\xff"),
        vec![m(0, 1, 3), m(1, 2, 3), m(0, 3, 5), m(1, 4, 5)]
    );
    // UTF-8 の文字列は、そのバイト列として探す（位置はバイト単位）
    assert_eq!(
        matcher.find_overlapping("ログ😀東京"),
        vec![m(3, 6, 10), m(2, 10, 16)]
    );
}

#[test]
fn very_long_pattern() {
    // 繰り返しのない 20,000 バイト（xorshift）
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let long: Vec<u8> = (0..20_000)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 56) as u8
        })
        .collect();
    let mut haystack = vec![b'x'; 1000];
    haystack.extend_from_slice(&long);
    haystack.extend_from_slice(b"tail");
    let matcher = Matcher::new([&long[..], &long[..10], b"tail"]).unwrap();
    assert_eq!(
        matcher.find_overlapping(&haystack),
        vec![m(1, 1000, 1010), m(0, 1000, 21_000), m(2, 21_000, 21_004)]
    );
    assert_eq!(
        matcher.find_longest(&haystack),
        vec![m(0, 1000, 21_000), m(2, 21_000, 21_004)]
    );
}

#[test]
fn thousands_of_ioc_patterns() {
    let patterns: Vec<String> = (0..5000).map(|i| format!("ioc-{i:04}.example")).collect();
    let matcher = Matcher::new(&patterns).unwrap();
    let log = "GET http://ioc-4999.example/ from 10.0.0.1; dns ioc-0042.example\n";
    let hits: Vec<&str> = matcher
        .find_overlapping(log)
        .iter()
        .map(|m| patterns[m.pattern].as_str())
        .collect();
    assert_eq!(hits, ["ioc-4999.example", "ioc-0042.example"]);
    assert!(matcher.memory_usage() > 0);
}

#[test]
fn iterator_can_stop_early() {
    let matcher = Matcher::new(["a", "aa"]).unwrap();
    let haystack = vec![b'a'; 1 << 20];
    let first: Vec<Match> = matcher.find_overlapping_iter(&haystack).take(3).collect();
    assert_eq!(first, vec![m(0, 0, 1), m(1, 0, 2), m(0, 1, 2)]);
    assert_eq!(
        matcher.find_longest_iter(&haystack).next(),
        Some(m(1, 0, 2))
    );
}

#[test]
fn shared_between_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Matcher>();

    let matcher = Matcher::new(["needle", "need"]).unwrap();
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                // 最左最長の automaton を最初に使うときに作るので、同時に呼んでも壊れないことも見る
                assert_eq!(matcher.find_longest("a needle"), vec![m(0, 2, 8)]);
                assert_eq!(matcher.count_overlapping("a needle"), 2);
            });
        }
    });
}

#[test]
fn match_helpers() {
    let hit = m(0, 3, 7);
    assert_eq!(hit.len(), 4);
    assert!(!hit.is_empty());
    let matcher = Matcher::new(["x"]).unwrap();
    assert!(format!("{matcher:?}").contains("pattern_count: 1"));
}
