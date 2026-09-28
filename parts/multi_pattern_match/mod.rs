//! Aho–Corasick search for many byte patterns in one pass, for scanning logs
//! and payloads for indicators of compromise (domains, IPs, hashes, paths).
//!
//! Semantics (checked against `pyahocorasick`'s `Automaton.iter()`, byte for byte):
//!
//! - Patterns and haystacks are **bytes**. Text is searched as its UTF-8 bytes
//!   (`&str` works directly); offsets are byte offsets.
//! - A pattern's id is its index in the list given to [`Matcher::new`].
//! - **Empty patterns never match** (`pyahocorasick` ignores them too).
//! - **Duplicate patterns** report the id of their *first* occurrence only,
//!   so every `(start, end)` span is reported at most once per distinct pattern.
//! - [`Matcher::find_overlapping`] reports **every** occurrence of every pattern,
//!   including overlapping ones and patterns that are prefixes or suffixes of
//!   each other, ordered by `end` ascending and, for the same `end`, longest
//!   first (`start` ascending). This is exactly the order of `Automaton.iter()`.
//! - [`Matcher::find_longest`] reports non-overlapping matches, scanning left to
//!   right and taking the longest pattern that starts at the leftmost possible
//!   position (`MatchKind::LeftmostLongest`). This is what `Automaton.iter_long()`
//!   is meant to return, but `pyahocorasick` 2.3.1 misses matches in some cases
//!   (patterns `b`, `abb` over `ab` return nothing), so it is checked against the
//!   leftmost-longest selection from `Automaton.iter()` instead.
//!
//! Automaton choice: when the distinct patterns total at most
//! [`DFA_MAX_PATTERN_BYTES`] (about 1,500 typical IOC strings), a full DFA is
//! built: about twice as fast to search, at roughly 200 bytes of memory per
//! pattern byte (≈ 4 MB for 1,000 IOCs). Larger sets use a compact NFA
//! (≈ 15 bytes per pattern byte). Results are identical either way.
//!
//! The haystack may be untrusted: search time is linear in the haystack length
//! plus the number of matches reported. Note that overlapping output can be
//! large for adversarial inputs (patterns `a`, `aa`, …, `a`×k over `aaaa…`
//! give k matches per byte). Use [`Matcher::find_overlapping_iter`] and stop
//! early, or [`Matcher::count_overlapping`] / [`Matcher::is_match`], when the
//! haystack is hostile and only a verdict is needed.
//!
//! ```ignore
//! use multi_pattern_match::Matcher;
//!
//! let iocs = ["evil.example", "198.51.100.7", "/wp-admin/setup.php"];
//! let matcher = Matcher::new(&iocs)?;
//!
//! for m in matcher.find_overlapping_iter("GET /wp-admin/setup.php from 198.51.100.7") {
//!     println!("{} at {}..{}", iocs[m.pattern], m.start, m.end);
//! }
//! assert!(matcher.is_match(b"dns query evil.example"));
//! ```
//!
//! Dependencies: `aho-corasick = "1.1"`.
//!
//! Part of kura-rs (https://github.com/h-kurashina/kura-rs). MIT OR Apache-2.0.

use std::collections::HashSet;
use std::fmt;
use std::sync::OnceLock;

use aho_corasick::{AhoCorasick, AhoCorasickKind, MatchKind};

/// Up to this many bytes of distinct patterns, the matcher builds a DFA
/// (faster search, more memory); above it, a compact NFA.
pub const DFA_MAX_PATTERN_BYTES: usize = 32 * 1024;

/// One occurrence of a pattern: `haystack[start..end] == patterns[pattern]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Match {
    /// Index of the pattern in the list given to [`Matcher::new`]
    /// (the first one, if the same pattern was given more than once).
    pub pattern: usize,
    /// Byte offset of the first byte of the match.
    pub start: usize,
    /// Byte offset just past the last byte of the match (exclusive).
    pub end: usize,
}

impl Match {
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Always `false`: empty patterns never match.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// Error when the automaton cannot be built (only for pattern sets far beyond
/// what fits in memory, e.g. billions of bytes of patterns).
#[derive(Debug, Clone)]
pub struct BuildError(String);

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for BuildError {}

/// A compiled set of patterns. Build once, search many haystacks; it is
/// `Send + Sync`, so one matcher can be shared between threads.
pub struct Matcher {
    /// 重なりも含めてすべて報告する用（MatchKind::Standard）。空と重複を除いたパターンだけで作る
    overlapping: AhoCorasick,
    /// 最左最長・重なりなし用。使われるまで作らない
    longest: OnceLock<AhoCorasick>,
    /// 空と重複を除いたパターン（最左最長の automaton を後から作るために持っておく）
    unique: Vec<Box<[u8]>>,
    /// unique の番号 → 呼び出し側が渡したリストでの番号（最初に出てきた位置）
    ids: Vec<usize>,
    /// 呼び出し側が渡したパターンの数（空・重複も数える）
    pattern_count: usize,
}

impl Matcher {
    /// Compiles the patterns. Empty patterns are accepted but never match;
    /// duplicates are reported under the id of their first occurrence.
    pub fn new<I, P>(patterns: I) -> Result<Self, BuildError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<[u8]>,
    {
        let mut unique: Vec<Box<[u8]>> = Vec::new();
        let mut ids = Vec::new();
        let mut seen: HashSet<Box<[u8]>> = HashSet::new();
        let mut pattern_count = 0;
        for (id, pattern) in patterns.into_iter().enumerate() {
            pattern_count += 1;
            let pattern = pattern.as_ref();
            // 空パターンはどこにでも一致してしまうので登録しない（pyahocorasick も無視する）
            if pattern.is_empty() || !seen.insert(pattern.into()) {
                continue;
            }
            unique.push(pattern.into());
            ids.push(id);
        }
        let overlapping = build(MatchKind::Standard, &unique)?;
        Ok(Self {
            overlapping,
            longest: OnceLock::new(),
            unique,
            ids,
            pattern_count,
        })
    }

