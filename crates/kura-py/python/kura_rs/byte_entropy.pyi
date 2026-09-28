from collections.abc import Sequence
from typing import Final

# bytes, bytearray, memoryview or any other C-contiguous buffer (str must be encoded first)
Buffer = bytes | bytearray | memoryview

MAX_ENTROPY: Final[float]

def entropy(data: Buffer) -> float:
    """Entropy in bits per byte (0.0 to 8.0). Empty data gives 0.0."""

def histogram(data: Buffer) -> list[int]:
    """How often each byte value 0..255 occurs (256 ints)."""

def entropy_of_histogram(counts: Sequence[int]) -> float:
    """Entropy in bits of 256 non-negative counts. All zero gives 0.0."""

def windows(data: Buffer, window: int, step: int | None = None) -> list[tuple[int, float]]:
    """(offset, entropy) of each whole window at offsets 0, step, 2*step, ... (step defaults to window)."""

def window_count(length: int, window: int, step: int | None = None) -> int:
    """Number of windows `windows` returns for data of `length` bytes."""
