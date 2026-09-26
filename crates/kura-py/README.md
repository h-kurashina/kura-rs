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

## License

MIT OR Apache-2.0
