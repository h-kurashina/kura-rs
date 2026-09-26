//! file_hash の使い方のテスト：誤った入力はエラーになり、落ちないこと。

mod common;

use std::collections::HashSet;
use std::io::{self, Read};

use common::{ChunkedReader, TempFile};
use kura_parts::file_hash::{
    Algorithm, DIGEST_LEN, Digest, Hasher, READ_BUFFER_SIZE, hash_bytes, hash_file, hash_reader,
};

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

// --- Algorithm ---

#[test]
fn algorithm_names() {
    assert_eq!(Algorithm::Sha256.name(), "sha256");
    assert_eq!(Algorithm::Blake3.name(), "blake3");
    assert_eq!(Algorithm::Sha256.to_string(), "sha256");
    assert_eq!(Algorithm::ALL, [Algorithm::Sha256, Algorithm::Blake3]);
}

#[test]
fn algorithm_parses_known_names_in_any_case() {
    for (s, expected) in [
        ("sha256", Algorithm::Sha256),
        ("SHA256", Algorithm::Sha256),
        ("sha-256", Algorithm::Sha256),
        ("SHA-256", Algorithm::Sha256),
        ("blake3", Algorithm::Blake3),
        ("BLAKE3", Algorithm::Blake3),
        ("Blake3", Algorithm::Blake3),
    ] {
        assert_eq!(s.parse::<Algorithm>().unwrap(), expected, "{s}");
    }
}

#[test]
fn algorithm_rejects_unknown_names() {
    for s in [
        "",
        "sha1",
        "md5",
        "sha512",
        "sha_256",
        "sha 256",
        " sha256",
        "sha256 ",
        "blake2b",
        "blake",
        "sha2",
        "ｓｈａ２５６",
    ] {
        let err = s.parse::<Algorithm>().unwrap_err();
        assert!(err.to_string().contains("unknown algorithm"), "{s}: {err}");
    }
}

#[test]
fn algorithm_name_round_trips() {
    for alg in Algorithm::ALL {
        assert_eq!(alg.name().parse::<Algorithm>().unwrap(), alg);
    }
}

// --- Digest ---

#[test]
fn digest_hex_and_bytes() {
    let d = hash_bytes(Algorithm::Sha256, b"abc");
    assert_eq!(d.to_hex(), ABC_SHA256);
    assert_eq!(d.to_string(), ABC_SHA256);
    assert_eq!(d.as_bytes().len(), DIGEST_LEN);
    assert_eq!(d.as_ref(), d.as_bytes());
    assert_eq!(d.as_bytes()[0], 0xba);
    assert_eq!(d.as_bytes()[31], 0xad);
    assert_eq!(d.algorithm(), Algorithm::Sha256);
    assert_eq!(format!("{d:?}"), format!("Digest(sha256:{ABC_SHA256})"));
}

#[test]
fn digest_from_hex_accepts_upper_and_lower_case() {
    let lower = Digest::from_hex(Algorithm::Sha256, ABC_SHA256).unwrap();
    let upper = Digest::from_hex(Algorithm::Sha256, &ABC_SHA256.to_uppercase()).unwrap();
    assert_eq!(lower, upper);
    assert_eq!(lower, hash_bytes(Algorithm::Sha256, b"abc"));
}

#[test]
fn digest_from_hex_rejects_bad_input() {
    let bad = [
        String::new(),
        ABC_SHA256[..63].to_string(),
        format!("{ABC_SHA256}0"),
        format!("{ABC_SHA256}00"),
        format!("g{}", &ABC_SHA256[1..]),
        format!("{} ", &ABC_SHA256[..63]),
        format!("0x{}", &ABC_SHA256[..62]),
        format!("-{}", &ABC_SHA256[1..]),
        format!("+{}", &ABC_SHA256[1..]),
        // 全角や多バイト文字（バイト数では 64 になりうる）
        format!("é{}", &ABC_SHA256[2..]),
        "ｆ".repeat(21) + "a",
    ];
    for s in bad {
        assert!(
            Digest::from_hex(Algorithm::Sha256, &s).is_err(),
            "{s:?} was accepted"
        );
    }
}

#[test]
fn digest_new_round_trips_bytes() {
    let d = hash_bytes(Algorithm::Blake3, b"x");
    assert_eq!(Digest::new(Algorithm::Blake3, *d.as_bytes()), d);
}

#[test]
fn digests_of_different_algorithms_are_not_equal() {
    let bytes = *hash_bytes(Algorithm::Sha256, b"abc").as_bytes();
    // 同じ 32 バイトでも、アルゴリズムが違えば別物として扱う
    assert_ne!(
        Digest::new(Algorithm::Sha256, bytes),
        Digest::new(Algorithm::Blake3, bytes)
    );
}

