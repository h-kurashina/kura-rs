"""kura_rs.minhash と datasketch.MinHash を、ランダムな入力で突き合わせる。"""

from datasketch import MinHash
from hypothesis import given
from hypothesis import strategies as st

from kura_rs import minhash

num_perms = st.integers(min_value=1, max_value=300)
seeds = st.integers(min_value=0, max_value=2**32 - 1)
byte_items = st.lists(st.binary(max_size=64), max_size=40)
text_items = st.lists(st.text(max_size=20), max_size=40)


def reference(items: list[bytes], num_perm: int, seed: int) -> MinHash:
    m = MinHash(num_perm=num_perm, seed=seed)
    m.update_batch(items)
    return m


@given(num_perms, seeds, byte_items)
def test_signature_matches(num_perm, seed, items):
    sig = minhash.MinHasher(num_perm, seed).signature(items)
    assert sig.values == [int(v) for v in reference(items, num_perm, seed).hashvalues]


@given(num_perms, seeds)
def test_permutations_match(num_perm, seed):
    a, b = minhash.MinHasher(num_perm, seed).permutations
    ref = MinHash(num_perm=num_perm, seed=seed).permutations
    assert a == [int(v) for v in ref[0]]
    assert b == [int(v) for v in ref[1]]


@given(num_perms, seeds, byte_items, byte_items)
def test_jaccard_matches(num_perm, seed, xs, ys):
    hasher = minhash.MinHasher(num_perm, seed)
    expected = reference(xs, num_perm, seed).jaccard(reference(ys, num_perm, seed))
    assert hasher.signature(xs).jaccard(hasher.signature(ys)) == expected


@given(num_perms, seeds, text_items)
def test_str_items_are_utf8(num_perm, seed, items):
    sig = minhash.MinHasher(num_perm, seed).signature(items)
    ref = reference([t.encode("utf-8") for t in items], num_perm, seed)
    assert sig.values == [int(v) for v in ref.hashvalues]


@given(num_perms, seeds, byte_items, byte_items)
def test_merge_matches_datasketch_merge(num_perm, seed, xs, ys):
    hasher = minhash.MinHasher(num_perm, seed)
    sig = hasher.signature(xs)
    sig.merge(hasher.signature(ys))
    ref = reference(xs, num_perm, seed)
    ref.merge(reference(ys, num_perm, seed))
    assert sig.values == [int(v) for v in ref.hashvalues]


@given(num_perms, seeds, byte_items)
def test_update_one_by_one_matches(num_perm, seed, items):
    hasher = minhash.MinHasher(num_perm, seed)
    sig = hasher.empty()
    ref = MinHash(num_perm=num_perm, seed=seed)
    for item in items:
        sig.update(hasher, item)
        ref.update(item)
    assert sig.values == [int(v) for v in ref.hashvalues]
