"""kura_rs.byte_entropy を scipy.stats.entropy と、ランダムな入力で突き合わせる。

比べ方: float が 1 ビットも違わないこと（== ではなく、ビット列で比べる。許容誤差 0）。
"""

import array
import os
import random
import struct
import zlib

import numpy as np
import pytest
import scipy.stats
from hypothesis import given
from hypothesis import strategies as st

from kura_rs import byte_entropy


def bits(x: float) -> bytes:
    return struct.pack(">d", x)


def reference(data) -> float:
    """部品の定義: scipy.stats.entropy(bincount, base=2)。空だけは nan ではなく 0.0。"""
    raw = bytes(data)
    if not raw:
        return 0.0
    return float(scipy.stats.entropy(np.bincount(np.frombuffer(raw, np.uint8), minlength=256), base=2))


def reference_windows(data: bytes, window: int, step: int) -> list[tuple[int, float]]:
    return [(off, reference(data[off:off + window])) for off in range(0, len(data) - window + 1, step)]


def same(got: float, want: float) -> bool:
    return bits(got) == bits(want)


# 偏ったバイト列（使う値が少ない）、ゼロの連続、テキスト、ランダム
skewed = st.builds(
    lambda values, n, seed: bytes(random.Random(seed).choices(values, k=n)),
    st.lists(st.integers(0, 255), min_size=1, max_size=20),
    st.integers(0, 5000),
    st.integers(),
)
data = st.one_of(
    st.binary(max_size=300),
    st.binary(max_size=10_000),
    skewed,
    st.builds(lambda b, n: bytes([b]) * n, st.integers(0, 255), st.integers(0, 20_000)),
    st.text(max_size=2000).map(str.encode),
    st.builds(lambda parts: b"".join(parts), st.lists(st.one_of(st.binary(max_size=500), st.integers(0, 3000).map(bytes)), max_size=10)),
)


# --- 全体のエントロピー ---

@given(data)
def test_entropy_matches_scipy_bit_for_bit(d):
    got = byte_entropy.entropy(d)
    assert same(got, reference(d)), (got, reference(d))


@given(data)
def test_bytearray_and_memoryview_match(d):
    want = reference(d)
    assert same(byte_entropy.entropy(bytearray(d)), want)
    assert same(byte_entropy.entropy(memoryview(d)), want)
    assert same(byte_entropy.entropy(memoryview(bytearray(d))), want)


@given(st.binary(max_size=3000), st.integers(0, 100), st.integers(0, 100))
def test_memoryview_slices_match(d, head, tail):
    view = memoryview(d)[head:max(head, len(d) - tail)]
    assert same(byte_entropy.entropy(view), reference(view.tobytes()))


@given(st.lists(st.integers(-(2**31), 2**31 - 1), max_size=500), st.sampled_from(["i", "l", "q", "h"]))
def test_array_array_counts_its_raw_bytes(values, typecode):
    if typecode == "h":
        values = [v % 2**15 for v in values]
    a = array.array(typecode, values)
    assert same(byte_entropy.entropy(a), reference(a.tobytes()))


@given(st.integers(0, 50), st.integers(1, 20), st.sampled_from([np.uint8, np.int16, np.float32, np.float64]))
def test_numpy_arrays_count_their_raw_bytes(rows, cols, dtype):
    arr = (np.arange(rows * cols) * 7919).astype(dtype).reshape(rows, cols)
    assert same(byte_entropy.entropy(arr), reference(arr.tobytes()))


@given(data)
def test_histogram_matches_bincount(d):
    got = byte_entropy.histogram(d)
    assert got == np.bincount(np.frombuffer(d, np.uint8), minlength=256).tolist()
    assert len(got) == 256 and sum(got) == len(d)


@given(st.lists(st.integers(0, 2**40), min_size=256, max_size=256))
def test_entropy_of_histogram_matches_scipy(counts):
    got = byte_entropy.entropy_of_histogram(counts)
    if sum(counts) == 0:
        assert got == 0.0
    else:
        assert same(got, float(scipy.stats.entropy(np.array(counts, dtype=np.int64), base=2)))


