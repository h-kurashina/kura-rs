//! multi_pattern_match の fuzz ターゲット。libFuzzer が作る任意のパターンと入力で、
//! 落ちない（panic しない）ことと、素朴な O(n·m) の実装と完全に一致することを確かめる。
//!
//!   cargo +nightly fuzz run multi_pattern_match -- -max_total_time=600

#![no_main]

use std::collections::BTreeSet;

use kura_parts::multi_pattern_match::{Match, Matcher};
use libfuzzer_sys::fuzz_target;

/// 素朴な実装がすぐ終わるように大きさを抑える
const MAX_PATTERNS: usize = 64;
const MAX_HAYSTACK: usize = 4096;

fuzz_target!(|input: (Vec<Vec<u8>>, Vec<u8>)| {
    let (mut patterns, mut haystack) = input;
    patterns.truncate(MAX_PATTERNS);
    haystack.truncate(MAX_HAYSTACK);

    let matcher = Matcher::new(&patterns).expect("small pattern sets always build");
    let overlapping = matcher.find_overlapping(&haystack);
    assert_eq!(overlapping, naive_overlapping(&patterns, &haystack));
    assert_eq!(matcher.count_overlapping(&haystack), overlapping.len());
    assert_eq!(matcher.is_match(&haystack), !overlapping.is_empty());
    assert_eq!(
        matcher.find_longest(&haystack),
        naive_longest(&patterns, &haystack)
    );
});

fn distinct(patterns: &[Vec<u8>]) -> Vec<(usize, &[u8])> {
    let mut seen = BTreeSet::new();
    patterns
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.is_empty() && seen.insert(p.as_slice()))
        .map(|(id, p)| (id, p.as_slice()))
        .collect()
}

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
