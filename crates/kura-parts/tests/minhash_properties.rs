//! MinHash の性質テスト。ランダムな入力を大量に作り、どんな入力でも成り立つべき性質を確かめる。
//! 1つの性質につき既定で 2,000 通り。PROPTEST_CASES=100000 のように環境変数で増やせる。

use kura_parts::minhash::MinHasher;
use proptest::prelude::*;

fn config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2_000);
    ProptestConfig {
        cases,
        ..ProptestConfig::default()
    }
}

/// 任意のバイト列の集合（空・長いもの・重複を含む）
fn items() -> impl Strategy<Value = Vec<Vec<u8>>> {
    prop::collection::vec(prop::collection::vec(any::<u8>(), 0..64), 0..40)
}

fn params() -> impl Strategy<Value = (usize, u32)> {
    (1usize..300, any::<u32>())
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn order_does_not_matter((n, seed) in params(), mut xs in items()) {
        let hasher = MinHasher::new(n, seed);
        let forward = hasher.signature(&xs);
        xs.reverse();
        prop_assert_eq!(forward, hasher.signature(&xs));
    }

    #[test]
    fn duplicates_do_not_matter((n, seed) in params(), xs in items()) {
        let hasher = MinHasher::new(n, seed);
        let doubled: Vec<&Vec<u8>> = xs.iter().chain(xs.iter()).collect();
        prop_assert_eq!(hasher.signature(&xs), hasher.signature(doubled));
    }

    #[test]
    fn union_equals_merge((n, seed) in params(), xs in items(), ys in items()) {
        let hasher = MinHasher::new(n, seed);
        let mut merged = hasher.signature(&xs);
        merged.merge(&hasher.signature(&ys));
        prop_assert_eq!(merged, hasher.signature(xs.iter().chain(ys.iter())));
    }

    #[test]
    fn jaccard_is_a_ratio_in_unit_range((n, seed) in params(), xs in items(), ys in items()) {
        let hasher = MinHasher::new(n, seed);
        let (a, b) = (hasher.signature(&xs), hasher.signature(&ys));
        let j = a.jaccard(&b);
        prop_assert!((0.0..=1.0).contains(&j));
        prop_assert_eq!(j, b.jaccard(&a), "jaccard must be symmetric");
        // 0/n, 1/n, ..., n/n のどれか
        let k = (j * n as f64).round();
        prop_assert_eq!(j, k / n as f64);
    }

    #[test]
    fn identical_sets_have_jaccard_one((n, seed) in params(), xs in items()) {
        let hasher = MinHasher::new(n, seed);
        prop_assert_eq!(hasher.signature(&xs).jaccard(&hasher.signature(&xs)), 1.0);
    }

    #[test]
    fn adding_items_never_raises_a_value((n, seed) in params(), xs in items(), extra in items()) {
        let hasher = MinHasher::new(n, seed);
        let small = hasher.signature(&xs);
        let large = hasher.signature(xs.iter().chain(extra.iter()));
        prop_assert!(large.values().iter().zip(small.values()).all(|(l, s)| l <= s));
    }

    #[test]
    fn signature_length_is_num_perm((n, seed) in params(), xs in items()) {
        let hasher = MinHasher::new(n, seed);
        prop_assert_eq!(hasher.num_perm(), n);
        prop_assert_eq!(hasher.signature(&xs).values().len(), n);
        let (a, b) = hasher.permutations();
        prop_assert_eq!((a.len(), b.len()), (n, n));
    }

    #[test]
    fn same_seed_same_permutations((n, seed) in params()) {
        prop_assert_eq!(MinHasher::new(n, seed), MinHasher::new(n, seed));
    }

    #[test]
    fn a_prefix_of_permutations_is_stable(seed in any::<u32>(), n in 1usize..200, extra in 1usize..200) {
        // datasketch と同じく a を全部引いてから b を引くので、num_perm を増やすと b の並びはずれる。
        // a の先頭は num_perm によらず同じになる
        let small = MinHasher::new(n, seed);
        let large = MinHasher::new(n + extra, seed);
        prop_assert_eq!(small.permutations().0, &large.permutations().0[..n]);
    }

    #[test]
    fn arbitrary_bytes_never_panic(n in 1usize..64, seed in any::<u32>(), bytes in prop::collection::vec(any::<u8>(), 0..4096)) {
        let hasher = MinHasher::new(n, seed);
        let mut sig = hasher.empty();
        for chunk in bytes.chunks(7) {
            sig.update(&hasher, chunk);
        }
        sig.update(&hasher, &bytes);
        prop_assert_eq!(sig.values().len(), n);
    }
}
