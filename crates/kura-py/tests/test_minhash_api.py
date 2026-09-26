"""kura_rs.minhash の使い方：誤った値はエラーになり、落ちないこと。"""

import concurrent.futures

import pytest
from hypothesis import given
from hypothesis import strategies as st

from kura_rs import minhash

items = st.lists(st.binary(max_size=32), max_size=30)


# --- 引数の検査 ---

@pytest.mark.parametrize("num_perm", [0, -1, -(2**40)])
def test_rejects_non_positive_num_perm(num_perm):
    with pytest.raises(ValueError, match="num_perm must be positive"):
        minhash.MinHasher(num_perm)


@pytest.mark.parametrize("seed", [-1, 2**32, 2**64])
def test_rejects_seed_out_of_range(seed):
    with pytest.raises((ValueError, OverflowError)):
        minhash.MinHasher(8, seed)


def test_accepts_seed_bounds():
    minhash.MinHasher(8, 0)
    minhash.MinHasher(8, 2**32 - 1)


@pytest.mark.parametrize("bad", [1, 1.5, None, object(), ["nested"], {"a": 1}])
def test_rejects_non_bytes_items(bad):
    with pytest.raises(TypeError, match="bytes, bytearray or str"):
        minhash.MinHasher(8).signature([b"ok", bad])


@pytest.mark.parametrize("single", [b"abc", "abc"])
def test_rejects_a_single_bytes_or_str(single):
    # 文字列を1つ渡すと1文字ずつの集合として扱われてしまうのを防ぐ
    with pytest.raises(TypeError, match="iterable of items"):
        minhash.MinHasher(8).signature(single)


def test_rejects_non_iterable():
    with pytest.raises(TypeError):
        minhash.MinHasher(8).signature(42)


def test_jaccard_rejects_different_num_perm():
    a = minhash.MinHasher(8).signature([b"x"])
    b = minhash.MinHasher(16).signature([b"x"])
    with pytest.raises(ValueError, match="num_perm"):
        a.jaccard(b)
    with pytest.raises(ValueError, match="num_perm"):
        a.merge(b)


def test_update_rejects_a_different_hasher():
    sig = minhash.MinHasher(8).empty()
    with pytest.raises(ValueError, match="hasher"):
        sig.update(minhash.MinHasher(16), b"x")
    with pytest.raises(ValueError, match="hasher"):
        sig.update_batch(minhash.MinHasher(16), [b"x"])


def test_iterator_errors_propagate():
    def broken():
        yield b"a"
        raise RuntimeError("boom")

    with pytest.raises(RuntimeError, match="boom"):
        minhash.MinHasher(8).signature(broken())


# --- 振る舞い ---

@given(items)
def test_bytes_bytearray_and_generators_agree(xs):
    hasher = minhash.MinHasher(32)
    expected = hasher.signature(xs)
    assert hasher.signature([bytearray(x) for x in xs]) == expected
    assert hasher.signature(x for x in xs) == expected
    assert hasher.signature(tuple(xs)) == expected


@given(st.lists(items, max_size=10))
def test_signatures_equals_signature_per_document(docs):
    hasher = minhash.MinHasher(32)
    assert hasher.signatures(docs) == [hasher.signature(d) for d in docs]


@given(items, items)
def test_update_batch_equals_signature(xs, ys):
    hasher = minhash.MinHasher(32)
    sig = hasher.signature(xs)
    sig.update_batch(hasher, ys)
    assert sig == hasher.signature(xs + ys)


def test_empty_signature():
    sig = minhash.MinHasher(4).empty()
    assert sig.values == [2**32 - 1] * 4
    assert sig == minhash.MinHasher(4).signature([])


def test_copy_is_independent():
    hasher = minhash.MinHasher(16)
    sig = hasher.signature([b"a"])
    dup = sig.copy()
    dup.update(hasher, b"b")
    assert sig == hasher.signature([b"a"])
    assert dup != sig


def test_len_repr_and_aliases():
    hasher = minhash.MinHasher(64, 7)
    sig = hasher.signature([b"a"])
    assert len(sig) == 64 and hasher.num_perm == 64
    assert sig.hashvalues == sig.values
    assert repr(hasher) == "MinHasher(num_perm=64)"
    assert repr(sig) == "Signature(num_perm=64)"


def test_jaccard_of_identical_and_disjoint_sets():
    hasher = minhash.MinHasher(256)
    a = hasher.signature([f"a{i}" for i in range(200)])
    b = hasher.signature([f"b{i}" for i in range(200)])
    assert a.jaccard(a) == 1.0
    assert a.jaccard(b) < 0.1


def test_large_input_does_not_crash():
    sig = minhash.MinHasher(128).signature(f"token-{i}" for i in range(200_000))
    assert len(sig) == 128


def test_large_items_do_not_crash():
    sig = minhash.MinHasher(8).signature([b"\x00" * 10_000_000, b"\xff" * 1_000_000])
    assert len(sig) == 8


def test_same_results_from_many_threads():
    hasher = minhash.MinHasher(128)
    docs = [[f"t{i}-{j}" for j in range(2000)] for i in range(32)]
    expected = [hasher.signature(d) for d in docs]
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        assert list(pool.map(hasher.signature, docs)) == expected


def test_hasher_is_immutable():
    hasher = minhash.MinHasher(8)
    with pytest.raises(AttributeError):
        hasher.num_perm = 16


def test_module_layout():
    import kura_rs

    assert kura_rs.minhash is minhash
    assert isinstance(kura_rs.__version__, str)
    assert minhash.MinHasher.__module__ == "kura_rs.minhash"
