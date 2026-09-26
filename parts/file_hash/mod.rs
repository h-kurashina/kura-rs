//! Streaming SHA-256 and BLAKE3 hashing, for file integrity checks and
//! exact-duplicate detection in datasets.
//!
//! Output is bit-for-bit the same as Python's `hashlib.sha256(data).digest()`
//! and the `blake3` package's `blake3(data).digest()` (32 bytes). How the input
//! is split into chunks never changes the digest.
//!
//! ```ignore
//! use file_hash::{hash_bytes, hash_file, Algorithm, Digest, Hasher};
//!
//! // A whole file, read in 64 KiB blocks (memory use does not grow with file size)
//! let digest = hash_file(Algorithm::Blake3, "model.safetensors")?;
//! println!("{digest}"); // lowercase hex
//!
//! // Bytes already in memory
//! let digest = hash_bytes(Algorithm::Sha256, b"abc");
//! assert_eq!(digest.to_hex(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
//!
//! // Pieces arriving one at a time (network, decompression, ...)
//! let mut hasher = Hasher::new(Algorithm::Sha256);
//! hasher.update(b"a").update(b"bc");
//! assert_eq!(hasher.finalize(), digest);
//!
//! // Integrity check against a published checksum
//! let expected = Digest::from_hex(Algorithm::Sha256, "ba7816bf...")?;
//! ```
//!
//! Dependencies: `sha2 = "0.11"` (default features are not needed) and `blake3 = "1.8"`.
//!
//! Part of kura-rs (https://github.com/h-kurashina/kura-rs). MIT OR Apache-2.0.

use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;

use sha2::Digest as _;

/// Size of the read buffer used by [`hash_reader`] and [`hash_file`].
/// Large enough for BLAKE3 to use SIMD across several 1 KiB chunks at once.
pub const READ_BUFFER_SIZE: usize = 64 * 1024;

/// Length of every digest in bytes (both algorithms produce 256 bits).
pub const DIGEST_LEN: usize = 32;

/// The hash function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithm {
    /// SHA-256 (FIPS 180-4). Same as `hashlib.sha256`.
    Sha256,
    /// BLAKE3 with the default 32-byte output. Same as the `blake3` Python package.
    Blake3,
}

impl Algorithm {
    /// Every supported algorithm.
    pub const ALL: [Algorithm; 2] = [Algorithm::Sha256, Algorithm::Blake3];

    /// Lowercase name, the same as `hashlib` uses: `"sha256"` or `"blake3"`.
    pub fn name(self) -> &'static str {
        match self {
            Algorithm::Sha256 => "sha256",
            Algorithm::Blake3 => "blake3",
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Error for an unknown algorithm name or a malformed hex digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

impl FromStr for Algorithm {
    type Err = ParseError;

    /// Accepts `sha256`, `sha-256` and `blake3`, in any letter case.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "sha256" | "sha-256" => Ok(Algorithm::Sha256),
            "blake3" => Ok(Algorithm::Blake3),
            _ => Err(ParseError(format!(
                "unknown algorithm {s:?} (expected \"sha256\" or \"blake3\")"
            ))),
        }
    }
}

/// A 32-byte digest, tagged with the algorithm that produced it.
///
/// Equality compares the algorithm and all 32 bytes without stopping at the
/// first difference (constant time), so comparing against a secret expected
/// value does not leak how many leading bytes matched.
#[derive(Clone, Copy)]
pub struct Digest {
    algorithm: Algorithm,
    bytes: [u8; DIGEST_LEN],
}

impl Digest {
    /// Wraps raw digest bytes (e.g. read from a database).
    pub fn new(algorithm: Algorithm, bytes: [u8; DIGEST_LEN]) -> Self {
        Self { algorithm, bytes }
    }

    /// Parses 64 hex digits (upper or lower case), e.g. a published checksum.
    pub fn from_hex(algorithm: Algorithm, hex: &str) -> Result<Self, ParseError> {
        let hex = hex.as_bytes();
        if hex.len() != DIGEST_LEN * 2 {
            return Err(ParseError(format!(
                "expected {} hex digits, got {}",
                DIGEST_LEN * 2,
                hex.len()
            )));
        }
        let mut bytes = [0u8; DIGEST_LEN];
        for (byte, &[hi, lo]) in bytes.iter_mut().zip(hex.as_chunks::<2>().0) {
            match (hex_value(hi), hex_value(lo)) {
                (Some(hi), Some(lo)) => *byte = hi << 4 | lo,
                _ => return Err(ParseError("digest contains a non-hex character".into())),
            }
        }
        Ok(Self { algorithm, bytes })
    }

