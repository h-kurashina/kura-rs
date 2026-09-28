//! byte_entropy の使い方のテスト：決まった答え（SciPy で出した値）、端の場合、誤った引数。

use kura_parts::byte_entropy::{
    MAX_ENTROPY, Window, WindowError, entropy, entropy_of_histogram, histogram, window_count,
    windows,
};

fn cycle(len: usize, period: usize) -> Vec<u8> {
    (0..len).map(|i| (i % period) as u8).collect()
}

// --- entropy（SciPy 1.18.1 の scipy.stats.entropy(bincount, base=2) で出した値。ビットまで同じ） ---

#[test]
fn matches_scipy_golden_values() {
    let mut all_values_three_times: Vec<u8> = (0..=255u8).cycle().take(768).collect();
    all_values_three_times.push(b'x');
    for (data, expected) in [
        (b"hello world".to_vec(), 2.8453509366224368),
        (b"abc".to_vec(), 1.584962500721156),
        (b"aab".to_vec(), 0.9182958340544894),
        (cycle(1000, 251), 7.970764734653433),
        (
            b"The quick brown fox jumps over the lazy dog".to_vec(),
            4.431965045349459,
        ),
        (all_values_three_times, 7.999718444591623),
    ] {
        let got = entropy(&data);
        assert_eq!(got.to_bits(), f64::to_bits(expected), "{got} != {expected}");
    }
}

#[test]
fn empty_input_is_zero() {
    assert_eq!(entropy(b""), 0.0);
    assert_eq!(entropy_of_histogram(&[0; 256]), 0.0);
}

#[test]
fn a_single_value_is_zero() {
    for b in [0u8, 1, 0x7f, 0xff] {
        for n in [1, 2, 3, 1000, 1 << 16] {
            let e = entropy(&vec![b; n]);
            assert_eq!(e, 0.0);
            assert!(e.is_sign_positive(), "must not be -0.0");
        }
    }
}

#[test]
fn powers_of_two_are_exact() {
    for k in 0..=8u32 {
        let period = 1usize << k;
        for reps in [1, 2, 7, 4096] {
            assert_eq!(
                entropy(&cycle(period * reps, period)),
                f64::from(k),
                "{period} x {reps}"
            );
        }
    }
}

#[test]
fn uniform_256_is_exactly_eight() {
    let data: Vec<u8> = (0..=255u8).cycle().take(256 * 4096).collect();
    assert_eq!(entropy(&data), MAX_ENTROPY);
    assert_eq!(entropy_of_histogram(&[u64::from(u32::MAX); 256]), 8.0);
}

#[test]
fn histogram_counts_every_byte() {
    let data = cycle(1003, 7);
    let h = histogram(&data);
    assert_eq!(h.iter().sum::<u64>(), 1003);
    assert_eq!(h[0], 144);
    assert_eq!(h[6], 143);
    assert_eq!(h[7], 0);
    // どの長さでも（4 バイトずつ数える部分の端数も）数え漏れがない
    for len in 0..20 {
        let data = cycle(len, 3);
        assert_eq!(histogram(&data).iter().sum::<u64>(), len as u64);
    }
}

#[test]
fn histogram_entropy_equals_entropy() {
    let data = b"some bytes \x00\x01\xff and more".repeat(33);
    assert_eq!(entropy_of_histogram(&histogram(&data)), entropy(&data));
}

// --- windows ---

#[test]
fn zero_window_or_step_is_an_error() {
    assert_eq!(windows(b"abc", 0, 1).unwrap_err(), WindowError::ZeroWindow);
    assert_eq!(windows(b"abc", 1, 0).unwrap_err(), WindowError::ZeroStep);
    assert_eq!(windows(b"", 0, 0).unwrap_err(), WindowError::ZeroWindow);
    assert_eq!(window_count(10, 0, 1), Err(WindowError::ZeroWindow));
    assert_eq!(window_count(10, 1, 0), Err(WindowError::ZeroStep));
    assert_eq!(
        WindowError::ZeroStep.to_string(),
        "step must be at least 1 byte"
    );
}

#[test]
fn data_shorter_than_the_window_yields_nothing() {
    assert_eq!(windows(b"abc", 4, 1).unwrap().count(), 0);
    assert_eq!(windows(b"", 1, 1).unwrap().count(), 0);
    assert_eq!(window_count(3, 4, 1), Ok(0));
}

#[test]
fn the_partial_window_at_the_end_is_skipped() {
    let data = cycle(10, 10);
    let got: Vec<usize> = windows(&data, 4, 4).unwrap().map(|w| w.offset).collect();
    assert_eq!(got, [0, 4]); // 8..10 は 2 バイトしかないので含めない
    let got: Vec<usize> = windows(&data, 4, 3).unwrap().map(|w| w.offset).collect();
    assert_eq!(got, [0, 3, 6]);
    let got: Vec<usize> = windows(&data, 10, 1).unwrap().map(|w| w.offset).collect();
    assert_eq!(got, [0]);
}

#[test]
fn step_larger_than_window_leaves_gaps() {
    let data = cycle(100, 256);
    let got: Vec<Window> = windows(&data, 2, 30).unwrap().collect();
    assert_eq!(
        got.iter().map(|w| w.offset).collect::<Vec<_>>(),
        [0, 30, 60, 90]
    );
    assert!(got.iter().all(|w| w.entropy == 1.0));
}

#[test]
fn window_entropy_equals_entropy_of_the_slice() {
    let mut data = cycle(5000, 256);
    data.extend(vec![0u8; 3000]);
    data.extend(b"plain text, plain text, plain text".repeat(100));
    for (window, step) in [
        (1, 1),
        (7, 1),
        (256, 64),
        (512, 512),
        (1000, 999),
        (4096, 1),
    ] {
        let got: Vec<Window> = windows(&data, window, step).unwrap().collect();
        assert_eq!(got.len(), window_count(data.len(), window, step).unwrap());
        for w in got {
            let expected = entropy(&data[w.offset..w.offset + window]);
            assert_eq!(
                w.entropy.to_bits(),
                expected.to_bits(),
                "{window}/{step} at {}",
                w.offset
            );
        }
    }
}

#[test]
fn iterator_is_exact_size_and_supports_nth() {
    let data = cycle(1000, 13);
    let mut it = windows(&data, 100, 10).unwrap();
    assert_eq!(it.len(), 91);
    it.next();
    assert_eq!(it.len(), 90);
    let w = it.nth(5).unwrap();
    assert_eq!(w.offset, 60);
    assert_eq!(w.entropy, entropy(&data[60..160]));
    let w = it.next().unwrap();
    assert_eq!(w.offset, 70);
    assert_eq!(w.entropy, entropy(&data[70..170]));
    assert!(it.nth(1000).is_none());
    assert!(it.next().is_none());
    assert_eq!(it.len(), 0);
}

#[test]
fn a_high_entropy_region_stands_out() {
    // 前後がゼロで、真ん中だけ「暗号化されたような」バイト列（xorshift）
    let mut x = 0x2545_f491_4f6c_dd1du64;
    let mut noise = vec![0u8; 8192];
    for b in &mut noise {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = x as u8;
    }
    let mut data = vec![0u8; 8192];
    data.extend(&noise);
    data.extend(vec![0u8; 8192]);
    let hot: Vec<usize> = windows(&data, 4096, 4096)
        .unwrap()
        .filter(|w| w.entropy > 7.0)
        .map(|w| w.offset)
        .collect();
    assert_eq!(hot, [8192, 12288]);
}
