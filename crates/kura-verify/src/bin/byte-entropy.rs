//! byte-entropy の差分テスト（verify）とベンチマーク（bench）。
//!
//! verify: 標準入力の各ケースについて、scipy.stats.entropy(numpy.bincount(...), base=2) の答えと
//!         f64 のビット列が 1 ビットも違わないかを見る（許容誤差 0）。全体の entropy・entropy_of_histogram・
//!         windows（窓の数とオフセットも）を比べる。不一致があれば、差の最大（絶対・相対）も報告する。
//! bench:  入力の大きさ（バイト）ごとに、4 KiB の窓（4 KiB 刻み）のエントロピーをすべて求める時間と、
//!         全体のエントロピーを1つ求める時間を計る。

use std::cell::Cell;
use std::hint::black_box;

use kura_parts::byte_entropy::{entropy, entropy_of_histogram, histogram, window_count, windows};
use kura_verify::{VerifyReport, median_ms, print_json, run_cases};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Case {
    source: Source,
    len: usize,
    /// None なら全体のエントロピー
    window: Option<usize>,
    step: Option<usize>,
    /// 「窓の数:」に続けて、各値の f64 ビット列（16 進 16 桁）を並べたもの
    expected: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Source {
    Hex { hex: String },
    File { path: std::path::PathBuf },
}

#[derive(Serialize)]
struct Report {
    #[serde(flatten)]
    report: VerifyReport,
    /// 答えが食い違った値のうち、差の最大（すべて一致すれば 0）
    max_abs_error: f64,
    max_rel_error: f64,
}

#[derive(Serialize)]
struct BenchPoint {
    input_size: u64,
    rust_ms: f64,
    whole_ms: f64,
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("verify") => {
            let max = Cell::new((0.0f64, 0.0f64));
            let report = run_cases(|case: &Case| check(case, &max));
            let (max_abs_error, max_rel_error) = max.get();
            print_json(&Report {
                report,
                max_abs_error,
                max_rel_error,
            });
        }
        Some("bench") => bench(),
        _ => {
            eprintln!(
                "usage: byte-entropy verify < cases.jsonl | byte-entropy bench windows <window> <size>..."
            );
            std::process::exit(2);
        }
    }
}

fn parse_expected(s: &str) -> Result<Vec<f64>, String> {
    let (count, hex) = s.split_once(':').ok_or("expected has no ':'")?;
    let count: usize = count.parse().map_err(|e| format!("count: {e}"))?;
    if hex.len() != count * 16 {
        return Err(format!("{count} values but {} hex digits", hex.len()));
    }
    (0..count)
        .map(|i| {
            u64::from_str_radix(&hex[i * 16..i * 16 + 16], 16)
                .map(f64::from_bits)
                .map_err(|e| e.to_string())
        })
        .collect()
}

fn check(case: &Case, max: &Cell<(f64, f64)>) -> Result<(), String> {
    let data = match &case.source {
        Source::Hex { hex } => unhex(hex)?,
        Source::File { path } => std::fs::read(path).map_err(|e| format!("{path:?}: {e}"))?,
    };
    if data.len() != case.len {
        return Err(format!(
            "input has {} bytes, expected {}",
            data.len(),
            case.len
        ));
    }
    let expected = parse_expected(&case.expected)?;
    let compare = |what: String, got: f64, want: f64| {
        if got.to_bits() == want.to_bits() {
            return Ok(());
        }
        let abs = (got - want).abs();
        let rel = abs / want.abs();
        let (a, r) = max.get();
        max.set((a.max(abs), r.max(rel)));
        Err(format!(
            "{what} ({} bytes): {got:?} != {want:?} (diff {abs:e})",
            data.len()
        ))
    };
    match (case.window, case.step) {
        (None, None) => {
            let [want] = expected[..] else {
                return Err(format!("expected 1 value, got {}", expected.len()));
            };
            compare("entropy".into(), entropy(&data), want)?;
            compare(
                "entropy_of_histogram".into(),
                entropy_of_histogram(&histogram(&data)),
                want,
            )
        }
        (Some(window), Some(step)) => {
            let what = format!("window {window}, step {step}");
            let count = window_count(data.len(), window, step).map_err(|e| e.to_string())?;
            if count != expected.len() {
                return Err(format!(
                    "{what}: window_count = {count}, expected {}",
                    expected.len()
                ));
            }
            let got: Vec<_> = windows(&data, window, step)
                .map_err(|e| e.to_string())?
                .collect();
            if got.len() != expected.len() {
                return Err(format!(
                    "{what}: {} windows, expected {}",
                    got.len(),
                    expected.len()
                ));
            }
            for (i, (w, &want)) in got.iter().zip(&expected).enumerate() {
                if w.offset != i * step {
                    return Err(format!("{what}: window {i} at offset {}", w.offset));
                }
                compare(format!("{what}, offset {}", w.offset), w.entropy, want)?;
            }
            Ok(())
        }
        _ => Err("window and step must both be given or both be null".into()),
    }
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

/// xorshift64 の乱数バイト列（中身は速さにほとんど関係しない。Python 側は NumPy の乱数）
fn random_bytes(len: usize) -> Vec<u8> {
    let mut x = 0x2545_f491_4f6c_dd1du64;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 32) as u8
        })
        .collect()
}

