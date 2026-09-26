//! file_hash の公式テストベクタとの照合（Python がなくても CI で走る分）。
//! たくさんの入力での突き合わせは verify/run.py file-hash が行う。
//!
//! 出典:
//! - SHA-256: NIST FIPS 180-4 の例（"abc"、448 ビット・896 ビットの文字列、'a' を 100 万回。NIST
//!   "Examples with Intermediate Values" の SHA256.pdf / SHA2_Additional.pdf）と、NIST CAVP の
//!   SHAVS 用ファイル SHA256ShortMsg.rsp（バイト単位の短い入力）の Len=8〜32 の 4 件。
//! - BLAKE3: 公式リポジトリの test_vectors/test_vectors.json
//!   (https://github.com/BLAKE3-team/BLAKE3/blob/master/test_vectors/test_vectors.json)。
//!   hash モードの全 35 件について、出力の先頭 32 バイト（既定の長さ）を載せている。
//!   入力は 0, 1, ..., 250 を繰り返したバイト列。

mod common;

use common::{all_paths, cycle251};
use kura_parts::file_hash::{Algorithm, Hasher, hash_bytes};

/// (入力, SHA-256 の16進)
const SHA256_FIPS: &[(&[u8], &str)] = &[
    (
        b"",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        b"abc",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    ),
    (
        // 448 ビット（56 バイト）：長さの欄が入らず、パディングが次のブロックにはみ出す境目
        b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    ),
    (
        // 896 ビット（112 バイト）
        b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
    ),
];

/// NIST CAVP SHA256ShortMsg.rsp より（Msg は16進、Len=8, 16, 24, 32 ビット）
const SHA256_CAVP_SHORT: &[(&str, &str)] = &[
    (
        "d3",
        "28969cdfa74a12c82f3bad960b0b000aca2ac329deea5c2328ebc6f2ba9802c1",
    ),
    (
        "11af",
        "5ca7133fa735326081558ac312c620eeca9970d1e70a4b95533d956f072d1f98",
    ),
    (
        "b4190e",
        "dff2e73091f6c05e528896c4c831b9448653dc2ff043528f6769437bc7b975c2",
    ),
    (
        "74ba2521",
        "b16aa56be3880d18cd41e68384cf1ec8c17680c45a02b1575dc1518923ae8b0e",
    ),
];

