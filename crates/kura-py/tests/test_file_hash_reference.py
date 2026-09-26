"""kura_rs.file_hash を hashlib.sha256・blake3 パッケージと、ランダムな入力で突き合わせる。"""

import array
import hashlib
import os
import tempfile
from pathlib import Path

import blake3
import numpy as np
import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

from kura_rs import file_hash

ALGORITHMS = ["sha256", "blake3"]
algorithms = st.sampled_from(ALGORITHMS)
data = st.one_of(
    st.binary(max_size=300),
    st.binary(max_size=5000),
    st.binary(min_size=60, max_size=70),  # SHA-256 / BLAKE3 のブロック（64 バイト）の境目
    st.binary(min_size=1020, max_size=1030),  # BLAKE3 のチャンク（1024 バイト）の境目
    st.builds(lambda b, n: bytes([b]) * n, st.integers(0, 255), st.integers(0, 70_000)),
)
# 区切りの位置（0 バイトの区切りやブロックの境目を含む）
splits = st.lists(st.one_of(st.integers(0, 3), st.integers(0, 200), st.sampled_from([63, 64, 65, 1023, 1024, 1025, 2047, 2048, 2049])), max_size=20)


def reference(algorithm: str, data: bytes):
    return hashlib.sha256(data) if algorithm == "sha256" else blake3.blake3(data)


def split(data: bytes, sizes: list[int]) -> list[bytes]:
    pieces, rest = [], data
    for size in sizes:
        pieces.append(rest[:size])
        rest = rest[size:]
    return pieces + [rest]


# --- メモリ上のバイト列 ---

@given(algorithms, data)
def test_hash_bytes_matches(algorithm, data):
    ref = reference(algorithm, data)
    d = file_hash.hash_bytes(algorithm, data)
    assert d.hexdigest() == ref.hexdigest()
    assert d.digest() == ref.digest()
    assert bytes(d) == ref.digest()
    assert str(d) == ref.hexdigest()


@given(data)
def test_sha256_matches_hashlib(data):
    assert file_hash.hash_bytes("sha256", data).hexdigest() == hashlib.sha256(data).hexdigest()


@given(data)
def test_blake3_matches_blake3_package(data):
    assert file_hash.hash_bytes("blake3", data).hexdigest() == blake3.blake3(data).hexdigest()


@given(algorithms, data)
def test_bytearray_and_memoryview_match(algorithm, data):
    expected = reference(algorithm, data).hexdigest()
    assert file_hash.hash_bytes(algorithm, bytearray(data)).hexdigest() == expected
    assert file_hash.hash_bytes(algorithm, memoryview(data)).hexdigest() == expected
    assert file_hash.hash_bytes(algorithm, memoryview(bytearray(data))).hexdigest() == expected


@given(algorithms, st.binary(max_size=2000), st.integers(0, 100), st.integers(0, 100))
def test_memoryview_slices_match(algorithm, data, head, tail):
    view = memoryview(data)[head : max(head, len(data) - tail)]
    assert file_hash.hash_bytes(algorithm, view).hexdigest() == reference(algorithm, view.tobytes()).hexdigest()


@given(algorithms, st.lists(st.integers(-(2**31), 2**31 - 1), max_size=500), st.sampled_from(["i", "l", "q", "I"]))
def test_array_array_matches(algorithm, values, typecode):
    if typecode == "I":
        values = [v % 2**32 for v in values]
    a = array.array(typecode, values)
    # hashlib と同じく、中身のバイト列をそのまま見る
    assert file_hash.hash_bytes(algorithm, a).hexdigest() == reference(algorithm, a.tobytes()).hexdigest()


@given(algorithms, st.integers(0, 50), st.integers(1, 20), st.sampled_from([np.uint8, np.int16, np.float32, np.float64, np.int64]))
def test_numpy_arrays_match(algorithm, rows, cols, dtype):
    arr = (np.arange(rows * cols) * 7919).astype(dtype).reshape(rows, cols)
    assert file_hash.hash_bytes(algorithm, arr).hexdigest() == reference(algorithm, arr.tobytes()).hexdigest()


