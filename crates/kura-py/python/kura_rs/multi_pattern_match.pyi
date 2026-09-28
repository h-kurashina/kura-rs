from collections.abc import Iterable
from typing import Literal

# bytes, bytearray, memoryview or any other buffer
Buffer = bytes | bytearray | memoryview
# (pattern_id, start, end): haystack[start:end] == patterns[pattern_id]; end is exclusive
Hit = tuple[int, int, int]

class Matcher:
    """Many patterns compiled into one Aho–Corasick automaton. Build once, search many haystacks.

    Patterns must be all `str` (then search `str`; offsets are in characters, like pyahocorasick)
    or all bytes-like (then search bytes-like; offsets are in bytes). Empty patterns never match;
    a duplicate pattern is reported under the id of its first occurrence.
    """

    def __init__(self, patterns: Iterable[str] | Iterable[Buffer]) -> None: ...
    def find_all(self, haystack: str | Buffer, *, limit: int | None = None) -> list[Hit]:
        """Every occurrence, overlapping ones included, ordered by end and then longest first
        (the order of pyahocorasick's `Automaton.iter()`). Stops after `limit` matches if given."""
    def find_longest(self, haystack: str | Buffer) -> list[Hit]:
        """Non-overlapping matches, leftmost first and, at the same start, the longest."""
    def count(self, haystack: str | Buffer) -> int:
        """`len(find_all(haystack))` without building the list."""
    def is_match(self, haystack: str | Buffer) -> bool:
        """Whether any pattern occurs. Stops at the first match."""
    @property
    def patterns(self) -> tuple[str, ...] | tuple[bytes, ...]:
        """The patterns as given (bytes-like ones as `bytes`), indexed by pattern id."""
    @property
    def kind(self) -> Literal["str", "bytes", "empty"]: ...
    @property
    def memory_usage(self) -> int:
        """Heap memory used by the automaton, in bytes."""
    def __len__(self) -> int: ...
