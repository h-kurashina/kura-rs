//! file_hash の性質テスト。ランダムな入力を大量に作り、どんな入力でも成り立つべき性質を確かめる。
//! 1つの性質につき既定で 2,000 通り。PROPTEST_CASES=100000 のように環境変数で増やせる。

mod common;

use common::{ChunkedReader, TempFile, hash_in_chunks};
use kura_parts::file_hash::{Algorithm, Digest, Hasher, hash_bytes, hash_file, hash_reader};
use proptest::prelude::*;
use sha2::Digest as _;

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

/// ファイルを作る性質は重いので、回数を 1/10 にする
fn file_config() -> ProptestConfig {
    let mut c = config();
    c.cases = (c.cases / 10).max(50);
    c
}

fn algorithm() -> impl Strategy<Value = Algorithm> {
    prop_oneof![Just(Algorithm::Sha256), Just(Algorithm::Blake3)]
}

/// 空・短い・ブロック境目付近・数十 KiB までの任意のバイト列
fn data() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        4 => prop::collection::vec(any::<u8>(), 0..200),
        3 => prop::collection::vec(any::<u8>(), 0..5000),
        1 => prop::collection::vec(any::<u8>(), 0..70_000),
        1 => (0usize..5000, any::<u8>()).prop_map(|(n, b)| vec![b; n]),
    ]
}

/// 分割の大きさ（0 バイトの分割やブロックの境目の値を含む）
fn chunk_sizes() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(
        prop_oneof![
            0usize..3,
            0usize..200,
            prop::sample::select(vec![63usize, 64, 65, 127, 128, 129, 1023, 1024, 1025, 4096]),
            0usize..20_000,
        ],
        0..30,
    )
}

