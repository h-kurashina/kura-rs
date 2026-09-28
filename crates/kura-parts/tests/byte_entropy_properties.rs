//! byte_entropy の性質テスト。ランダムな入力を大量に作り、どんな入力でも成り立つべき性質を確かめる。
//! 1つの性質につき既定で 2,000 通り。PROPTEST_CASES=100000 のように環境変数で増やせる。

use kura_parts::byte_entropy::{
    MAX_ENTROPY, MAX_ROUNDING_ABOVE_8, entropy, entropy_of_histogram, histogram, window_count,
    windows,
};
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

/// 空・短い・偏った（使う値が少ない）・長いバイト列
fn data() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        4 => prop::collection::vec(any::<u8>(), 0..300),
        2 => prop::collection::vec(any::<u8>(), 0..20_000),
        2 => (prop::collection::vec(any::<u8>(), 1..6), 0usize..5000)
            .prop_flat_map(|(values, n)| prop::collection::vec(prop::sample::select(values), n)),
        1 => (0usize..5000, any::<u8>()).prop_map(|(n, b)| vec![b; n]),
    ]
}

/// 0 から 255 の並べ替え（バイトの値の付け替え）
fn relabeling() -> impl Strategy<Value = Vec<u8>> {
    Just((0..=255u8).collect::<Vec<u8>>()).prop_shuffle()
}

/// 足し算の順番が変わったときの差の許容（8 ビットの値の 1e-13 は、有効数字でおよそ 14 桁）
const REORDER_TOLERANCE: f64 = 1e-13;

