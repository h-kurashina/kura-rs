"""kura_rs.file_hash の使い方：誤った値はエラーになり、落ちないこと。GIL を手放すこと。"""

import concurrent.futures
import hashlib
import os
import threading
import time

import pytest

from kura_rs import file_hash

ABC = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"


# --- 引数の検査 ---

@pytest.mark.parametrize("name", ["md5", "sha1", "sha512", "", "sha_256", " sha256", "blake2b", "SHA3-256"])
def test_rejects_unknown_algorithms(name):
    with pytest.raises(ValueError, match="unknown algorithm"):
        file_hash.hash_bytes(name, b"")
    with pytest.raises(ValueError, match="unknown algorithm"):
        file_hash.Hasher(name)
    with pytest.raises(ValueError, match="unknown algorithm"):
        file_hash.hash_file(name, __file__)
    with pytest.raises(ValueError, match="unknown algorithm"):
        file_hash.Digest.from_hex(name, ABC)


@pytest.mark.parametrize("name", ["sha256", "SHA256", "sha-256", "SHA-256", "Sha256"])
def test_accepts_sha256_spellings(name):
    assert file_hash.hash_bytes(name, b"abc").hexdigest() == ABC
    assert file_hash.hash_bytes(name, b"abc").algorithm == "sha256"


@pytest.mark.parametrize("name", ["blake3", "BLAKE3", "Blake3"])
def test_accepts_blake3_spellings(name):
    assert file_hash.hash_bytes(name, b"").algorithm == "blake3"


@pytest.mark.parametrize("bad", [None, 1, b"sha256", ["sha256"]])
def test_rejects_non_str_algorithm(bad):
    with pytest.raises(TypeError):
        file_hash.hash_bytes(bad, b"")


def test_rejects_str_data_like_hashlib():
    with pytest.raises(TypeError, match="Strings must be encoded"):
        file_hash.hash_bytes("sha256", "abc")
    with pytest.raises(TypeError, match="Strings must be encoded"):
        file_hash.Hasher("sha256").update("abc")
    with pytest.raises(TypeError):
        hashlib.sha256("abc")  # hashlib も同じ


@pytest.mark.parametrize("bad", [None, 1, 1.5, object(), [1, 2], (b"a",), {"a": 1}])
def test_rejects_non_buffers(bad):
    with pytest.raises(TypeError, match="bytes-like object is required"):
        file_hash.hash_bytes("sha256", bad)
    with pytest.raises(TypeError, match="bytes-like object is required"):
        file_hash.Hasher("blake3").update(bad)


def test_rejects_non_contiguous_buffers_like_hashlib():
    view = memoryview(bytes(range(10)))[::2]
    with pytest.raises(BufferError):
        file_hash.hash_bytes("sha256", view)
    with pytest.raises(BufferError):
        hashlib.sha256(view)


def test_rejects_non_contiguous_numpy_arrays():
    np = pytest.importorskip("numpy")
    arr = np.arange(12, dtype=np.int32).reshape(3, 4).T
    with pytest.raises((BufferError, ValueError)):
        file_hash.hash_bytes("sha256", arr)


def test_failed_update_leaves_the_hasher_unchanged():
    h = file_hash.Hasher("sha256", b"abc")
    with pytest.raises(TypeError):
        h.update("text")
    assert h.hexdigest() == ABC


# --- ファイルのエラー ---

def test_missing_file_is_file_not_found(tmp_path):
    path = tmp_path / "missing.bin"
    with pytest.raises(FileNotFoundError) as info:
        file_hash.hash_file("sha256", path)
    assert info.value.errno is not None
    assert info.value.filename == str(path)
    assert "os error" not in info.value.strerror


def test_directory_is_an_error(tmp_path):
    with pytest.raises(OSError):
        file_hash.hash_file("blake3", tmp_path)


@pytest.mark.skipif(os.name != "posix" or os.geteuid() == 0, reason="needs file permissions (and not root)")
def test_unreadable_file_is_permission_error(tmp_path):
    path = tmp_path / "secret.bin"
    path.write_bytes(b"x")
    path.chmod(0)
    try:
        with pytest.raises(PermissionError):
            file_hash.hash_file("sha256", path)
    finally:
        path.chmod(0o600)


def test_hash_files_stops_at_the_first_bad_path(tmp_path):
    good = tmp_path / "good.bin"
    good.write_bytes(b"abc")
    with pytest.raises(FileNotFoundError):
        file_hash.hash_files("sha256", [good, tmp_path / "missing.bin", good])


def test_hash_files_rejects_non_paths():
    with pytest.raises(TypeError):
        file_hash.hash_files("sha256", [1, 2])
    with pytest.raises(TypeError):
        file_hash.hash_file("sha256", 42)


def test_update_file_missing(tmp_path):
    h = file_hash.Hasher("sha256", b"abc")
    with pytest.raises(FileNotFoundError):
        h.update_file(tmp_path / "missing.bin")
    assert h.hexdigest() == ABC


def test_path_kinds(tmp_path):
    path = tmp_path / "f.bin"
    path.write_bytes(b"abc")

    class MyPath(os.PathLike):
        def __fspath__(self):
            return str(path)

    for p in [path, str(path), MyPath()]:
        assert file_hash.hash_file("sha256", p).hexdigest() == ABC