fn bench() {
    let mut args = std::env::args().skip(2);
    assert_eq!(args.next().as_deref(), Some("windows"));
    let window: usize = args.next().expect("window").parse().expect("window");
    let points: Vec<BenchPoint> = args
        .map(|a| a.parse::<u64>().expect("size"))
        .map(|size| {
            let data = random_bytes(size as usize);
            let rust_ms = median_ms(
                || {
                    // Python 側と同じく、すべての窓の値を集める
                    let all: Vec<f64> = windows(black_box(&data), window, window)
                        .unwrap()
                        .map(|w| w.entropy)
                        .collect();
                    black_box(all);
                },
                5,
                300.0,
                1000,
            );
            let whole_ms = median_ms(
                || {
                    black_box(entropy(black_box(&data)));
                },
                5,
                300.0,
                1000,
            );
            BenchPoint {
                input_size: size,
                rust_ms,
                whole_ms,
            }
        })
        .collect();
    print_json(&points);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(x: f64) -> String {
        format!("{:016x}", x.to_bits())
    }

    fn case(hex: &str, window: Option<usize>, step: Option<usize>, expected: String) -> Case {
        Case {
            source: Source::Hex { hex: hex.into() },
            len: hex.len() / 2,
            window,
            step,
            expected,
        }
    }

    fn run(c: &Case) -> Result<(), String> {
        check(c, &Cell::new((0.0, 0.0)))
    }

    // "abab" の全体は 1.0、窓 2・刻み 1 なら 3 つの窓がすべて 1.0
    const ABAB: &str = "61626162";

    #[test]
    fn accepts_the_right_answer() {
        assert_eq!(
            run(&case(ABAB, None, None, format!("1:{}", bits(1.0)))),
            Ok(())
        );
        let three = format!("3:{}", bits(1.0).repeat(3));
        assert_eq!(run(&case(ABAB, Some(2), Some(1), three)), Ok(()));
        assert_eq!(
            run(&case(ABAB, Some(5), Some(1), "0:".into())),
            Ok(()),
            "no window fits"
        );
    }

    // 以下は「検証がわざと壊した答えを見逃さない」ことの確認

    #[test]
    fn rejects_a_one_ulp_difference() {
        let next = f64::from_bits(1.0f64.to_bits() + 1);
        let prev = f64::from_bits(1.0f64.to_bits() - 1);
        for wrong in [next, prev] {
            assert!(run(&case(ABAB, None, None, format!("1:{}", bits(wrong)))).is_err());
        }
        let max = Cell::new((0.0, 0.0));
        let _ = check(&case(ABAB, None, None, format!("1:{}", bits(next))), &max);
        assert_eq!(max.get().0, next - 1.0);
    }

    #[test]
    fn rejects_every_tampered_digit() {
        let good = format!("3:{}", bits(1.0).repeat(3));
        for i in 0..good.len() {
            let mut tampered = good.as_bytes().to_vec();
            tampered[i] = if tampered[i] == b'0' { b'1' } else { b'0' };
            let tampered = String::from_utf8(tampered).unwrap();
            assert!(run(&case(ABAB, Some(2), Some(1), tampered)).is_err(), "{i}");
        }
    }

    #[test]
    fn rejects_a_wrong_window_count() {
        // 末尾の半端な窓を含めた数（2）や、1 つ少ない数は不一致
        assert!(
            run(&case(
                ABAB,
                Some(3),
                Some(2),
                format!("2:{}", bits(0.9182958340544894).repeat(2))
            ))
            .is_err()
        );
        assert!(
            run(&case(
                ABAB,
                Some(2),
                Some(1),
                format!("2:{}", bits(1.0).repeat(2))
            ))
            .is_err()
        );
    }

    #[test]
    fn rejects_nan_for_empty_input() {
        // 空の入力は 0.0 と決めている（SciPy の nan ではない）
        assert_eq!(
            run(&case("", None, None, format!("1:{}", bits(0.0)))),
            Ok(())
        );
        assert!(run(&case("", None, None, format!("1:{}", bits(f64::NAN)))).is_err());
        assert!(run(&case("", None, None, format!("1:{}", bits(-0.0)))).is_err());
    }

    #[test]
    fn rejects_tampered_input_and_bad_arguments() {
        let one = format!("1:{}", bits(1.0));
        assert!(run(&case("61626163", None, None, one.clone())).is_err());
        let mut c = case(ABAB, None, None, one.clone());
        c.len = 5;
        assert!(run(&c).is_err());
        assert!(run(&case(ABAB, Some(0), Some(1), "0:".into())).is_err());
        assert!(run(&case(ABAB, Some(1), Some(0), "0:".into())).is_err());
        assert!(run(&case(ABAB, Some(1), None, one)).is_err());
    }
}