#[test]
fn digest_equality_detects_every_single_bit() {
    let d = hash_bytes(Algorithm::Sha256, b"abc");
    for i in 0..DIGEST_LEN {
        for bit in 0..8 {
            let mut bytes = *d.as_bytes();
            bytes[i] ^= 1 << bit;
            assert_ne!(
                Digest::new(Algorithm::Sha256, bytes),
                d,
                "byte {i} bit {bit}"
            );
        }
    }
}

#[test]
fn digests_work_as_hash_set_keys() {
    // 完全一致の重複検出：同じ中身は1つにまとまる
    let files: [&[u8]; 5] = [b"a", b"b", b"a", b"", b""];
    let set: HashSet<Digest> = files
        .iter()
        .map(|f| hash_bytes(Algorithm::Blake3, f))
        .collect();
    assert_eq!(set.len(), 3);
}

#[test]
fn sha256_and_blake3_differ() {
    for data in [&b""[..], b"a", b"abc"] {
        assert_ne!(
            hash_bytes(Algorithm::Sha256, data).as_bytes(),
            hash_bytes(Algorithm::Blake3, data).as_bytes()
        );
    }
}

// --- Hasher ---

#[test]
fn update_can_be_chained() {
    let mut hasher = Hasher::new(Algorithm::Sha256);
    hasher.update(b"a").update(b"").update(b"bc");
    assert_eq!(hasher.finalize().to_hex(), ABC_SHA256);
    assert_eq!(hasher.algorithm(), Algorithm::Sha256);
}

#[test]
fn finalize_does_not_consume_and_input_can_continue() {
    for alg in Algorithm::ALL {
        let mut hasher = Hasher::new(alg);
        hasher.update(b"ab");
        let first = hasher.finalize();
        assert_eq!(first, hasher.finalize(), "finalize is repeatable");
        assert_eq!(first, hash_bytes(alg, b"ab"));
        hasher.update(b"c");
        assert_eq!(hasher.finalize(), hash_bytes(alg, b"abc"));
    }
}

#[test]
fn reset_forgets_input() {
    for alg in Algorithm::ALL {
        let mut hasher = Hasher::new(alg);
        hasher.update(b"garbage");
        hasher.reset();
        assert_eq!(hasher.algorithm(), alg);
        hasher.update(b"abc");
        assert_eq!(hasher.finalize(), hash_bytes(alg, b"abc"));
    }
}

#[test]
fn cloned_hasher_is_independent() {
    for alg in Algorithm::ALL {
        let mut a = Hasher::new(alg);
        a.update(b"prefix-");
        let mut b = a.clone();
        a.update(b"one");
        b.update(b"two");
        assert_eq!(a.finalize(), hash_bytes(alg, b"prefix-one"));
        assert_eq!(b.finalize(), hash_bytes(alg, b"prefix-two"));
    }
}

#[test]
fn hasher_debug_does_not_leak_state() {
    let mut hasher = Hasher::new(Algorithm::Blake3);
    hasher.update(b"secret");
    let debug = format!("{hasher:?}");
    assert!(debug.contains("Blake3"), "{debug}");
    assert!(!debug.contains("secret"), "{debug}");
}

#[test]
fn io_copy_into_hasher() {
    for alg in Algorithm::ALL {
        let data = common::cycle251(200_000);
        let mut hasher = Hasher::new(alg);
        let copied = io::copy(&mut data.as_slice(), &mut hasher).unwrap();
        assert_eq!(copied, 200_000);
        assert_eq!(hasher.finalize(), hash_bytes(alg, &data));
    }
}

#[test]
fn update_reader_returns_the_byte_count() {
    let data = common::cycle251(3 * READ_BUFFER_SIZE + 17);
    let mut hasher = Hasher::new(Algorithm::Sha256);
    let n = hasher.update_reader(data.as_slice()).unwrap();
    assert_eq!(n, data.len() as u64);
    assert_eq!(hasher.finalize(), hash_bytes(Algorithm::Sha256, &data));
}

// --- hash_reader ---

/// 途中で失敗する Reader
struct FailingReader {
    good: usize,
    kind: io::ErrorKind,
}

impl Read for FailingReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.good == 0 {
            return Err(io::Error::new(self.kind, "disk on fire"));
        }
        let n = self.good.min(buf.len());
        buf[..n].fill(7);
        self.good -= n;
        Ok(n)
    }
}

#[test]
fn hash_reader_propagates_read_errors() {
    for kind in [
        io::ErrorKind::Other,
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::UnexpectedEof,
        io::ErrorKind::InvalidData,
    ] {
        for good in [0, 1, READ_BUFFER_SIZE, READ_BUFFER_SIZE * 2 + 5] {
            let err = hash_reader(Algorithm::Sha256, FailingReader { good, kind }).unwrap_err();
            assert_eq!(err.kind(), kind);
            assert_eq!(err.to_string(), "disk on fire");
        }
    }
}

