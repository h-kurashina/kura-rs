//! Permutation parameters, drawn exactly like `numpy.random.RandomState(seed)`
//! (MT19937 with numpy's legacy bounded-integer sampling), so signatures match datasketch.

/// `a` (odd) and `b` for `h -> a * h + b (mod 2^32)`, as datasketch's `affine32` scheme draws them:
/// `a = randint(0, 2**31, n) * 2 + 1`, then `b = randint(0, 2**32, n)`.
pub(super) fn affine32(num_perm: usize, seed: u32) -> (Vec<u32>, Vec<u32>) {
    let mut rng = Mt19937::new(seed);
    let a = (0..num_perm)
        .map(|_| rng.bounded((1 << 31) - 1).wrapping_mul(2).wrapping_add(1))
        .collect();
    let b = (0..num_perm).map(|_| rng.bounded(u32::MAX)).collect();
    (a, b)
}

const N: usize = 624;
const M: usize = 397;

struct Mt19937 {
    state: [u32; N],
    index: usize,
}

impl Mt19937 {
    /// numpy's legacy seeding for an integer seed (`init_genrand`).
    fn new(seed: u32) -> Self {
        let mut state = [0u32; N];
        state[0] = seed;
        for i in 1..N {
            let prev = state[i - 1];
            state[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: N }
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    fn twist(&mut self) {
        for i in 0..N {
            let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % N] & 0x7fff_ffff);
            let mag = if y & 1 == 0 { 0 } else { 0x9908_b0df };
            self.state[i] = self.state[(i + M) % N] ^ (y >> 1) ^ mag;
        }
        self.index = 0;
    }

    /// A uniform integer in `0..=max`, sampled like numpy's legacy `randint` for 32-bit dtypes:
    /// the full range is returned as-is, otherwise draws are masked and rejected until they fit.
    fn bounded(&mut self, max: u32) -> u32 {
        // numpy と同じく、幅が 0 なら乱数を使わずに 0 を返す（mask の計算で 32 ビットずらしてしまうのも防ぐ）
        if max == 0 {
            return 0;
        }
        if max == u32::MAX {
            return self.next_u32();
        }
        let mask = u32::MAX >> max.leading_zeros();
        loop {
            let value = self.next_u32() & mask;
            if value <= max {
                return value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mt19937;

    fn draws(seed: u32, max: u32, n: usize) -> Vec<u32> {
        let mut rng = Mt19937::new(seed);
        (0..n).map(|_| rng.bounded(max)).collect()
    }

    #[test]
    fn matches_the_reference_mt19937() {
        // MT19937 の公式の検証値（seed 5489）。numpy の RandomState(5489) とも同じ
        assert_eq!(
            draws(5489, u32::MAX, 5),
            [3499211612, 581869302, 3890346734, 3586334585, 545404204]
        );
    }

    #[test]
    fn matches_numpy_randint() {
        // RandomState(0).randint(0, 2**31, 5, dtype=uint32)
        assert_eq!(
            draws(0, (1 << 31) - 1, 5),
            [209652396, 398764591, 924231285, 1478610112, 441365315]
        );
        // RandomState(2**32 - 1).randint(0, 2**32, 3, dtype=uint32)
        assert_eq!(
            draws(u32::MAX, u32::MAX, 3),
            [419326371, 479346978, 3918654476]
        );
        // RandomState(7).randint(0, 5, 8, dtype=uint32): 捨てて引き直す（rejection）経路
        assert_eq!(draws(7, 4, 8), [4, 1, 3, 3, 4, 1, 0, 1]);
    }

    #[test]
    fn matches_numpy_across_a_twist() {
        // 624 個を使い切って内部状態を作り直した直後の値
        let values = draws(1, u32::MAX, 627);
        assert_eq!(
            values[623..627],
            [2006116153, 1104314680, 939235918, 476274519]
        );
    }

    #[test]
    fn bounded_never_exceeds_max() {
        for max in [0, 1, 2, 3, 5, 100, (1 << 31) - 1, u32::MAX - 1] {
            assert!(
                draws(42, max, 2000).into_iter().all(|v| v <= max),
                "max = {max}"
            );
        }
    }

    #[test]
    fn zero_width_range_returns_zero_without_drawing() {
        // numpy の RandomState(3).randint(0, 1, 4) は乱数を消費せずに 0 を返す。次の値がずれないことも確かめる
        let mut rng = Mt19937::new(3);
        assert_eq!([rng.bounded(0), rng.bounded(0)], [0, 0]);
        assert_eq!(rng.bounded(u32::MAX), Mt19937::new(3).bounded(u32::MAX));
    }

    #[test]
    fn a_is_always_odd() {
        for seed in [0, 1, 42, u32::MAX] {
            let (a, _) = super::affine32(512, seed);
            assert!(a.iter().all(|x| x % 2 == 1), "seed = {seed}");
        }
    }
}