fn reference(alg: Algorithm, data: &[u8]) -> [u8; 32] {
    // 部品を通さずに、元の crate を直接呼んだ答え
    match alg {
        Algorithm::Sha256 => sha2::Sha256::digest(data).into(),
        Algorithm::Blake3 => *blake3::hash(data).as_bytes(),
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn chunking_does_not_matter(alg in algorithm(), data in data(), sizes in chunk_sizes()) {
        prop_assert_eq!(hash_in_chunks(alg, &data, &sizes), hash_bytes(alg, &data).to_hex());
    }

    #[test]
    fn short_reads_do_not_matter(alg in algorithm(), data in data(), sizes in chunk_sizes()) {
        let reader = ChunkedReader::new(&data, sizes);
        prop_assert_eq!(hash_reader(alg, reader).unwrap(), hash_bytes(alg, &data));
    }

    #[test]
    fn interrupted_reads_do_not_matter(alg in algorithm(), data in data(), sizes in chunk_sizes()) {
        let mut reader = ChunkedReader::new(&data, sizes);
        reader.interrupt = true;
        prop_assert_eq!(hash_reader(alg, reader).unwrap(), hash_bytes(alg, &data));
    }

    #[test]
    fn matches_the_underlying_crates(alg in algorithm(), data in data()) {
        prop_assert_eq!(*hash_bytes(alg, &data).as_bytes(), reference(alg, &data));
    }

    #[test]
    fn digest_is_32_bytes_and_64_lowercase_hex(alg in algorithm(), data in data()) {
        let d = hash_bytes(alg, &data);
        let hex = d.to_hex();
        prop_assert_eq!(d.as_bytes().len(), 32);
        prop_assert_eq!(hex.len(), 64);
        prop_assert!(hex.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
        prop_assert_eq!(d.to_string(), hex);
    }

    #[test]
    fn hex_round_trips(alg in algorithm(), bytes in any::<[u8; 32]>(), upper in any::<bool>()) {
        let d = Digest::new(alg, bytes);
        let hex = if upper { d.to_hex().to_uppercase() } else { d.to_hex() };
        prop_assert_eq!(Digest::from_hex(alg, &hex).unwrap(), d);
    }

    #[test]
    fn from_hex_never_panics(alg in algorithm(), s in ".{0,80}") {
        // どんな文字列でも落ちずに、Ok なら 64 桁の16進だった
        if let Ok(d) = Digest::from_hex(alg, &s) {
            prop_assert_eq!(d.to_hex(), s.to_lowercase());
        }
    }

    #[test]
    fn from_hex_rejects_any_wrong_length(alg in algorithm(), len in (0usize..200).prop_filter("not 64", |&n| n != 64)) {
        prop_assert!(Digest::from_hex(alg, &"a".repeat(len)).is_err());
    }

    #[test]
    fn from_hex_rejects_one_bad_character(alg in algorithm(), pos in 0usize..64, bad in any::<char>().prop_filter("not hex", |c| !c.is_ascii_hexdigit())) {
        let mut chars: Vec<char> = "0".repeat(64).chars().collect();
        chars[pos] = bad;
        let s: String = chars.into_iter().collect();
        prop_assert!(Digest::from_hex(alg, &s).is_err());
    }

    #[test]
    fn equality_matches_bytes(alg in algorithm(), a in any::<[u8; 32]>(), b in any::<[u8; 32]>()) {
        prop_assert_eq!(Digest::new(alg, a) == Digest::new(alg, b), a == b);
        prop_assert_eq!(Digest::new(alg, a), Digest::new(alg, a));
    }

    #[test]
    fn a_flipped_bit_changes_the_digest(alg in algorithm(), data in data().prop_filter("non-empty", |d| !d.is_empty()), pos in any::<prop::sample::Index>(), bit in 0u8..8) {
        let mut tampered = data.clone();
        tampered[pos.index(data.len())] ^= 1 << bit;
        prop_assert_ne!(hash_bytes(alg, &tampered), hash_bytes(alg, &data));
    }

    #[test]
    fn appending_changes_the_digest(alg in algorithm(), data in data(), extra in prop::collection::vec(any::<u8>(), 1..100)) {
        let mut longer = data.clone();
        longer.extend_from_slice(&extra);
        prop_assert_ne!(hash_bytes(alg, &longer), hash_bytes(alg, &data));
    }

    #[test]
    fn finalize_midway_then_continue(alg in algorithm(), a in data(), b in data()) {
        let mut hasher = Hasher::new(alg);
        hasher.update(&a);
        prop_assert_eq!(hasher.finalize(), hash_bytes(alg, &a));
        hasher.update(&b);
        prop_assert_eq!(hasher.finalize(), hash_bytes(alg, &[a, b].concat()));
    }

    #[test]
    fn clone_forks_the_state(alg in algorithm(), prefix in data(), x in data(), y in data()) {
        let mut a = Hasher::new(alg);
        a.update(&prefix);
        let mut b = a.clone();
        a.update(&x);
        b.update(&y);
        prop_assert_eq!(a.finalize(), hash_bytes(alg, &[prefix.clone(), x].concat()));
        prop_assert_eq!(b.finalize(), hash_bytes(alg, &[prefix, y].concat()));
    }

    #[test]
    fn reset_is_a_fresh_hasher(alg in algorithm(), junk in data(), data in data()) {
        let mut hasher = Hasher::new(alg);
        hasher.update(&junk);
        hasher.reset();
        hasher.update(&data);
        prop_assert_eq!(hasher.finalize(), hash_bytes(alg, &data));
    }

    #[test]
    fn algorithms_never_agree(data in data()) {
        prop_assert_ne!(
            *hash_bytes(Algorithm::Sha256, &data).as_bytes(),
            *hash_bytes(Algorithm::Blake3, &data).as_bytes()
        );
    }
}

proptest! {
    #![proptest_config(file_config())]

    #[test]
    fn hash_file_equals_hash_bytes(alg in algorithm(), data in data()) {
        let file = TempFile::with_contents(&data);
        prop_assert_eq!(hash_file(alg, &file.0).unwrap(), hash_bytes(alg, &data));
    }
}