#[test]
fn hash_reader_retries_interrupted_reads() {
    let data = common::cycle251(100_000);
    for alg in Algorithm::ALL {
        let mut reader = ChunkedReader::new(&data, vec![1, 999, 65536, 3]);
        reader.interrupt = true;
        assert_eq!(
            hash_reader(alg, reader).unwrap(),
            hash_bytes(alg, &data),
            "{alg}"
        );
    }
}

#[test]
fn hash_reader_of_empty_reader() {
    for alg in Algorithm::ALL {
        assert_eq!(hash_reader(alg, io::empty()).unwrap(), hash_bytes(alg, b""));
    }
}

#[test]
fn hash_reader_across_buffer_boundaries() {
    // 読み込み用バッファ（64 KiB）の境目の前後
    for len in [
        READ_BUFFER_SIZE - 1,
        READ_BUFFER_SIZE,
        READ_BUFFER_SIZE + 1,
        2 * READ_BUFFER_SIZE,
        2 * READ_BUFFER_SIZE + 1,
    ] {
        let data = common::cycle251(len);
        for alg in Algorithm::ALL {
            assert_eq!(
                hash_reader(alg, data.as_slice()).unwrap(),
                hash_bytes(alg, &data),
                "{alg} len={len}"
            );
        }
    }
}

// --- hash_file ---

#[test]
fn hash_file_matches_hash_bytes() {
    for len in [0, 1, 64, 1024, READ_BUFFER_SIZE + 1, 1 << 20] {
        let data = common::cycle251(len);
        let file = TempFile::with_contents(&data);
        for alg in Algorithm::ALL {
            assert_eq!(
                hash_file(alg, &file.0).unwrap(),
                hash_bytes(alg, &data),
                "{alg} len={len}"
            );
        }
    }
}

#[test]
fn hash_file_accepts_str_string_and_path() {
    let file = TempFile::with_contents(b"abc");
    let as_str: &str = file.0.to_str().unwrap();
    let expected = hash_bytes(Algorithm::Sha256, b"abc");
    assert_eq!(hash_file(Algorithm::Sha256, as_str).unwrap(), expected);
    assert_eq!(
        hash_file(Algorithm::Sha256, String::from(as_str)).unwrap(),
        expected
    );
    assert_eq!(
        hash_file(Algorithm::Sha256, file.0.as_path()).unwrap(),
        expected
    );
}

#[test]
fn hash_file_missing_file_is_not_found() {
    let path = std::env::temp_dir().join("kura-file-hash-does-not-exist-9f2c1e");
    let err = hash_file(Algorithm::Sha256, &path).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}

#[test]
fn hash_file_on_a_directory_is_an_error() {
    // ディレクトリは開けても読めない（あるいは開けない）ので、どちらにしてもエラーになる
    assert!(hash_file(Algorithm::Blake3, std::env::temp_dir()).is_err());
}

#[test]
fn hash_file_sees_the_current_contents() {
    let file = TempFile::with_contents(b"version 1");
    let before = hash_file(Algorithm::Sha256, &file.0).unwrap();
    std::fs::write(&file.0, b"version 2").unwrap();
    let after = hash_file(Algorithm::Sha256, &file.0).unwrap();
    assert_ne!(before, after, "a changed file must give a different digest");
    assert_eq!(after, hash_bytes(Algorithm::Sha256, b"version 2"));
}

#[test]
fn one_flipped_bit_in_a_file_changes_the_digest() {
    // 改ざん検出：1 ビットでも変われば必ず別の値になる
    let data = common::cycle251(10_000);
    let original = TempFile::with_contents(&data);
    for alg in Algorithm::ALL {
        let expected = hash_file(alg, &original.0).unwrap();
        for pos in [0, 1, 63, 64, 1023, 1024, 5000, 9999] {
            let mut tampered = data.clone();
            tampered[pos] ^= 0x01;
            let file = TempFile::with_contents(&tampered);
            assert_ne!(
                hash_file(alg, &file.0).unwrap(),
                expected,
                "{alg} pos={pos}"
            );
        }
    }
}

#[test]
fn truncated_or_extended_files_change_the_digest() {
    let data = common::cycle251(4096);
    for alg in Algorithm::ALL {
        let expected = hash_bytes(alg, &data);
        assert_ne!(hash_bytes(alg, &data[..4095]), expected);
        let mut longer = data.clone();
        longer.push(0);
        assert_ne!(hash_bytes(alg, &longer), expected);
    }
}

#[test]
fn every_length_up_to_300_bytes_differs_from_its_neighbours() {
    // 長さ違いの全ゼロ入力（パディングの境目が集まるところ）がどれも別の値になる
    for alg in Algorithm::ALL {
        let digests: HashSet<Digest> = (0..=300).map(|n| hash_bytes(alg, &vec![0; n])).collect();
        assert_eq!(digests.len(), 301, "{alg}");
    }
}
