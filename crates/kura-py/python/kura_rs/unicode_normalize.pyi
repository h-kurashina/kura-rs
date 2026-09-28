from collections.abc import Iterable
from typing import Final, Literal

# "NFC", "NFD", "NFKC" or "NFKD" (any letter case)
FormName = Literal["NFC", "NFD", "NFKC", "NFKD"] | str

FORMS: Final[tuple[str, str, str, str]]
# Unicode version of the normalization data, e.g. "16.0.0" (compare with unicodedata.unidata_version)
UNICODE_VERSION: Final[str]
# The 29 characters for which str.isspace() is true, in code point order
WHITESPACE: Final[str]
# Removed by strip_invisible: inclusive code point ranges (General_Category Cf and Default_Ignorable_Code_Point)
INVISIBLE: Final[tuple[tuple[int, int], ...]]

def normalize(
    text: str,
    form: FormName | None = "NFC",
    *,
    fold_whitespace: bool = False,
    strip_invisible: bool = False,
) -> str:
    """Remove invisible characters (if strip_invisible), normalize to form, then fold whitespace (if fold_whitespace).

    With only form, the same as unicodedata.normalize(form, text). Raises UnicodeEncodeError for lone surrogates.
    """

def normalize_many(
    texts: Iterable[str],
    form: FormName | None = "NFC",
    *,
    fold_whitespace: bool = False,
    strip_invisible: bool = False,
) -> list[str]:
    """normalize() for many strings, with the GIL released for the whole batch."""

def is_normalized(text: str, form: FormName = "NFC") -> bool:
    """Same as unicodedata.is_normalized(form, text) (note the argument order)."""

def fold_whitespace(text: str) -> str:
    """Same as " ".join(text.split())."""

def strip_invisible(text: str) -> str:
    """Remove every character in INVISIBLE."""