@given(st.lists(data, max_size=6))
def test_histograms_of_pieces_add_up(pieces):
    total = [0] * 256
    for p in pieces:
        total = [a + b for a, b in zip(total, byte_entropy.histogram(p))]
    whole = b"".join(pieces)
    assert total == byte_entropy.histogram(whole)
    assert same(byte_entropy.entropy_of_histogram(total), byte_entropy.entropy(whole))


@given(data)
def test_range(d):
    h = byte_entropy.entropy(d)
    assert 0.0 <= h <= 8.0 + 1e-12


# --- 窓 ---

windows_and_steps = st.tuples(
    st.one_of(st.integers(1, 10), st.integers(1, 600), st.sampled_from([255, 256, 257, 4096])),
    st.one_of(st.none(), st.integers(1, 10), st.integers(1, 700)),
)


@given(data, windows_and_steps)
def test_windows_match_scipy_per_window(d, ws):
    window, step = ws
    # SciPy を窓ごとに呼ぶと遅いので、窓が 200 を超えるときは刻みを広げる
    if len(d) >= window and (len(d) - window) // (step or window) >= 200:
        step = (len(d) - window) // 200 + 1
    got = byte_entropy.windows(d, window, step)
    want = reference_windows(d, window, step or window)
    assert [off for off, _ in got] == [off for off, _ in want]
    for (off, h), (_, r) in zip(got, want):
        assert same(h, r), (off, h, r)
    assert len(got) == byte_entropy.window_count(len(d), window, step)


@given(data, windows_and_steps)
def test_windows_accept_every_buffer_type(d, ws):
    window, step = ws
    want = byte_entropy.windows(d, window, step)
    assert byte_entropy.windows(bytearray(d), window, step) == want
    assert byte_entropy.windows(memoryview(d), window, step) == want


@given(st.integers(0, 100_000), st.integers(1, 5000), st.one_of(st.none(), st.integers(1, 5000)))
def test_window_count_formula(length, window, step):
    s = step or window
    expected = (length - window) // s + 1 if length >= window else 0
    assert byte_entropy.window_count(length, window, step) == expected


@given(data, st.integers(1, 300))
def test_each_window_equals_entropy_of_its_slice(d, window):
    for off, h in byte_entropy.windows(d, window, max(1, window // 3)):
        assert same(h, byte_entropy.entropy(d[off:off + window]))


# --- 決まった形（固定） ---

@pytest.mark.parametrize("k", range(9))
@pytest.mark.parametrize("reps", [1, 2, 3, 100])
def test_uniform_over_2_to_the_k_values(k, reps):
    d = bytes(i % (1 << k) for i in range((1 << k) * reps))
    assert byte_entropy.entropy(d) == float(k)
    assert same(byte_entropy.entropy(d), reference(d))


def test_empty_is_zero_while_scipy_says_nan():
    assert byte_entropy.entropy(b"") == 0.0
    assert byte_entropy.entropy(bytearray()) == 0.0
    assert byte_entropy.entropy_of_histogram([0] * 256) == 0.0
    with np.errstate(invalid="ignore"):
        assert np.isnan(scipy.stats.entropy(np.zeros(256), base=2))


@pytest.mark.parametrize("size", [1, 255, 256, 4095, 4096, 4097, 65536, 1 << 20])
def test_os_urandom(size):
    d = os.urandom(size)
    assert same(byte_entropy.entropy(d), reference(d))
    for window, step in [(256, 256), (4096, 4096), (4096, 1024)]:
        got = byte_entropy.windows(d, window, step)
        assert got == [(off, h) for off, h in reference_windows(d, window, step)]


def test_compressed_data_is_high_entropy():
    d = zlib.compress(b"".join(str(i).encode() for i in range(200_000)), 9)
    assert same(byte_entropy.entropy(d), reference(d))
    assert byte_entropy.entropy(d) > 7.0


def test_real_binaries():
    import numpy._core._multiarray_umath as umath

    d = open(umath.__file__, "rb").read()
    assert same(byte_entropy.entropy(d), reference(d))
    got = byte_entropy.windows(d, 4096, 4096 * 16)
    assert got == reference_windows(d, 4096, 4096 * 16)


def test_large_uniform_input():
    d = bytes(range(256)) * (64 << 10)  # 16 MiB、全 256 値が同じ回数
    assert byte_entropy.entropy(d) == 8.0
    assert all(h == 8.0 for _, h in byte_entropy.windows(d, 4096))