proptest! {
    #![proptest_config(config())]

    #[test]
    fn entropy_is_between_zero_and_eight(data in data()) {
        let h = entropy(&data);
        prop_assert!(h >= 0.0, "{h}");
        prop_assert!(h <= MAX_ENTROPY + MAX_ROUNDING_ABOVE_8, "{h}");
        prop_assert!(!h.is_nan());
    }

    #[test]
    fn nearly_uniform_counts_stay_within_the_rounding_bound(
        base in 1u64..(1 << 40),
        bumps in prop::collection::vec((0usize..256, 0u64..3), 0..6),
    ) {
        let mut counts = [base; 256];
        for (i, extra) in bumps {
            counts[i] += extra;
        }
        let h = entropy_of_histogram(&counts);
        prop_assert!(h > 7.99 && h <= MAX_ENTROPY + MAX_ROUNDING_ABOVE_8, "{h}");
    }

    #[test]
    fn entropy_is_at_most_log2_of_distinct_values(data in data()) {
        let distinct = histogram(&data).iter().filter(|&&c| c > 0).count();
        let h = entropy(&data);
        if distinct <= 1 {
            prop_assert_eq!(h, 0.0);
        } else {
            prop_assert!(h <= (distinct as f64).log2() + 1e-12, "{} > log2({})", h, distinct);
        }
    }

    #[test]
    fn uniform_over_all_256_values_is_exactly_eight(reps in 1usize..200, shuffle in relabeling()) {
        let data: Vec<u8> = shuffle.iter().copied().cycle().take(256 * reps).collect();
        prop_assert_eq!(entropy(&data), 8.0);
    }

    #[test]
    fn uniform_over_2_to_the_k_values_is_exactly_k(k in 0u32..=8, reps in 1usize..100, shuffle in relabeling()) {
        // 値 0..2^k なら丸めずに k ちょうど。ほかの値の組では、足す順番しだいで 1 ulp ほどずれうる（SciPy も同じ）
        let data: Vec<u8> = (0..1usize << k).map(|v| v as u8).cycle().take((1 << k) * reps).collect();
        prop_assert_eq!(entropy(&data), f64::from(k));
        let values = &shuffle[..1 << k];
        let data: Vec<u8> = values.iter().copied().cycle().take(values.len() * reps).collect();
        prop_assert!((entropy(&data) - f64::from(k)).abs() <= REORDER_TOLERANCE);
    }

    #[test]
    fn byte_order_does_not_matter(mut data in data(), seed in any::<u64>()) {
        // 同じ数え上げになるので、1 ビットも変わらない
        let before = entropy(&data);
        let mut x = seed | 1;
        for i in (1..data.len()).rev() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            data.swap(i, (x % (i as u64 + 1)) as usize);
        }
        prop_assert_eq!(entropy(&data).to_bits(), before.to_bits());
    }

    #[test]
    fn relabeling_byte_values_does_not_matter(data in data(), map in relabeling()) {
        // 値を付け替えると 256 個の項の足す順番が変わるので、丸めの分だけ違いうる（SciPy も同じ）
        let relabeled: Vec<u8> = data.iter().map(|&b| map[usize::from(b)]).collect();
        let (a, b) = (entropy(&data), entropy(&relabeled));
        prop_assert!((a - b).abs() <= REORDER_TOLERANCE, "{a} vs {b}");
    }

    #[test]
    fn repeating_the_data_does_not_change_entropy(data in data(), k in 1usize..5) {
        // 各値の割合 (k c) / (k n) は c / n と同じ数なので、丸めも同じ
        prop_assert_eq!(entropy(&data.repeat(k)).to_bits(), entropy(&data).to_bits());
    }

    #[test]
    fn histograms_of_pieces_add_up(data in data(), cut in any::<prop::sample::Index>()) {
        let cut = cut.index(data.len() + 1);
        let (a, b) = data.split_at(cut);
        let (ha, hb) = (histogram(a), histogram(b));
        let mut sum = [0u64; 256];
        for i in 0..256 {
            sum[i] = ha[i] + hb[i];
        }
        prop_assert_eq!(sum, histogram(&data));
        prop_assert_eq!(entropy_of_histogram(&sum).to_bits(), entropy(&data).to_bits());
    }

    #[test]
    fn window_count_follows_the_formula(len in 0usize..100_000, window in 1usize..5000, step in 1usize..5000) {
        let expected = if len >= window { (len - window) / step + 1 } else { 0 };
        prop_assert_eq!(window_count(len, window, step), Ok(expected));
        let data = vec![0u8; len];
        let it = windows(&data, window, step).unwrap();
        prop_assert_eq!(it.len(), expected);
        prop_assert_eq!(it.count(), expected);
    }

    #[test]
    fn windows_match_entropy_of_each_slice(data in data(), window in 1usize..600, step in 1usize..700) {
        let mut count = 0;
        for (i, w) in windows(&data, window, step).unwrap().enumerate() {
            prop_assert_eq!(w.offset, i * step);
            prop_assert!(w.offset + window <= data.len());
            let expected = entropy(&data[w.offset..w.offset + window]);
            prop_assert_eq!(w.entropy.to_bits(), expected.to_bits(), "offset {}", w.offset);
            count += 1;
        }
        prop_assert_eq!(count, window_count(data.len(), window, step).unwrap());
    }

    #[test]
    fn nth_matches_next(data in data(), window in 1usize..300, step in 1usize..300, skips in prop::collection::vec(0usize..5, 0..20)) {
        let all: Vec<_> = windows(&data, window, step).unwrap().collect();
        let mut it = windows(&data, window, step).unwrap();
        let mut index = 0;
        for skip in skips {
            let got = it.nth(skip);
            index += skip;
            prop_assert_eq!(got, all.get(index).copied());
            index += 1;
            if got.is_none() {
                break;
            }
        }
    }

    #[test]
    fn never_panics_on_any_arguments(data in data(), window in any::<usize>(), step in any::<usize>(), small in 0usize..3) {
        // 0・巨大な値・usize::MAX でも、エラーか正しい数の窓になる
        for (w, s) in [(window, step), (small, step), (window, small), (small, small), (usize::MAX, 1), (1, usize::MAX)] {
            match windows(&data, w, s) {
                Ok(it) => {
                    let n = it.len();
                    prop_assert_eq!(it.count(), n);
                }
                Err(_) => prop_assert!(w == 0 || s == 0),
            }
        }
    }
}