    pub fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    /// The raw 32 bytes, same as Python's `.digest()`.
    pub fn as_bytes(&self) -> &[u8; DIGEST_LEN] {
        &self.bytes
    }

    /// Lowercase hex (64 characters), same as Python's `.hexdigest()`.
    pub fn to_hex(&self) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(DIGEST_LEN * 2);
        for &b in &self.bytes {
            out.push(DIGITS[usize::from(b >> 4)] as char);
            out.push(DIGITS[usize::from(b & 0x0f)] as char);
        }
        out
    }
}

fn hex_value(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl PartialEq for Digest {
    fn eq(&self, other: &Self) -> bool {
        // すべてのバイトを見てから判定する（早く抜けると、何バイト目まで合っていたかが時間で漏れる）
        let diff = self
            .bytes
            .iter()
            .zip(&other.bytes)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b));
        std::hint::black_box(diff) == 0 && self.algorithm == other.algorithm
    }
}

impl Eq for Digest {}

impl std::hash::Hash for Digest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.algorithm.hash(state);
        self.bytes.hash(state);
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({}:{})", self.algorithm, self.to_hex())
    }
}

impl AsRef<[u8]> for Digest {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

/// An incremental hasher: feed the input in pieces of any size with
/// [`Hasher::update`], then read the digest with [`Hasher::finalize`].
#[derive(Clone)]
pub struct Hasher {
    state: State,
}

#[derive(Clone)]
enum State {
    Sha256(sha2::Sha256),
    // blake3::Hasher は約 2 KB あるので、enum 全体が大きくならないよう Box に入れる
    Blake3(Box<blake3::Hasher>),
}

impl Hasher {
    pub fn new(algorithm: Algorithm) -> Self {
        let state = match algorithm {
            Algorithm::Sha256 => State::Sha256(sha2::Sha256::new()),
            Algorithm::Blake3 => State::Blake3(Box::new(blake3::Hasher::new())),
        };
        Self { state }
    }

    pub fn algorithm(&self) -> Algorithm {
        match self.state {
            State::Sha256(_) => Algorithm::Sha256,
            State::Blake3(_) => Algorithm::Blake3,
        }
    }

    /// Appends `data` to the input. Returns `self` so calls can be chained.
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        match &mut self.state {
            State::Sha256(h) => h.update(data),
            State::Blake3(h) => {
                h.update(data);
            }
        }
        self
    }

    /// Reads `reader` to the end and appends everything to the input.
    /// Returns the number of bytes read.
    pub fn update_reader(&mut self, mut reader: impl Read) -> io::Result<u64> {
        let mut buffer = vec![0u8; READ_BUFFER_SIZE];
        let mut total = 0u64;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => return Ok(total),
                Ok(n) => {
                    self.update(&buffer[..n]);
                    total += n as u64;
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }

    /// The digest of everything fed so far. The hasher is left unchanged,
    /// so more input can still be added afterwards.
    pub fn finalize(&self) -> Digest {
        let bytes: [u8; DIGEST_LEN] = match &self.state {
            State::Sha256(h) => h.clone().finalize().into(),
            State::Blake3(h) => *h.finalize().as_bytes(),
        };
        Digest::new(self.algorithm(), bytes)
    }

    /// Forgets all input, as if newly created with the same algorithm.
    pub fn reset(&mut self) {
        *self = Self::new(self.algorithm());
    }
}

impl fmt::Debug for Hasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Hasher")
            .field("algorithm", &self.algorithm())
            .finish_non_exhaustive()
    }
}

/// Lets a hasher be the target of [`std::io::copy`] and `write!`.
impl io::Write for Hasher {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The digest of bytes already in memory.
pub fn hash_bytes(algorithm: Algorithm, data: &[u8]) -> Digest {
    Hasher::new(algorithm).update(data).finalize()
}

/// The digest of everything `reader` yields, read in [`READ_BUFFER_SIZE`] blocks.
/// Reads interrupted by a signal are retried; any other read error is returned.
pub fn hash_reader(algorithm: Algorithm, reader: impl Read) -> io::Result<Digest> {
    let mut hasher = Hasher::new(algorithm);
    hasher.update_reader(reader)?;
    Ok(hasher.finalize())
}

/// The digest of a file's contents, streamed so memory use stays constant.
pub fn hash_file(algorithm: Algorithm, path: impl AsRef<Path>) -> io::Result<Digest> {
    hash_reader(algorithm, File::open(path)?)
}
