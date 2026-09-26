//! MinHash signatures for near-duplicate detection.
//!
//! Compatible with the default scheme of Python's `datasketch` 2.x (`affine32`):
//! with the same `num_perm`, `seed` and input bytes, [`Signature::values`] is
//! bit-for-bit identical to `datasketch.MinHash(...).hashvalues`.
//!
//! ```ignore
//! let hasher = MinHasher::new(128, 1);
//! let a = hasher.signature("the quick brown fox jumps".split_whitespace());
//! let b = hasher.signature("the quick brown fox leaps".split_whitespace());
//! println!("estimated jaccard = {:.3}", a.jaccard(&b));
//! ```
//!
//! Part of kura-rs (https://github.com/h-kurashina/kura-rs). MIT OR Apache-2.0.

mod permutation;

use sha1::{Digest, Sha1};

/// The permutation functions. Build once and reuse for every document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinHasher {
    a: Vec<u32>,
    b: Vec<u32>,
}

/// A MinHash signature: the minimum permuted hash value per permutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    values: Vec<u32>,
}

impl MinHasher {
    /// `num_perm` permutations drawn from `seed`, exactly as
    /// `datasketch.MinHash(num_perm=num_perm, seed=seed)` does (datasketch's default seed is 1).
    ///
    /// # Panics
    /// If `num_perm` is zero.
    pub fn new(num_perm: usize, seed: u32) -> Self {
        assert!(num_perm > 0, "num_perm must be positive");
        let (a, b) = permutation::affine32(num_perm, seed);
        Self { a, b }
    }

    pub fn num_perm(&self) -> usize {
        self.a.len()
    }

    /// The permutation parameters `(a, b)`, same as `datasketch.MinHash(...).permutations`.
    pub fn permutations(&self) -> (&[u32], &[u32]) {
        (&self.a, &self.b)
    }

    /// An empty signature (every value at its maximum), to fill with [`Signature::update`].
    pub fn empty(&self) -> Signature {
        Signature {
            values: vec![u32::MAX; self.num_perm()],
        }
    }

    /// The signature of a set of items (tokens, shingles, ...). Items are hashed as raw bytes.
    pub fn signature<I>(&self, items: I) -> Signature
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        let mut signature = self.empty();
        for item in items {
            signature.update(self, item.as_ref());
        }
        signature
    }
}

impl Signature {
    /// Adds one item. `hasher` must be the one that created this signature.
    pub fn update(&mut self, hasher: &MinHasher, item: &[u8]) {
        debug_assert_eq!(
            self.values.len(),
            hasher.num_perm(),
            "signature from another MinHasher"
        );
        let h = fmix32(sha1_hash32(item));
        for ((value, &a), &b) in self.values.iter_mut().zip(&hasher.a).zip(&hasher.b) {
            *value = (*value).min(a.wrapping_mul(h).wrapping_add(b));
        }
    }

    /// Estimated Jaccard similarity: the share of permutations whose minimums agree.
    ///
    /// # Panics
    /// If the signatures have different lengths.
    pub fn jaccard(&self, other: &Signature) -> f64 {
        assert_eq!(
            self.values.len(),
            other.values.len(),
            "signatures must have the same num_perm"
        );
        let equal = self
            .values
            .iter()
            .zip(&other.values)
            .filter(|(x, y)| x == y)
            .count();
        equal as f64 / self.values.len() as f64
    }

    /// Merges `other` into this signature, giving the signature of the union of both sets.
    pub fn merge(&mut self, other: &Signature) {
        assert_eq!(
            self.values.len(),
            other.values.len(),
            "signatures must have the same num_perm"
        );
        for (value, &o) in self.values.iter_mut().zip(&other.values) {
            *value = (*value).min(o);
        }
    }

    pub fn values(&self) -> &[u32] {
        &self.values
    }
}

/// datasketch's `sha1_hash32`: the first 4 bytes of SHA-1, little-endian.
fn sha1_hash32(item: &[u8]) -> u32 {
    let digest = Sha1::digest(item);
    u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]])
}

/// The MurmurHash3 32-bit finalizer, applied once before the permutations.
fn fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^ (h >> 16)
}
