//! multi-pattern-match の差分テスト（verify）とベンチマーク（bench）。
//!
//! verify: 標準入力の各ケースについて、pyahocorasick の Automaton.iter()（重なりあり・すべて）と
//!         iter_long()（最左最長・重なりなし）の答えと、並び順まで完全に一致するかを見る。
//!         答えは (一致の最後のバイトの位置, パターンの番号) の列（pyahocorasick の iter() と同じ形）。
//! bench:  IOC 風のパターン（ドメイン・IP・ハッシュ・パス）とアクセスログ風の入力を作り、
//!         入力の大きさごとに、すべての一致（重なりあり）を集める時間を計る。
//!         作ったパターンと入力は target/verify-data/multi-pattern-match/ に書き、Python 側も同じものを読む。

use std::hint::black_box;
use std::path::PathBuf;

use kura_parts::multi_pattern_match::{Match, Matcher};
use kura_verify::{median_ms, print_json, run_cases};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Case {
    patterns: Vec<String>,
    haystack: Haystack,
    /// pyahocorasick の iter() の答え：[末尾の位置, パターンの番号] の列
    expected: Vec<[usize; 2]>,
    /// pyahocorasick の iter_long() の答え
    expected_long: Vec<[usize; 2]>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Haystack {
    Hex {
        hex: String,
    },
    /// hex の並びを count 回繰り返し、tail を後ろに付けたもの
    Repeat {
        hex: String,
        count: usize,
        tail: String,
    },
}

#[derive(Serialize)]
struct BenchPoint {
    input_size: u64,
    rust_ms: f64,
    matches: usize,
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("verify") => print_json(&run_cases(check)),
        Some("bench") => bench(),
        _ => {
            eprintln!(
                "usage: multi-pattern-match verify < cases.jsonl | multi-pattern-match bench <patterns> <size>..."
            );
            std::process::exit(2);
        }
    }
}

fn check(case: &Case) -> Result<(), String> {
    let patterns = case
        .patterns
        .iter()
        .map(|p| unhex(p))
        .collect::<Result<Vec<_>, _>>()?;
    let haystack = match &case.haystack {
        Haystack::Hex { hex } => unhex(hex)?,
        Haystack::Repeat { hex, count, tail } => {
            let mut data = unhex(hex)?.repeat(*count);
            data.extend(unhex(tail)?);
            data
        }
    };
    let matcher = Matcher::new(&patterns).map_err(|e| e.to_string())?;
    let what = format!("{} patterns, {} bytes", patterns.len(), haystack.len());

    let overlapping = matcher.find_overlapping(&haystack);
    compare("find_overlapping", &overlapping, &case.expected, &what)?;
    compare(
        "find_longest",
        &matcher.find_longest(&haystack),
        &case.expected_long,
        &what,
    )?;
    // 報告した位置に、報告したパターンがそのままあるか
    for m in &overlapping {
        if haystack.get(m.start..m.end) != Some(patterns[m.pattern].as_slice()) {
            return Err(format!("{m:?} does not point at pattern {}", m.pattern));
        }
    }
    let count = matcher.count_overlapping(&haystack);
    if count != case.expected.len() {
        return Err(format!(
            "count_overlapping = {count}, expected {} ({what})",
            case.expected.len()
        ));
    }
    if matcher.is_match(&haystack) == case.expected.is_empty() {
        return Err(format!("is_match disagrees ({what})"));
    }
    Ok(())
}

/// 並び順まで含めて比べる。違ったら最初に違う位置を示す。
fn compare(name: &str, got: &[Match], expected: &[[usize; 2]], what: &str) -> Result<(), String> {
    let got: Vec<[usize; 2]> = got.iter().map(|m| [m.end - 1, m.pattern]).collect();
    if got == expected {
        return Ok(());
    }
    let at = got
        .iter()
        .zip(expected)
        .position(|(a, b)| a != b)
        .unwrap_or(got.len().min(expected.len()));
    Err(format!(
        "{name} differs ({what}): {} matches vs {} expected; first difference at #{at}: {:?} vs {:?}",
        got.len(),
        expected.len(),
        got.get(at),
        expected.get(at)
    ))
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("odd-length hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

// --- ベンチマーク用のデータ ---

/// 決まった列を作る乱数（SplitMix64）。毎回同じパターンと入力になる。
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }

    fn word(&mut self, alphabet: &[u8], len: usize) -> String {
        (0..len)
            .map(|_| alphabet[self.below(alphabet.len())] as char)
            .collect()
    }

    fn ip(&mut self) -> String {
        format!(
            "{}.{}.{}.{}",
            self.below(256),
            self.below(256),
            self.below(256),
            self.below(256)
        )
    }
}

