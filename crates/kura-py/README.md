# kura-rs

Small, verified Rust parts for AI data processing and security, for Python.

Every part is checked against a reference implementation with differential tests and benchmarked across input sizes. See [the registry](https://github.com/h-kurashina/kura-rs) for the evidence.

```sh
pip install kura-rs
```

## minhash

Near-duplicate detection with MinHash, **bit-for-bit compatible with `datasketch.MinHash`** (default scheme of datasketch 2.x). Same `num_perm`, `seed` and input give the same signature values.

```python
from kura_rs import minhash

hasher = minhash.MinHasher(num_perm=128, seed=1)  # datasketch's defaults

a = hasher.signature("the quick brown fox jumps".split())
b = hasher.signature("the quick brown fox leaps".split())
print(a.jaccard(b))

# Many documents at once (runs without holding the GIL)
sigs = hasher.signatures([doc.split() for doc in documents])
```

Items may be `bytes`, `bytearray` or `str` (encoded as UTF-8, same as `datasketch` with `.encode("utf-8")`).

## file_hash

Streaming SHA-256 and BLAKE3 for file integrity checks and exact-duplicate detection. **Bit-for-bit the same as `hashlib.sha256` and the `blake3` package.** Files are read in 64 KiB blocks (constant memory), and the GIL is released while hashing.

```python
from kura_rs import file_hash

digest = file_hash.hash_file("blake3", "model.safetensors")
print(digest)                 # lowercase hex, same as hexdigest()

file_hash.hash_bytes("sha256", b"abc").hexdigest()   # == hashlib.sha256(b"abc").hexdigest()

h = file_hash.Hasher("sha256")  # like hashlib.sha256()
h.update(b"a")
h.update(b"bc")
h.hexdigest()

# Integrity check against a published checksum
assert digest == file_hash.Digest.from_hex("blake3", published_hex)

# Exact duplicates: digests are hashable
unique = set(file_hash.hash_files("sha256", paths))
```

Data may be `bytes`, `bytearray`, `memoryview` or any C-contiguous buffer (e.g. a NumPy array); `str` must be encoded first, as with `hashlib`. Paths may be `str` or `os.PathLike`. Algorithm names are `"sha256"` and `"blake3"` (any letter case).

## multi_pattern_match

Aho–Corasick search for many patterns in one pass, for scanning logs and payloads for indicators of compromise. **Every match is the same as `pyahocorasick`'s `Automaton.iter()`, in the same order.** The GIL is released while searching.

```python
from kura_rs import multi_pattern_match

matcher = multi_pattern_match.Matcher(["evil.example", "198.51.100.7", "/wp-admin/setup.php"])

# (pattern_id, start, end), overlapping ones included, ordered by end and then longest first
for pattern_id, start, end in matcher.find_all(log_line):
    print(matcher.patterns[pattern_id], start, end)

matcher.find_longest(payload)       # non-overlapping, leftmost-longest
matcher.is_match(payload)           # stops at the first match
matcher.count(payload)              # number of matches, without building a list
matcher.find_all(payload, limit=100)
```

Patterns must be all `str` (then search `str`; offsets are in characters, like `pyahocorasick`) or all bytes-like (then search `bytes`, `bytearray` or `memoryview`; offsets are in bytes). Empty patterns never match, and a duplicate pattern is reported under the id of its first occurrence.

## byte_entropy

Shannon entropy of bytes in bits per byte (0.0 to 8.0), for spotting packed, compressed or encrypted regions of a binary. **Bit-for-bit the same float as `scipy.stats.entropy(numpy.bincount(data, minlength=256), base=2)`**, except that empty data gives 0.0 (SciPy gives `nan`). The GIL is released while computing.

```python
from kura_rs import byte_entropy

byte_entropy.entropy(data)  # 0.0 (one byte value) ... 8.0 (all 256 values equally often)

# (offset, entropy) of every whole 4 KiB window; step defaults to the window size
for offset, h in byte_entropy.windows(data, 4096):
    if h > 7.2:
        print(f"{offset:#x}: likely packed or encrypted ({h:.2f} bits/byte)")

byte_entropy.windows(data, 4096, 1024)          # overlapping windows
byte_entropy.window_count(len(data), 4096)      # (len - window) // step + 1, or 0 if shorter
```

Only windows that fit entirely in the data are returned: a partial window at the end is skipped, and data shorter than the window gives `[]`. A window or step below 1 raises `ValueError`.

## unicode_normalize

Unicode cleanup for text pipelines. **Normalization is the same as `unicodedata.normalize` on Python 3.14 (Unicode 16.0.0)** and passes the official `NormalizationTest.txt`. Long strings are processed with the GIL released, and text that is already clean is returned as is.

```python
from kura_rs import unicode_normalize

# Remove invisible characters, NFKC, then fold whitespace (always in this order)
unicode_normalize.normalize("﻿ﾃﾞｰﾀ　　ＡＩ​ ", form="NFKC", fold_whitespace=True, strip_invisible=True)
# -> "データ AI"

unicode_normalize.normalize("é", "NFC")          # == unicodedata.normalize("NFC", "é")
unicode_normalize.is_normalized("é", "NFC")       # == unicodedata.is_normalized("NFC", "é")
unicode_normalize.normalize_many(texts, "NFKC", fold_whitespace=True)   # a list, one GIL release
```

- `fold_whitespace`: every run of `str.isspace()` characters becomes one space, and both ends are trimmed (same as `" ".join(text.split())`).
- `strip_invisible`: removes the characters in `unicode_normalize.INVISIBLE`, the format characters that are default-ignorable (ZWSP, ZWNJ, ZWJ, word joiner, BOM, soft hyphen, bidi controls, tag characters, ...). Variation selectors are kept. Removing ZWJ splits emoji ZWJ sequences.
- `form` is `"NFC"` (default), `"NFD"`, `"NFKC"`, `"NFKD"` or `None`. `unicode_normalize.UNICODE_VERSION` is the Unicode version (`"16.0.0"`); on Python 3.13 and older, `unicodedata` uses an older Unicode version and newly assigned characters can differ.
- Strings with lone surrogates raise `UnicodeEncodeError`.

## License

MIT OR Apache-2.0
