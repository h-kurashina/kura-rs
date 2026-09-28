"""kura-rs: small, verified Rust parts for AI data processing and security.

    from kura_rs import minhash
    from kura_rs import file_hash
    from kura_rs import byte_entropy
    from kura_rs import multi_pattern_match
    from kura_rs import unicode_normalize
"""

from kura_rs import byte_entropy, file_hash, minhash, multi_pattern_match, unicode_normalize
from kura_rs._native import __version__

__all__ = ["__version__", "byte_entropy", "file_hash", "minhash", "multi_pattern_match", "unicode_normalize"]
