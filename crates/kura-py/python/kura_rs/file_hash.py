"""Streaming SHA-256 and BLAKE3, bit-for-bit the same as hashlib.sha256 and the blake3 package.

    from kura_rs import file_hash

    digest = file_hash.hash_file("blake3", "model.safetensors")
    print(digest)  # lowercase hex
"""

from kura_rs._native import file_hash as _native

ALGORITHMS = _native.ALGORITHMS
DIGEST_SIZE = _native.DIGEST_SIZE
Digest = _native.Digest
Hasher = _native.Hasher
hash_bytes = _native.hash_bytes
hash_file = _native.hash_file
hash_files = _native.hash_files

__all__ = ["ALGORITHMS", "DIGEST_SIZE", "Digest", "Hasher", "hash_bytes", "hash_file", "hash_files"]
