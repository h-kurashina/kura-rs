"""Shannon entropy of bytes in bits per byte (0.0 to 8.0), bit-for-bit the same as scipy.stats.entropy.

    from kura_rs import byte_entropy

    byte_entropy.entropy(data)  # == scipy.stats.entropy(numpy.bincount(..., minlength=256), base=2)
    for offset, h in byte_entropy.windows(data, 4096):
        if h > 7.2:
            print(f"{offset:#x}: likely packed or encrypted")
"""

from kura_rs._native import byte_entropy as _native

MAX_ENTROPY = _native.MAX_ENTROPY
entropy = _native.entropy
entropy_of_histogram = _native.entropy_of_histogram
histogram = _native.histogram
window_count = _native.window_count
windows = _native.windows

__all__ = ["MAX_ENTROPY", "entropy", "entropy_of_histogram", "histogram", "window_count", "windows"]
