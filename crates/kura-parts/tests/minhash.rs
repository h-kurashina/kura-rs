//! datasketch 2.0.0 で求めた値との照合（Python がなくても CI で走る分）。
//! たくさんの入力での突き合わせは verify/run.py minhash が行う。

use kura_parts::minhash::MinHasher;

#[test]
fn matches_datasketch_signature() {
    // MinHash(num_perm=8, seed=1).update_batch([b"hello", b"world"]).hashvalues
    let sig = MinHasher::new(8, 1).signature(["hello", "world"]);
    assert_eq!(
        sig.values(),
        [
            1255611051, 658569228, 453856259, 3032158293, 1637026616, 1348343608, 1081391673,
            2120294805
        ]
    );
}

#[test]
fn empty_signature_is_all_max() {
    let sig = MinHasher::new(4, 42).signature(std::iter::empty::<&[u8]>());
    assert_eq!(sig.values(), [u32::MAX; 4]);
}

#[test]
fn matches_datasketch_jaccard() {
    let hasher = MinHasher::new(64, 7);
    let a = hasher.signature((0..50).map(|i| format!("t{i}")));
    let b = hasher.signature((25..75).map(|i| format!("t{i}")));
    assert_eq!(a.jaccard(&b), 0.34375);
}

#[test]
fn merge_is_the_union() {
    let hasher = MinHasher::new(32, 1);
    let mut a = hasher.signature(["a", "b"]);
    a.merge(&hasher.signature(["c"]));
    assert_eq!(a, hasher.signature(["a", "b", "c"]));
}

#[test]
fn update_matches_signature() {
    let hasher = MinHasher::new(16, 3);
    let mut sig = hasher.empty();
    for token in ["x", "y", "z"] {
        sig.update(&hasher, token.as_bytes());
    }
    assert_eq!(sig, hasher.signature(["x", "y", "z"]));
}

#[test]
#[should_panic(expected = "num_perm must be positive")]
fn zero_permutations_panic() {
    MinHasher::new(0, 1);
}