const LOWER: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const HEX: &[u8] = b"0123456789abcdef";

/// IOC 風のパターン：ドメイン・IPv4・MD5/SHA-1/SHA-256・URL のパス・ファイル名を同じくらいずつ
fn ioc_patterns(rng: &mut Rng, n: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let ioc = match out.len() % 5 {
            0 => {
                let len = 5 + rng.below(10);
                let name = rng.word(LOWER, len);
                let tld = rng.pick(&["com", "net", "org", "ru", "xyz", "top", "info"]);
                format!("{name}.{tld}")
            }
            1 => rng.ip(),
            2 => {
                let len = [32, 40, 64][rng.below(3)];
                rng.word(HEX, len)
            }
            3 => {
                let dir = rng.pick(&["wp-admin", "wp-content/plugins", "cgi-bin", ".git", "admin"]);
                let len = 4 + rng.below(8);
                let file = rng.word(LOWER, len);
                format!("/{dir}/{file}.php")
            }
            _ => {
                let len = 4 + rng.below(8);
                let name = rng.word(LOWER, len);
                let ext = rng.pick(&["exe", "dll", "ps1", "sh", "bin"]);
                format!("{name}.{ext}")
            }
        };
        if !out.contains(&ioc) {
            out.push(ioc);
        }
    }
    out
}

/// アクセスログ風の行。およそ 50 行に 1 つ、どこかに IOC が入る
fn log_lines(rng: &mut Rng, iocs: &[String], size: usize) -> Vec<u8> {
    const PATHS: &[&str] = &[
        "/",
        "/index.html",
        "/api/v1/users",
        "/static/app.js",
        "/login",
        "/images/logo.png",
    ];
    const AGENTS: &[&str] = &[
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)",
        "curl/8.9.1",
        "python-requests/2.32.3",
    ];
    let mut out = Vec::with_capacity(size + 512);
    while out.len() < size {
        let planted = (rng.below(50) == 0).then(|| iocs[rng.below(iocs.len())].as_str());
        let ip = rng.ip();
        let path = rng.pick(PATHS);
        let status = rng.pick(&["200", "200", "200", "304", "404", "500"]);
        let bytes = rng.below(100_000);
        let agent = rng.pick(AGENTS);
        let trace = rng.word(HEX, 32);
        let line = match planted {
            Some(ioc) => format!(
                "{ip} - - [27/Sep/2026:10:{:02}:{:02} +0000] \"GET {path}?q={ioc} HTTP/1.1\" {status} {bytes} \"{agent}\" trace={trace}\n",
                rng.below(60),
                rng.below(60)
            ),
            None => format!(
                "{ip} - - [27/Sep/2026:10:{:02}:{:02} +0000] \"GET {path} HTTP/1.1\" {status} {bytes} \"{agent}\" trace={trace}\n",
                rng.below(60),
                rng.below(60)
            ),
        };
        out.extend_from_slice(line.as_bytes());
    }
    out.truncate(size);
    out
}

