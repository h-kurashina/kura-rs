//! Shannon entropy of bytes, in bits per byte (0.0 to 8.0), over a whole
//! buffer or over fixed-size windows, for spotting packed, compressed or
//! encrypted regions of a binary.
//!
//! The result is bit-for-bit the same `f64` as
//!
//! ```python
//! counts = numpy.bincount(numpy.frombuffer(data, numpy.uint8), minlength=256)
//! scipy.stats.entropy(counts, base=2)
//! ```
//!
//! because it follows SciPy's steps in the same order: `p = count / total` for each
//! of the 256 byte values, `-p * ln(p)` per value (0 for an absent value), a sum in
//! NumPy's pairwise order, then a division by `ln(2)`. `ln` is the platform's C
//! library `log`, which is what SciPy uses too.
//!
//! ```ignore
//! use byte_entropy::{entropy, windows};
//!
//! assert_eq!(entropy(b""), 0.0);
//! assert_eq!(entropy(b"aaaa"), 0.0);
//! assert_eq!(entropy(b"abab"), 1.0);
//!
//! // 4 KiB windows, every 4 KiB. Only whole windows: a shorter tail is skipped.
//! for w in windows(&binary, 4096, 4096)? {
//!     if w.entropy > 7.2 {
//!         println!("offset {:#x}: likely packed or encrypted ({:.2} bits/byte)", w.offset, w.entropy);
//!     }
//! }
//! ```
//!
//! Edge cases:
//! - Empty input: [`entropy`] returns `0.0` (SciPy returns `nan`, from 0 / 0).
//! - One distinct byte value: `0.0`. All 256 values equally often: exactly `8.0`.
//! - Rounding: like SciPy, a nearly uniform input can come out a few units in the
//!   last place above 8.0 (at most [`MAX_ROUNDING_ABOVE_8`] above). The result is never negative.
//! - [`windows`] yields the windows starting at `0, step, 2 * step, ...` that fit
//!   entirely in the data. A partial window at the end is not included, so data
//!   shorter than `window` yields no windows ([`window_count`] gives the number).
//! - `window == 0` or `step == 0` is an error. `step > window` is allowed (gaps).
//!
//! No dependencies (std only).
//!
//! Part of kura-rs (https://github.com/h-kurashina/kura-rs). MIT OR Apache-2.0.

use std::fmt;

/// Entropy of data in which every byte value occurs equally often.
pub const MAX_ENTROPY: f64 = 8.0;

/// How far above 8.0 rounding can push a result for nearly uniform data.
/// (SciPy rounds the same way; the largest seen in testing is 8 + 4.4e-15.)
pub const MAX_ROUNDING_ABOVE_8: f64 = 1e-12;

/// Number of occurrences of each byte value.
pub type Histogram = [u64; 256];

/// One window of [`windows`]: where it starts and its entropy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    /// Byte offset of the window's first byte.
    pub offset: usize,
    /// Entropy of `data[offset..offset + window]`, in bits per byte.
    pub entropy: f64,
}

/// Error for a window or step of zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowError {
    ZeroWindow,
    ZeroStep,
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            WindowError::ZeroWindow => "window must be at least 1 byte",
            WindowError::ZeroStep => "step must be at least 1 byte",
        })
    }
}

impl std::error::Error for WindowError {}

/// Entropy of `data` in bits per byte: 0.0 (one value, or empty) to 8.0 (uniform).
pub fn entropy(data: &[u8]) -> f64 {
    entropy_of_histogram(&histogram(data))
}

/// Counts how often each byte value occurs.
pub fn histogram(data: &[u8]) -> Histogram {
    // 4 つの表に振り分けて数える（同じ値が続いても、1つの数え先への書き込み待ちにならない）
    let mut tables = [[0u64; 256]; 4];
    let (quads, rest) = data.as_chunks::<4>();
    for &[a, b, c, d] in quads {
        tables[0][usize::from(a)] += 1;
        tables[1][usize::from(b)] += 1;
        tables[2][usize::from(c)] += 1;
        tables[3][usize::from(d)] += 1;
    }
    for &b in rest {
        tables[0][usize::from(b)] += 1;
    }
    let mut counts = [0u64; 256];
    for (i, count) in counts.iter_mut().enumerate() {
        *count = tables[0][i] + tables[1][i] + tables[2][i] + tables[3][i];
    }
    counts
}

/// Entropy in bits of the distribution given by `counts` (normalized by their sum),
/// the same as `scipy.stats.entropy(counts, base=2)`. All zero: `0.0`.
///
/// Useful for data that arrives in pieces: add up the histograms of the pieces.
pub fn entropy_of_histogram(counts: &Histogram) -> f64 {
    // u128 で足すので、どの数え上げでもあふれない（SciPy は f64 で足す。合計が 2^53 未満なら同じ値）
    let total: u128 = counts.iter().map(|&c| u128::from(c)).sum();
    if total == 0 {
        return 0.0;
    }
    let total = total as f64;
    let mut terms = [0.0f64; 256];
    for (term, &count) in terms.iter_mut().zip(counts) {
        *term = entr(count as f64 / total);
    }
    finish(&terms)
}