/// BLAKE3 公式 test_vectors.json の hash（先頭 32 バイト）。(入力の長さ, 16進)
const BLAKE3_OFFICIAL: &[(usize, &str)] = &[
    (
        0,
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
    ),
    (
        1,
        "2d3adedff11b61f14c886e35afa036736dcd87a74d27b5c1510225d0f592e213",
    ),
    (
        2,
        "7b7015bb92cf0b318037702a6cdd81dee41224f734684c2c122cd6359cb1ee63",
    ),
    (
        3,
        "e1be4d7a8ab5560aa4199eea339849ba8e293d55ca0a81006726d184519e647f",
    ),
    (
        4,
        "f30f5ab28fe047904037f77b6da4fea1e27241c5d132638d8bedce9d40494f32",
    ),
    (
        5,
        "b40b44dfd97e7a84a996a91af8b85188c66c126940ba7aad2e7ae6b385402aa2",
    ),
    (
        6,
        "06c4e8ffb6872fad96f9aaca5eee1553eb62aed0ad7198cef42e87f6a616c844",
    ),
    (
        7,
        "3f8770f387faad08faa9d8414e9f449ac68e6ff0417f673f602a646a891419fe",
    ),
    (
        8,
        "2351207d04fc16ade43ccab08600939c7c1fa70a5c0aaca76063d04c3228eaeb",
    ),
    (
        63,
        "e9bc37a594daad83be9470df7f7b3798297c3d834ce80ba85d6e207627b7db7b",
    ),
    (
        64,
        "4eed7141ea4a5cd4b788606bd23f46e212af9cacebacdc7d1f4c6dc7f2511b98",
    ),
    (
        65,
        "de1e5fa0be70df6d2be8fffd0e99ceaa8eb6e8c93a63f2d8d1c30ecb6b263dee",
    ),
    (
        127,
        "d81293fda863f008c09e92fc382a81f5a0b4a1251cba1634016a0f86a6bd640d",
    ),
    (
        128,
        "f17e570564b26578c33bb7f44643f539624b05df1a76c81f30acd548c44b45ef",
    ),
    (
        129,
        "683aaae9f3c5ba37eaaf072aed0f9e30bac0865137bae68b1fde4ca2aebdcb12",
    ),
    (
        1023,
        "10108970eeda3eb932baac1428c7a2163b0e924c9a9e25b35bba72b28f70bd11",
    ),
    (
        1024,
        "42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7",
    ),
    (
        1025,
        "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444",
    ),
    (
        2048,
        "e776b6028c7cd22a4d0ba182a8bf62205d2ef576467e838ed6f2529b85fba24a",
    ),
    (
        2049,
        "5f4d72f40d7a5f82b15ca2b2e44b1de3c2ef86c426c95c1af0b6879522563030",
    ),
    (
        3072,
        "b98cb0ff3623be03326b373de6b9095218513e64f1ee2edd2525c7ad1e5cffd2",
    ),
    (
        3073,
        "7124b49501012f81cc7f11ca069ec9226cecb8a2c850cfe644e327d22d3e1cd3",
    ),
    (
        4096,
        "015094013f57a5277b59d8475c0501042c0b642e531b0a1c8f58d2163229e969",
    ),
    (
        4097,
        "9b4052b38f1c5fc8b1f9ff7ac7b27cd242487b3d890d15c96a1c25b8aa0fb995",
    ),
    (
        5120,
        "9cadc15fed8b5d854562b26a9536d9707cadeda9b143978f319ab34230535833",
    ),
    (
        5121,
        "628bd2cb2004694adaab7bbd778a25df25c47b9d4155a55f8fbd79f2fe154cff",
    ),
    (
        6144,
        "3e2e5b74e048f3add6d21faab3f83aa44d3b2278afb83b80b3c35164ebeca205",
    ),
    (
        6145,
        "f1323a8631446cc50536a9f705ee5cb619424d46887f3c376c695b70e0f0507f",
    ),
    (
        7168,
        "61da957ec2499a95d6b8023e2b0e604ec7f6b50e80a9678b89d2628e99ada77a",
    ),
    (
        7169,
        "a003fc7a51754a9b3c7fae0367ab3d782dccf28855a03d435f8cfe74605e7817",
    ),
    (
        8192,
        "aae792484c8efe4f19e2ca7d371d8c467ffb10748d8a5a1ae579948f718a2a63",
    ),
    (
        8193,
        "bab6c09cb8ce8cf459261398d2e7aef35700bf488116ceb94a36d0f5f1b7bc3b",
    ),
    (
        16384,
        "f875d6646de28985646f34ee13be9a576fd515f76b5b0a26bb324735041ddde4",
    ),
    (
        31744,
        "62b6960e1a44bcc1eb1a611a8d6235b6b4b78f32e7abc4fb4c6cdcce94895c47",
    ),
    (
        102400,
        "bc3e3d41a1146b069abffad3c0d44860cf664390afce4d9661f7902e7943e085",
    ),
];

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn sha256_fips_180_examples() {
    for &(input, expected) in SHA256_FIPS {
        all_paths(Algorithm::Sha256, input, expected);
    }
}

#[test]
fn sha256_cavp_short_messages() {
    for &(msg, expected) in SHA256_CAVP_SHORT {
        all_paths(Algorithm::Sha256, &unhex(msg), expected);
    }
}

#[test]
fn sha256_one_million_a() {
    // FIPS 180 の例：'a' を 1,000,000 回
    all_paths(
        Algorithm::Sha256,
        &vec![b'a'; 1_000_000],
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
    );
}

#[test]
#[ignore = "1 GiB を入れるので遅い。CI では cargo test --release -- --ignored で回す"]
fn sha256_extremely_long_message() {
    // NIST SHA2_Additional の "extremely long message"：64 バイトの文字列を 16,777,216 回（1 GiB）
    let block = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmno";
    let chunk = block.repeat(1 << 14); // 1 MiB
    let mut hasher = Hasher::new(Algorithm::Sha256);
    for _ in 0..(1 << 10) {
        hasher.update(&chunk);
    }
    assert_eq!(
        hasher.finalize().to_hex(),
        "50e72a0e26442fe2552dc3938ac58658228c0cbfb1d2ca872ae435266fcd055e"
    );
}

#[test]
fn blake3_official_test_vectors() {
    assert_eq!(BLAKE3_OFFICIAL.len(), 35);
    for &(len, expected) in BLAKE3_OFFICIAL {
        all_paths(Algorithm::Blake3, &cycle251(len), expected);
    }
}

#[test]
fn blake3_million_a_matches_python_blake3() {
    // Python の blake3 1.0.9 で求めた値：blake3(b"a" * 1_000_000).hexdigest()
    assert_eq!(
        hash_bytes(Algorithm::Blake3, &vec![b'a'; 1_000_000]).to_hex(),
        BLAKE3_MILLION_A
    );
}

const BLAKE3_MILLION_A: &str = "616f575a1b58d4c9797d4217b9730ae5e6eb319d76edef6549b46f4efe31ff8b";