fn bench() {
    let mut args = std::env::args().skip(2);
    let num_patterns: usize = args.next().expect("patterns").parse().expect("patterns");
    let sizes: Vec<usize> = args.map(|a| a.parse().expect("size")).collect();
    let max = sizes.iter().copied().max().unwrap_or(0);

    let mut rng = Rng(20260927);
    let patterns = ioc_patterns(&mut rng, num_patterns);
    let haystack = log_lines(&mut rng, &patterns, max);

    let dir: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "target",
        "verify-data",
    ]
    .iter()
    .collect::<PathBuf>()
    .join("multi-pattern-match");
    std::fs::create_dir_all(&dir).expect("create bench dir");
    std::fs::write(
        dir.join("patterns.json"),
        serde_json::to_string(&patterns).unwrap(),
    )
    .expect("write patterns");
    std::fs::write(dir.join("haystack.bin"), &haystack).expect("write haystack");

    let matcher = Matcher::new(&patterns).expect("build");
    let points: Vec<BenchPoint> = sizes
        .iter()
        .map(|&size| {
            let input = &haystack[..size];
            let matches = matcher.find_overlapping(input).len();
            let rust_ms = median_ms(
                || {
                    black_box(matcher.find_overlapping(black_box(input)));
                },
                5,
                300.0,
                1000,
            );
            BenchPoint {
                input_size: size as u64,
                rust_ms,
                matches,
            }
        })
        .collect();
    // Python 側が、同じ数の一致を見つけているかを確かめるために読む
    std::fs::write(
        dir.join("rust-counts.json"),
        serde_json::to_string(&points).unwrap(),
    )
    .expect("write counts");
    print_json(&points);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(patterns: &[&str], haystack: &str, expected: Vec<[usize; 2]>) -> Case {
        let expected_long = expected.clone();
        Case {
            patterns: patterns
                .iter()
                .map(|p| p.bytes().map(|b| format!("{b:02x}")).collect())
                .collect(),
            haystack: Haystack::Hex {
                hex: haystack.bytes().map(|b| format!("{b:02x}")).collect(),
            },
            expected,
            expected_long,
        }
    }

    #[test]
    fn accepts_the_right_answer() {
        assert_eq!(
            check(&case(&["ab", "x"], "abxab", vec![[1, 0], [2, 1], [4, 0]])),
            Ok(())
        );
        assert_eq!(check(&case(&["a"], "", vec![])), Ok(()));
    }

    #[test]
    fn accepts_repeated_haystacks() {
        let mut c = case(&["ab", "c"], "", vec![[1, 0], [3, 0], [4, 1]]);
        c.haystack = Haystack::Repeat {
            hex: "6162".into(),
            count: 2,
            tail: "63".into(),
        };
        assert_eq!(check(&c), Ok(()));
    }

    // 以下は「検証がわざと壊した答えを見逃さない」ことの確認

    #[test]
    fn rejects_tampered_expected_matches() {
        let right = vec![[1, 0], [2, 1], [4, 0]];
        let tampered = [
            vec![[1, 0], [2, 1]],                 // 1つ足りない
            vec![[1, 0], [2, 1], [4, 0], [4, 1]], // 1つ多い
            vec![[1, 0], [2, 0], [4, 0]],         // 番号が違う
            vec![[1, 0], [3, 1], [4, 0]],         // 位置が違う
            vec![[2, 1], [1, 0], [4, 0]],         // 順番が違う
            vec![],
        ];
        for t in tampered {
            let mut c = case(&["ab", "x"], "abxab", right.clone());
            c.expected = t.clone();
            assert!(check(&c).is_err(), "{t:?}");
            let mut c = case(&["ab", "x"], "abxab", right.clone());
            c.expected_long = t.clone();
            assert!(check(&c).is_err(), "long {t:?}");
        }
    }

    #[test]
    fn rejects_a_spurious_match_on_an_empty_answer() {
        assert!(check(&case(&["a"], "", vec![[0, 0]])).is_err());
        assert!(check(&case(&[""], "abc", vec![[0, 0]])).is_err());
    }

    #[test]
    fn rejects_tampered_input() {
        let right = vec![[1, 0], [2, 1], [4, 0]];
        assert!(check(&case(&["ab", "x"], "abxac", right.clone())).is_err());
        assert!(check(&case(&["ab", "y"], "abxab", right.clone())).is_err());
        assert!(check(&case(&["x", "ab"], "abxab", right)).is_err());
    }

    #[test]
    fn bench_data_is_deterministic() {
        let a = {
            let mut rng = Rng(1);
            let p = ioc_patterns(&mut rng, 100);
            (log_lines(&mut rng, &p, 10_000), p)
        };
        let b = {
            let mut rng = Rng(1);
            let p = ioc_patterns(&mut rng, 100);
            (log_lines(&mut rng, &p, 10_000), p)
        };
        assert_eq!(a, b);
        assert_eq!(a.0.len(), 10_000);
        assert_eq!(a.1.len(), 100);
    }
}