/// SciPy の `special.entr`：x > 0 なら `-x * log(x)`、0 なら 0。
fn entr(p: f64) -> f64 {
    if p > 0.0 { -p * p.ln() } else { 0.0 }
}

/// 256 個の項を NumPy と同じ順番で足し、ln(2) で割る（scipy.stats.entropy の `S /= math.log(base)`）。
fn finish(terms: &[f64; 256]) -> f64 {
    pairwise_sum(terms) / std::f64::consts::LN_2
}

/// NumPy の `sum` と同じ順番の足し算（numpy/_core/src/umath/loops_utils.h.src の pairwise_sum）。
/// 浮動小数点の足し算は順番で結果が変わるので、SciPy と1ビットも違わないようにこの順番を守る。
fn pairwise_sum(a: &[f64]) -> f64 {
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for &x in a {
            res += x;
        }
        res
    } else if n <= 128 {
        let mut r = [0.0f64; 8];
        r.copy_from_slice(&a[..8]);
        let mut i = 8;
        while i < n - n % 8 {
            for (j, acc) in r.iter_mut().enumerate() {
                *acc += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        for &x in &a[i..] {
            res += x;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        pairwise_sum(&a[..n2]) + pairwise_sum(&a[n2..])
    }
}

/// Number of windows [`windows`] yields: `(len - window) / step + 1` when
/// `len >= window`, otherwise 0.
pub fn window_count(len: usize, window: usize, step: usize) -> Result<usize, WindowError> {
    check(window, step)?;
    Ok(if len >= window {
        (len - window) / step + 1
    } else {
        0
    })
}

fn check(window: usize, step: usize) -> Result<(), WindowError> {
    if window == 0 {
        Err(WindowError::ZeroWindow)
    } else if step == 0 {
        Err(WindowError::ZeroStep)
    } else {
        Ok(())
    }
}

/// Entropy of each `window`-byte window, starting at offsets `0, step, 2 * step, ...`.
/// Only windows that fit entirely in `data` are yielded (no partial window at the end).
///
/// Each window's entropy equals [`entropy`] of that slice, bit for bit.
/// Overlapping windows (`step < window`) reuse the previous window's counts.
pub fn windows(data: &[u8], window: usize, step: usize) -> Result<Windows<'_>, WindowError> {
    let remaining = window_count(data.len(), window, step)?;
    // 窓の大きさが決まっているので、-p ln p は「その値が何回出たか」だけで決まる。
    // 窓が多いときは 0..=window 回の答えを先に表にしておき、窓ごとの log の計算を省く
    let table = (window <= TABLE_MAX_WINDOW && remaining.saturating_mul(256) > window).then(|| {
        let total = window as f64;
        (0..=window).map(|c| entr(c as f64 / total)).collect()
    });
    Ok(Windows {
        data,
        window,
        step,
        next: 0,
        remaining,
        counts: [0; 256],
        has_counts: false,
        table,
    })
}

/// これより大きい窓では表を作らない（表は window + 1 個の f64）
const TABLE_MAX_WINDOW: usize = 1 << 20;

/// Iterator returned by [`windows`].
#[derive(Debug, Clone)]
pub struct Windows<'a> {
    data: &'a [u8],
    window: usize,
    step: usize,
    next: usize,
    remaining: usize,
    /// 直前の窓の数え上げ（窓が重なるときに使い回す）
    counts: Histogram,
    has_counts: bool,
    table: Option<Vec<f64>>,
}

impl Windows<'_> {
    fn entropy_of_counts(&self) -> f64 {
        match &self.table {
            Some(table) => {
                let mut terms = [0.0f64; 256];
                for (term, &count) in terms.iter_mut().zip(&self.counts) {
                    *term = table[count as usize];
                }
                finish(&terms)
            }
            None => entropy_of_histogram(&self.counts),
        }
    }
}

impl Iterator for Windows<'_> {
    type Item = Window;

    fn next(&mut self) -> Option<Window> {
        if self.remaining == 0 {
            return None;
        }
        let offset = self.next;
        // 重なりが大きい（出る分と入る分の合計が窓より小さい）ときは、前の窓の数え上げをずらす
        if self.has_counts && self.step.saturating_mul(2) < self.window {
            let prev = offset - self.step;
            for &b in &self.data[prev..offset] {
                self.counts[usize::from(b)] -= 1;
            }
            for &b in &self.data[prev + self.window..offset + self.window] {
                self.counts[usize::from(b)] += 1;
            }
        } else {
            self.counts = histogram(&self.data[offset..offset + self.window]);
            self.has_counts = true;
        }
        let entropy = self.entropy_of_counts();
        self.remaining -= 1;
        if self.remaining > 0 {
            self.next = offset + self.step;
        }
        Some(Window { offset, entropy })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }

    fn nth(&mut self, n: usize) -> Option<Window> {
        if n > 0 {
            if n >= self.remaining {
                self.remaining = 0;
                return None;
            }
            // 飛ばした窓は数えない（次の窓は数え直す）
            self.next += n * self.step;
            self.remaining -= n;
            self.has_counts = false;
        }
        self.next()
    }
}

impl ExactSizeIterator for Windows<'_> {}

impl std::iter::FusedIterator for Windows<'_> {}