    /// Number of patterns given to [`Matcher::new`], including empty and duplicate ones.
    pub fn pattern_count(&self) -> usize {
        self.pattern_count
    }

    /// Number of distinct non-empty patterns (the ones that can match).
    pub fn distinct_pattern_count(&self) -> usize {
        self.unique.len()
    }

    /// Heap memory used by the automata, in bytes.
    pub fn memory_usage(&self) -> usize {
        self.overlapping.memory_usage() + self.longest.get().map_or(0, AhoCorasick::memory_usage)
    }

    /// Every occurrence of every pattern, overlapping ones included, ordered by
    /// `end` and then longest first. Same as `pyahocorasick`'s `Automaton.iter()`.
    pub fn find_overlapping_iter<'a, 'h, H>(&'a self, haystack: &'h H) -> OverlappingIter<'a, 'h>
    where
        H: AsRef<[u8]> + ?Sized,
    {
        OverlappingIter {
            inner: self.overlapping.find_overlapping_iter(haystack.as_ref()),
            ids: &self.ids,
        }
    }

    /// [`Matcher::find_overlapping_iter`], collected.
    pub fn find_overlapping<H: AsRef<[u8]> + ?Sized>(&self, haystack: &H) -> Vec<Match> {
        self.find_overlapping_iter(haystack).collect()
    }

    /// Number of overlapping matches, without storing them.
    pub fn count_overlapping<H: AsRef<[u8]> + ?Sized>(&self, haystack: &H) -> usize {
        self.overlapping
            .find_overlapping_iter(haystack.as_ref())
            .count()
    }

    /// Non-overlapping matches, leftmost first and, among matches starting at
    /// the same position, the longest (what `pyahocorasick`'s `Automaton.iter_long()`
    /// intends; see the module docs for where it differs).
    pub fn find_longest_iter<'a, 'h, H>(&'a self, haystack: &'h H) -> LongestIter<'a, 'h>
    where
        H: AsRef<[u8]> + ?Sized,
    {
        let automaton = self.longest.get_or_init(|| {
            // 同じパターンで Standard が作れているので、ここで失敗することはない
            build(MatchKind::LeftmostLongest, &self.unique)
                .expect("automaton was already built once from the same patterns")
        });
        LongestIter {
            inner: automaton.find_iter(haystack.as_ref()),
            ids: &self.ids,
        }
    }

    /// [`Matcher::find_longest_iter`], collected.
    pub fn find_longest<H: AsRef<[u8]> + ?Sized>(&self, haystack: &H) -> Vec<Match> {
        self.find_longest_iter(haystack).collect()
    }

    /// Whether any pattern occurs in the haystack. Stops at the first match.
    pub fn is_match<H: AsRef<[u8]> + ?Sized>(&self, haystack: &H) -> bool {
        self.overlapping.is_match(haystack.as_ref())
    }
}

impl fmt::Debug for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Matcher")
            .field("pattern_count", &self.pattern_count)
            .field("distinct_pattern_count", &self.unique.len())
            .finish_non_exhaustive()
    }
}

fn build(kind: MatchKind, patterns: &[Box<[u8]>]) -> Result<AhoCorasick, BuildError> {
    // 小さなパターン集合は DFA（速い・メモリを食う）、大きな集合は連続 NFA。
    // aho-corasick の自動選択はパターンが 100 個以下だと長さに関係なく DFA にするので、ここで決める
    let total: usize = patterns.iter().map(|p| p.len()).sum();
    let automaton = if total <= DFA_MAX_PATTERN_BYTES {
        AhoCorasickKind::DFA
    } else {
        AhoCorasickKind::ContiguousNFA
    };
    AhoCorasick::builder()
        .match_kind(kind)
        .kind(Some(automaton))
        .build(patterns)
        .map_err(|e| BuildError(e.to_string()))
}

fn convert(m: aho_corasick::Match, ids: &[usize]) -> Match {
    Match {
        pattern: ids[m.pattern().as_usize()],
        start: m.start(),
        end: m.end(),
    }
}

/// Iterator returned by [`Matcher::find_overlapping_iter`].
pub struct OverlappingIter<'a, 'h> {
    inner: aho_corasick::FindOverlappingIter<'a, 'h>,
    ids: &'a [usize],
}

impl Iterator for OverlappingIter<'_, '_> {
    type Item = Match;

    fn next(&mut self) -> Option<Match> {
        self.inner.next().map(|m| convert(m, self.ids))
    }
}

impl fmt::Debug for OverlappingIter<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OverlappingIter").finish_non_exhaustive()
    }
}

/// Iterator returned by [`Matcher::find_longest_iter`].
pub struct LongestIter<'a, 'h> {
    inner: aho_corasick::FindIter<'a, 'h>,
    ids: &'a [usize],
}

impl Iterator for LongestIter<'_, '_> {
    type Item = Match;

    fn next(&mut self) -> Option<Match> {
        self.inner.next().map(|m| convert(m, self.ids))
    }
}

impl fmt::Debug for LongestIter<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LongestIter").finish_non_exhaustive()
    }
}