def test_unicode_file_name(tmp_path):
    path = tmp_path / "データ 🦀.bin"
    path.write_bytes(b"abc")
    assert file_hash.hash_file("sha256", path).hexdigest() == ABC


def test_empty_file(tmp_path):
    path = tmp_path / "empty"
    path.write_bytes(b"")
    assert file_hash.hash_file("sha256", path).hexdigest() == hashlib.sha256(b"").hexdigest()
    assert file_hash.hash_files("sha256", []) == []


def test_file_changes_are_seen(tmp_path):
    path = tmp_path / "f.bin"
    path.write_bytes(b"v1")
    before = file_hash.hash_file("sha256", path)
    path.write_bytes(b"v2")
    assert file_hash.hash_file("sha256", path) != before


# --- Digest ---

def test_digest_attributes():
    d = file_hash.hash_bytes("sha256", b"abc")
    assert d.algorithm == "sha256"
    assert d.digest_size == 32 == file_hash.DIGEST_SIZE
    assert d.hex() == d.hexdigest() == str(d) == ABC
    assert len(d.digest()) == 32
    assert repr(d) == f"Digest('sha256', '{ABC}')"
    assert file_hash.ALGORITHMS == ("sha256", "blake3")


def test_digest_equality():
    a = file_hash.hash_bytes("sha256", b"abc")
    assert a == file_hash.Digest.from_hex("sha256", ABC)
    assert a == file_hash.Digest.from_hex("sha256", ABC.upper())
    assert a != file_hash.hash_bytes("sha256", b"abd")
    # 同じ 32 バイトでもアルゴリズムが違えば別物
    assert file_hash.Digest.from_hex("sha256", ABC) != file_hash.Digest.from_hex("blake3", ABC)
    # 文字列や bytes とは等しくならない（比べるなら hexdigest() / digest() を使う）
    assert a != ABC
    assert a != a.digest()


def test_digest_is_hashable_for_dedup():
    digests = {file_hash.hash_bytes("blake3", x) for x in [b"a", b"b", b"a", b"", b""]}
    assert len(digests) == 3


def test_digest_is_immutable():
    d = file_hash.hash_bytes("sha256", b"abc")
    with pytest.raises(AttributeError):
        d.algorithm = "blake3"


@pytest.mark.parametrize(
    "bad",
    ["", ABC[:63], ABC + "0", "g" + ABC[1:], " " + ABC[1:], "0x" + ABC[2:], "é" + ABC[2:], "ｆ" * 21 + "a"],
)
def test_from_hex_rejects(bad):
    with pytest.raises(ValueError):
        file_hash.Digest.from_hex("sha256", bad)


# --- Hasher ---

def test_hasher_defaults_to_sha256():
    h = file_hash.Hasher()
    h.update(b"abc")
    assert h.name == "sha256"
    assert h.digest_size == 32
    assert h.hexdigest() == ABC
    assert repr(h) == "Hasher('sha256')"


def test_hasher_keyword_arguments():
    assert file_hash.Hasher(algorithm="sha256", data=b"abc").hexdigest() == ABC
    assert file_hash.hash_bytes(algorithm="sha256", data=b"abc").hexdigest() == ABC


def test_update_returns_none():
    assert file_hash.Hasher().update(b"x") is None


def test_update_file_counts_bytes(tmp_path):
    path = tmp_path / "f.bin"
    path.write_bytes(b"bc")
    h = file_hash.Hasher("sha256", b"a")
    assert h.update_file(path) == 2
    assert h.hexdigest() == ABC


def test_bytearray_mutated_after_update_does_not_matter():
    buf = bytearray(b"abc")
    h = file_hash.Hasher("sha256", buf)
    buf[:] = b"xyz"
    assert h.hexdigest() == ABC


# --- GIL とスレッド ---

def test_same_results_from_many_threads(tmp_path):
    blobs = [os.urandom(200_000 + i) for i in range(32)]
    paths = []
    for i, blob in enumerate(blobs):
        p = tmp_path / f"{i}.bin"
        p.write_bytes(blob)
        paths.append(p)
    expected = [hashlib.sha256(b).hexdigest() for b in blobs]
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        assert [d.hexdigest() for d in pool.map(lambda b: file_hash.hash_bytes("sha256", b), blobs)] == expected
        assert [d.hexdigest() for d in pool.map(lambda p: file_hash.hash_file("sha256", p), paths)] == expected
    assert [d.hexdigest() for d in file_hash.hash_files("sha256", paths)] == expected


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
def test_releases_the_gil_while_hashing_memory(kind):
    data = kind(256 << 20)
    assert progress_while(lambda: file_hash.hash_bytes("sha256", data)) > 100


def test_releases_the_gil_while_hashing_files(tmp_path):
    path = tmp_path / "big.bin"
    with open(path, "wb") as f:
        f.truncate(256 << 20)  # 256 MiB のゼロ（ディスクをほとんど使わない）
    assert progress_while(lambda: file_hash.hash_file("sha256", path)) > 100


def test_module_layout():
    import kura_rs

    assert kura_rs.file_hash is file_hash
    assert file_hash.Hasher.__module__ == "kura_rs.file_hash"
    assert file_hash.Digest.__module__ == "kura_rs.file_hash"
