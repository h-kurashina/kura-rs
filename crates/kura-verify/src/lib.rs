//! 差分テストとベンチマークの共通処理。verify/run.py から呼ばれる。
//!
//! 入力（標準入力）は Python 側が参照実装で作った JSON Lines、
//! 出力（標準出力）は1つの JSON。人が数値を書き写さないように、結果はそのまま registry に書き込まれる。

use std::io::{BufRead, Write};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub mod shingle;

/// 差分テストの結果。
#[derive(Debug, Serialize)]
pub struct VerifyReport {
    pub cases: u32,
    pub passed: u32,
    /// 最初の数件の不一致（原因を探すため）。
    pub failures: Vec<String>,
}

/// JSON Lines の各ケースを `check` にかけ、一致した数を数える。
pub fn run_cases<C: DeserializeOwned>(check: impl Fn(&C) -> Result<(), String>) -> VerifyReport {
    let mut report = VerifyReport {
        cases: 0,
        passed: 0,
        failures: Vec::new(),
    };
    for (i, line) in std::io::stdin().lock().lines().enumerate() {
        let line = line.expect("read stdin");
        if line.trim().is_empty() {
            continue;
        }
        let case: C = serde_json::from_str(&line).unwrap_or_else(|e| panic!("case {i}: {e}"));
        report.cases += 1;
        match check(&case) {
            Ok(()) => report.passed += 1,
            Err(message) if report.failures.len() < 5 => {
                report.failures.push(format!("case {i}: {message}"))
            }
            Err(_) => {}
        }
    }
    report
}

/// 1回あたりの処理時間の中央値（ミリ秒）。少なくとも min_runs 回、合計が min_total_ms を超えるか max_runs に達するまで繰り返す。
pub fn median_ms(mut f: impl FnMut(), min_runs: usize, min_total_ms: f64, max_runs: usize) -> f64 {
    let mut samples = Vec::new();
    let mut total = 0.0;
    while samples.len() < min_runs || (total < min_total_ms && samples.len() < max_runs) {
        let start = std::time::Instant::now();
        f();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        samples.push(ms);
        total += ms;
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    samples[samples.len() / 2]
}

pub fn print_json(value: &impl Serialize) {
    let mut out = std::io::stdout().lock();
    serde_json::to_writer(&mut out, value).expect("write stdout");
    writeln!(out).expect("write stdout");
}
