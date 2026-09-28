"""kura_rs.byte_entropy の使い方：誤った値はエラーになり、落ちないこと。GIL を手放すこと。"""

import concurrent.futures
import os
import threading
import time

import pytest
from hypothesis import given
from hypothesis import strategies as st

from kura_rs import byte_entropy

# --- 引数の検査 ---


def test_rejects_str_like_hashlib():
    with pytest.raises(TypeError, match="Strings must be encoded"):
        byte_entropy.entropy("abc")
    with pytest.raises(TypeError, match="Strings must be encoded"):
        byte_entropy.windows("abc", 1)
    with pytest.raises(TypeError, match="Strings must be encoded"):
        byte_entropy.histogram("abc")


@pytest.mark.parametrize("bad", [None, 1, 1.5, object(), [1, 2], (b"a",), {"a": 1}])
def test_rejects_non_buffers(bad):
    with pytest.raises(TypeError, match="bytes-like object is required"):
        byte_entropy.entropy(bad)
    with pytest.raises(TypeError, match="bytes-like object is required"):
        byte_entropy.windows(bad, 4)


def test_rejects_non_contiguous_buffers():
    view = memoryview(bytes(range(10)))[::2]
    with pytest.raises(BufferError):
        byte_entropy.entropy(view)
    with pytest.raises(BufferError):
        byte_entropy.windows(view, 2)


@pytest.mark.parametrize("window", [0, -1, -4096])
def test_rejects_a_window_below_one(window):
    with pytest.raises(ValueError, match="window must be at least 1 byte"):
        byte_entropy.windows(b"abc", window)
    with pytest.raises(ValueError, match="window must be at least 1 byte"):
        byte_entropy.window_count(3, window)


@pytest.mark.parametrize("step", [0, -1])
def test_rejects_a_step_below_one(step):
    with pytest.raises(ValueError, match="step must be at least 1 byte"):
        byte_entropy.windows(b"abc", 1, step)
    with pytest.raises(ValueError, match="step must be at least 1 byte"):
        byte_entropy.window_count(3, 1, step)


def test_rejects_huge_or_non_int_arguments():
    with pytest.raises(OverflowError):
        byte_entropy.windows(b"abc", 2**70)
    with pytest.raises(TypeError):
        byte_entropy.windows(b"abc", 1.5)
    with pytest.raises(TypeError):
        byte_entropy.windows(b"abc", "4")
    with pytest.raises(OverflowError):
        byte_entropy.window_count(-1, 1)


@pytest.mark.parametrize("n", [0, 1, 255, 257, 1000])
def test_entropy_of_histogram_needs_256_counts(n):
    with pytest.raises(ValueError, match="expected 256 counts"):
        byte_entropy.entropy_of_histogram([1] * n)


def test_entropy_of_histogram_rejects_negative_counts():
    with pytest.raises(OverflowError):
        byte_entropy.entropy_of_histogram([-1] + [1] * 255)


def test_entropy_of_histogram_accepts_huge_counts():
    # 合計が 2**64 を超えても落ちない
    assert byte_entropy.entropy_of_histogram([2**64 - 1] * 256) == 8.0


# --- 端の場合 ---


def test_edge_cases():
    assert byte_entropy.entropy(b"") == 0.0
    assert byte_entropy.entropy(b"\x00") == 0.0
    assert byte_entropy.entropy(b"ab") == 1.0
    assert byte_entropy.entropy(bytes(range(256))) == 8.0
    assert byte_entropy.MAX_ENTROPY == 8.0
    # 窓より短いデータは窓なし。末尾の半端な窓は含めない
    assert byte_entropy.windows(b"abc", 4) == []
    assert byte_entropy.windows(b"", 1) == []
    assert byte_entropy.windows(b"aabbccdd", 2) == [(0, 0.0), (2, 0.0), (4, 0.0), (6, 0.0)]
    assert byte_entropy.windows(b"aabbccd", 2) == [(0, 0.0), (2, 0.0), (4, 0.0)]
    assert byte_entropy.windows(b"abab", 2, 1) == [(0, 1.0), (1, 1.0), (2, 1.0)]
    # 刻みが窓より大きいとすき間ができる
    assert byte_entropy.windows(b"ab__ab__ab", 2, 4) == [(0, 1.0), (4, 1.0), (8, 1.0)]
    assert byte_entropy.window_count(10, 2, 4) == 3
    assert byte_entropy.window_count(0, 1) == 0


def test_step_defaults_to_window():
    d = os.urandom(10_000)
    assert byte_entropy.windows(d, 1000) == byte_entropy.windows(d, 1000, 1000)
    assert byte_entropy.windows(d, 1000, None) == byte_entropy.windows(d, 1000, 1000)
    assert byte_entropy.window_count(10_000, 1000) == 10


def test_return_types():
    got = byte_entropy.windows(b"abcdef", 3)
    assert isinstance(got, list)
    assert all(isinstance(off, int) and isinstance(h, float) for off, h in got)
    assert isinstance(byte_entropy.entropy(b"x"), float)
    assert isinstance(byte_entropy.histogram(b"x"), list)


@given(st.binary(max_size=2000), st.integers(-5, 3000), st.one_of(st.none(), st.integers(-5, 3000)))
def test_never_crashes(d, window, step):
    try:
        got = byte_entropy.windows(d, window, step)
    except ValueError:
        assert window < 1 or (step is not None and step < 1)
        return
    assert len(got) == byte_entropy.window_count(len(d), window, step)


def test_a_mutated_bytearray_does_not_change_a_finished_result():
    d = bytearray(os.urandom(1 << 20))
    first = byte_entropy.entropy(d)
    d[:] = bytes(len(d))
    assert byte_entropy.entropy(d) == 0.0
    assert first > 7.9


# --- GIL とスレッド ---


def test_same_results_from_many_threads():
    blobs = [os.urandom(200_000 + i) for i in range(32)]
    expected = [byte_entropy.windows(b, 4096) for b in blobs]
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        assert list(pool.map(lambda b: byte_entropy.windows(b, 4096), blobs)) == expected
        assert list(pool.map(byte_entropy.entropy, blobs)) == [byte_entropy.entropy(b) for b in blobs]


def progress_while(work) -> int:
    """work を別スレッドで動かしている間に、このスレッドが何回回れたか（GIL を手放していれば多い）。"""
    done = threading.Event()
    thread = threading.Thread(target=lambda: (work(), done.set()))
    count = 0
    thread.start()
    while not done.is_set():
        count += 1
        time.sleep(0)
    thread.join()
    return count


@pytest.mark.parametrize("kind", [bytes, bytearray])
def test_releases_the_gil(kind):
    data = kind(os.urandom(64 << 20))
    assert progress_while(lambda: byte_entropy.entropy(data)) > 100
    assert progress_while(lambda: byte_entropy.windows(data, 4096)) > 100


def test_module_layout():
    import kura_rs

    assert kura_rs.byte_entropy is byte_entropy
    assert set(byte_entropy.__all__) == {"MAX_ENTROPY", "entropy", "entropy_of_histogram", "histogram", "window_count", "windows"}
