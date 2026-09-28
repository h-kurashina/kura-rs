"""Unicode cleanup for text pipelines: NFC/NFD/NFKC/NFKD, whitespace folding and invisible-character removal.

Normalization is the same as unicodedata.normalize on Python 3.14 (Unicode 16.0.0; see UNICODE_VERSION).

    from kura_rs import unicode_normalize

    clean = unicode_normalize.normalize(raw, form="NFKC", fold_whitespace=True, strip_invisible=True)
"""

from kura_rs._native import unicode_normalize as _native

FORMS = _native.FORMS
UNICODE_VERSION = _native.UNICODE_VERSION
WHITESPACE = _native.WHITESPACE
INVISIBLE = _native.INVISIBLE
normalize = _native.normalize
normalize_many = _native.normalize_many
is_normalized = _native.is_normalized
fold_whitespace = _native.fold_whitespace
strip_invisible = _native.strip_invisible

__all__ = [
    "FORMS",
    "INVISIBLE",
    "UNICODE_VERSION",
    "WHITESPACE",
    "fold_whitespace",
    "is_normalized",
    "normalize",
    "normalize_many",
    "strip_invisible",
]