# --- 分割入力（どこで区切っても答えは同じ） ---

@given(algorithms, data, splits)
def test_hasher_split_matches(algorithm, data, sizes):
    h = file_hash.Hasher(algorithm)
    ref = reference(algorithm, b"")
    for piece in split(data, sizes):
        h.update(piece)
        ref.update(piece)
    assert h.hexdigest() == ref.hexdigest() == reference(algorithm, data).hexdigest()
    assert h.finalize() == file_hash.hash_bytes(algorithm, data)


@given(algorithms, data, splits)
def test_hasher_split_with_mixed_buffer_types(algorithm, data, sizes):
    h = file_hash.Hasher(algorithm)
    kinds = [bytes, bytearray, memoryview]
    for i, piece in enumerate(split(data, sizes)):
        h.update(kinds[i % 3](piece))
    assert h.hexdigest() == reference(algorithm, data).hexdigest()


@given(algorithms, data, data)
def test_hasher_initial_data(algorithm, first, second):
    h = file_hash.Hasher(algorithm, first)
    h.update(second)
    assert h.hexdigest() == reference(algorithm, first + second).hexdigest()


@given(algorithms, data, data)
def test_digest_midway_then_continue(algorithm, first, second):
    h = file_hash.Hasher(algorithm)
    h.update(first)
    assert h.hexdigest() == reference(algorithm, first).hexdigest()
    assert h.digest() == reference(algorithm, first).digest()
    h.update(second)
    assert h.hexdigest() == reference(algorithm, first + second).hexdigest()


@given(algorithms, data, data, data)
def test_copy_forks_like_hashlib(algorithm, prefix, x, y):
    h = file_hash.Hasher(algorithm, prefix)
    ref = reference(algorithm, prefix)
    h2, ref2 = h.copy(), ref.copy()
    h.update(x)
    ref.update(x)
    h2.update(y)
    ref2.update(y)
    assert h.hexdigest() == ref.hexdigest()
    assert h2.hexdigest() == ref2.hexdigest()


@given(algorithms, data, data)
def test_reset(algorithm, junk, data):
    h = file_hash.Hasher(algorithm, junk)
    h.reset()
    h.update(data)
    assert h.hexdigest() == reference(algorithm, data).hexdigest()


# --- ファイル ---

