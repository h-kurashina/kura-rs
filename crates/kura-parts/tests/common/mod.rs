//! file_hash のテストで共通に使う道具。
#![allow(dead_code)]

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use kura_parts::file_hash::{Algorithm, Hasher, hash_bytes, hash_file, hash_reader};

/// BLAKE3 公式テストベクタの入力：0, 1, ..., 250 を繰り返したバイト列。
pub fn cycle251(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// テストが終わると消える一時ファイル。
pub struct TempFile(pub PathBuf);

impl TempFile {
    pub fn with_contents(data: &[u8]) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "kura-file-hash-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, data).expect("write temp file");
        Self(path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// 決められた大きさずつしか返さない Reader（短い読み込み・0 バイトの分割を再現する）。
pub struct ChunkedReader<'a> {
    data: &'a [u8],
    sizes: Vec<usize>,
    next: usize,
    /// true なら、読むたびに1回 Interrupted を返してから読む（シグナルで中断された読み込み）
    pub interrupt: bool,
    interrupted: bool,
}

impl<'a> ChunkedReader<'a> {
    pub fn new(data: &'a [u8], sizes: Vec<usize>) -> Self {
        Self {
            data,
            sizes,
            next: 0,
            interrupt: false,
            interrupted: false,
        }
    }
}

impl Read for ChunkedReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.interrupt && !self.interrupted {
            self.interrupted = true;
            return Err(io::Error::from(io::ErrorKind::Interrupted));
        }
        self.interrupted = false;
        let size = if self.sizes.is_empty() {
            self.data.len()
        } else {
            // 0 を返すと終わりの合図になるので、残りがあるうちは最低 1 バイト返す
            let s = self.sizes[self.next % self.sizes.len()].max(1);
            self.next += 1;
            s
        };
        let n = size.min(buf.len()).min(self.data.len());
        buf[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];
        Ok(n)
    }
}

/// `sizes` の大きさで区切って Hasher に入れる（足りなければ残りを最後にまとめて入れる）。
pub fn hash_in_chunks(algorithm: Algorithm, data: &[u8], sizes: &[usize]) -> String {
    let mut hasher = Hasher::new(algorithm);
    let mut rest = data;
    for &size in sizes {
        let n = size.min(rest.len());
        hasher.update(&rest[..n]);
        rest = &rest[n..];
    }
    hasher.update(rest);
    hasher.finalize().to_hex()
}

/// すべての入口（hash_bytes・Hasher の分割入力・io::Write・hash_reader・hash_file）で同じ答えになるかを見る。
pub fn all_paths(algorithm: Algorithm, data: &[u8], expected: &str) {
    let len = data.len();
    let check = |what: &str, got: String| {
        assert_eq!(got, expected, "{algorithm} {what} (len={len})");
    };
    check("hash_bytes", hash_bytes(algorithm, data).to_hex());
    check("one update", hash_in_chunks(algorithm, data, &[]));
    check("1-byte updates", {
        let mut hasher = Hasher::new(algorithm);
        data.iter().for_each(|b| {
            hasher.update(std::slice::from_ref(b));
        });
        hasher.finalize().to_hex()
    });
    for sizes in [
        vec![0, 1, 0, 63, 64, 65],
        vec![7, 1024, 1],
        vec![4096, 3, 4097],
    ] {
        check("mixed updates", hash_in_chunks(algorithm, data, &sizes));
    }
    check("io::Write", {
        let mut hasher = Hasher::new(algorithm);
        hasher.write_all(data).unwrap();
        hasher.finalize().to_hex()
    });
    check(
        "hash_reader",
        hash_reader(algorithm, data).unwrap().to_hex(),
    );
    let mut reader = ChunkedReader::new(data, vec![1, 5, 64, 1000]);
    reader.interrupt = true;
    check(
        "hash_reader (short, interrupted reads)",
        hash_reader(algorithm, reader).unwrap().to_hex(),
    );
    let file = TempFile::with_contents(data);
    check("hash_file", hash_file(algorithm, &file.0).unwrap().to_hex());
}