@settings(max_examples=max(50, settings().max_examples // 10))
@given(algorithms, data)
def test_hash_file_matches(algorithm, data):
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "f.bin"
        path.write_bytes(data)
        expected = reference(algorithm, data).hexdigest()
        assert file_hash.hash_file(algorithm, path).hexdigest() == expected
        assert file_hash.hash_file(algorithm, str(path)).hexdigest() == expected
        h = file_hash.Hasher(algorithm)
        assert h.update_file(path) == len(data)
        assert h.hexdigest() == expected


@settings(max_examples=max(50, settings().max_examples // 10))
@given(algorithms, st.lists(data, max_size=8))
def test_hash_files_matches(algorithm, contents):
    with tempfile.TemporaryDirectory() as tmp:
        paths = []
        for i, c in enumerate(contents):
            p = os.path.join(tmp, f"{i}.bin")
            Path(p).write_bytes(c)
            paths.append(p)
        got = [d.hexdigest() for d in file_hash.hash_files(algorithm, paths)]
        assert got == [reference(algorithm, c).hexdigest() for c in contents]


# --- 16進 ---

@given(algorithms, st.binary(min_size=32, max_size=32), st.booleans())
def test_from_hex_round_trips(algorithm, raw, upper):
    hex_ = raw.hex().upper() if upper else raw.hex()
    d = file_hash.Digest.from_hex(algorithm, hex_)
    assert d.digest() == raw
    assert d.hexdigest() == raw.hex()
    assert d.algorithm == algorithm


@given(algorithms, st.text(max_size=80))
def test_from_hex_rejects_or_round_trips(algorithm, text):
    try:
        d = file_hash.Digest.from_hex(algorithm, text)
    except ValueError:
        return
    assert d.hexdigest() == text.lower()


@given(algorithms, data, data)
def test_equality_and_hash_follow_content(algorithm, a, b):
    da, db = file_hash.hash_bytes(algorithm, a), file_hash.hash_bytes(algorithm, b)
    assert (da == db) == (a == b)
    if a == b:
        assert hash(da) == hash(db)


@given(data)
def test_algorithms_never_agree(data):
    assert file_hash.hash_bytes("sha256", data) != file_hash.hash_bytes("blake3", data)
    assert file_hash.hash_bytes("sha256", data).digest() != file_hash.hash_bytes("blake3", data).digest()


@given(algorithms, st.binary(min_size=1, max_size=3000), st.data())
def test_one_flipped_bit_changes_the_digest(algorithm, data, draw):
    pos = draw.draw(st.integers(0, len(data) - 1))
    bit = draw.draw(st.integers(0, 7))
    tampered = bytearray(data)
    tampered[pos] ^= 1 << bit
    assert file_hash.hash_bytes(algorithm, bytes(tampered)) != file_hash.hash_bytes(algorithm, data)


# --- 大きさの境目（固定） ---

# 1 MiB は bytearray などを写す単位、2048 は GIL を手放すかどうかの境目、64 KiB はファイルの読み込み単位
EDGE_SIZES = sorted({n + d for n in [0, 64, 1024, 2048, 4096, 65536, 1 << 20, 3 << 20] for d in (-1, 0, 1) if n + d >= 0})


@pytest.mark.parametrize("size", EDGE_SIZES)
@pytest.mark.parametrize("algorithm", ALGORITHMS)
def test_edge_sizes(algorithm, size, tmp_path):
    data = (bytes(range(251)) * (size // 251 + 1))[:size]
    expected = reference(algorithm, data).hexdigest()
    assert file_hash.hash_bytes(algorithm, data).hexdigest() == expected
    assert file_hash.hash_bytes(algorithm, bytearray(data)).hexdigest() == expected
    assert file_hash.hash_bytes(algorithm, memoryview(data)).hexdigest() == expected
    path = tmp_path / "f.bin"
    path.write_bytes(data)
    assert file_hash.hash_file(algorithm, path).hexdigest() == expected


@pytest.mark.parametrize("algorithm", ALGORITHMS)
@pytest.mark.parametrize("fill", [0x00, 0xFF])
def test_large_uniform_inputs(algorithm, fill):
    data = bytes([fill]) * (32 << 20)
    assert file_hash.hash_bytes(algorithm, data).hexdigest() == reference(algorithm, data).hexdigest()


# --- 公式テストベクタ（Rust 側 tests/file_hash_vectors.rs と同じ出典） ---

@pytest.mark.parametrize(
    ("message", "expected"),
    [
        (b"", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        (b"abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        (b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"),
        (b"a" * 1_000_000, "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"),
    ],
)
def test_sha256_fips_180_vectors(message, expected):
    assert file_hash.hash_bytes("sha256", message).hexdigest() == expected


@pytest.mark.parametrize(
    ("length", "expected"),
    [
        (0, "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"),
        (1, "2d3adedff11b61f14c886e35afa036736dcd87a74d27b5c1510225d0f592e213"),
        (1023, "10108970eeda3eb932baac1428c7a2163b0e924c9a9e25b35bba72b28f70bd11"),
        (1024, "42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7"),
        (1025, "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444"),
        (102400, "bc3e3d41a1146b069abffad3c0d44860cf664390afce4d9661f7902e7943e085"),
    ],
)
def test_blake3_official_vectors(length, expected):
    data = (bytes(range(251)) * (length // 251 + 1))[:length]
    assert file_hash.hash_bytes("blake3", data).hexdigest() == expected
