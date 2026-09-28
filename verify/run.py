"""部品の差分テストとベンチマークを動かし、結果を registry/<name>.json に書き込む。

    python verify/run.py minhash           # 差分テスト + ベンチマーク → registry に書き込む
    python verify/run.py minhash --check   # 差分テストだけ。registry の cases / passed と一致しなければ失敗（CI 用）
    python verify/run.py file-hash         # file-hash も同じ
    python verify/run.py multi-pattern-match  # multi-pattern-match も同じ
    python verify/run.py byte-entropy      # byte-entropy も同じ
    python verify/run.py minhash --synthetic  # ベンチマークを合成トークンで取る（registry の input_unit もそれに合わせる）

数値は人が書かず、必ずこのスクリプトが書く。書き込むと sample は false になる。
"""

import argparse
import datetime
import importlib.metadata
import json
import platform
import ssl
import subprocess
import sys
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PYTHON = sys.executable

# 部品ごとの設定。部品を増やすときはここと verify/<name>/reference.py、crates/kura-verify/src/bin/<name>.rs を足す
PARTS = {
    "minhash": {
        "reference_package": "datasketch",
        "cases": 20000,
        "case_seed": 20260926,
        "num_perm": 128,
        # 合成トークン（token-0, token-1, …）のベンチマーク（--synthetic のとき）
        "sizes": [10, 100, 1000, 10000, 100000],
        "synthetic_input_unit": "tokens per document (num_perm = 128)",
        # 実文書（Wikipedia）。記事の一覧は verify/minhash/corpus.json、本文は verify/data/（取得してキャッシュ）
        "prepare": [["verify/minhash/shingle.py", "check"], ["verify/minhash/corpus.py", "fetch"]],
        "text_case_seed": 20260926,
        "input_unit": "word 3-shingles per document from Wikipedia: English paragraphs, whole articles and "
        "concatenated articles; the largest adds German, French and Korean articles (num_perm = 128)",
    },
    "file-hash": {
        # SHA-256 は hashlib、BLAKE3 は blake3 パッケージと比べる（hashlib には BLAKE3 がない）
        "reference_package": "hashlib",
        "reference_label": "hashlib.sha256 / blake3",
        # hashlib は Python に付いてくるので、版は Python の版。速さは中の OpenSSL で決まるので環境の欄に書く
        "reference_version": platform.python_version,
        "reference_runtime": lambda: (
            f"Python {platform.python_version()}, {' '.join(ssl.OPENSSL_VERSION.split()[:2])}, "
            f"blake3 {importlib.metadata.version('blake3')}"
        ),
        "cases": 10000,
        "case_seed": 20260926,
        # ベンチマークは SHA-256 だけ（registry の reference は hashlib）
        "bench_args": ["sha256"],
        "sizes": [1 << k for k in range(10, 29, 2)],  # 1 KiB, 4 KiB, ..., 256 MiB
        "input_unit": "bytes (SHA-256, in memory)",
        # 期待値をわざと壊したケースが、すべて不一致になることも確かめる
        "tamper_field": "expected",
    },
    "multi-pattern-match": {
        "reference_package": "pyahocorasick",
        "reference_runtime": lambda: (
            f"Python {platform.python_version()}, pyahocorasick {importlib.metadata.version('pyahocorasick')} (C extension)"
        ),
        "cases": 5000,
        "case_seed": 20260927,
        # IOC 風のパターン 1,000 個で、アクセスログ風の入力を探す
        "bench_args": [1000],
        "sizes": [1 << k for k in range(10, 27, 2)],  # 1 KiB, 4 KiB, ..., 64 MiB
        "input_unit": "bytes of access-log text searched for 1,000 IOC patterns (domains, IPs, hashes, paths); "
        "all overlapping matches collected",
        # 一致の列（[末尾の位置, 番号] のリスト）をわざと壊したケースが、すべて不一致になることも確かめる
        "tamper_field": "expected",
        "tamper": lambda matches, i: tamper_matches(matches, i),
    },
    "byte-entropy": {
        # 答えは scipy.stats.entropy(numpy.bincount(data, minlength=256), base=2)。f64 のビット列で比べる（許容誤差 0）
        "reference_package": "scipy",
        "reference_label": "scipy.stats.entropy (bit-for-bit)",
        "reference_runtime": lambda: (
            f"Python {platform.python_version()}, numpy {importlib.metadata.version('numpy')}, "
            f"scipy {importlib.metadata.version('scipy')}"
        ),
        "cases": 10000,
        "case_seed": 20260927,
        # 4 KiB の窓（4 KiB 刻み）をすべて計算する時間。参照は NumPy でまとめて数え、scipy.stats.entropy(axis=1) にかけるもの
        "bench_args": ["windows", 4096],
        "sizes": [1 << k for k in range(12, 29, 2)],  # 4 KiB, 16 KiB, ..., 256 MiB
        "input_unit": "bytes of random data; entropy of every 4 KiB window (step 4 KiB). Reference: numpy.bincount "
        "over all windows at once + scipy.stats.entropy(axis=1)",
        # 期待値をわざと壊したケースが、すべて不一致になることも確かめる
        "tamper_field": "expected",
    },
    "unicode-normalize": {
        # 正規化は unicodedata（Python に付いてくる）と公式の NormalizationTest.txt、
        # 空白のまとめと見えない文字の除去は verify/unicode-normalize/reference.py の定義と比べる
        "reference_package": "unicodedata",
        "reference_label": "unicodedata / NormalizationTest-16.0.0.txt",
        "reference_version": platform.python_version,
        "reference_runtime": lambda: f"Python {platform.python_version()}, unicodedata (Unicode {unicodedata.unidata_version})",
        "prepare": [["verify/unicode-normalize/reference.py", "fetch"]],
        # ランダム・意地悪な入力の数（ほかに NormalizationTest.txt の全行と、すべての符号位置を1文字ずつ見るケースがつく）
        "cases": 40000,
        "case_seed": 20260927,
        # 記録するのは NFKC（unicodedata.normalize）。パイプライン全体は表示だけ
        "bench_args": ["NFKC"],
        "extra_bench_args": [["pipeline"]],
        "sizes": [1 << k for k in range(10, 27, 2)],  # 1 KiB, 4 KiB, ..., 64 MiB
        "input_unit": "bytes of UTF-8 text (NFKC; generated mix of Japanese and English sentences; "
        "11 of the 34 sentence patterns contain full-width, half-width or other characters that NFKC changes)",
        "tamper_field": "expected",
    },
}


def tamper_matches(matches: list, i: int) -> list:
    """一致の列を必ず違うものにする：1つ消す・1つ足す・番号を変える・位置を変える・順番を入れ替える。"""
    matches = [list(m) for m in matches]
    if not matches:
        return [[0, 0]]
    k = i % len(matches)
    way = i % 5
    if way == 0:
        del matches[k]
    elif way == 1:
        matches.insert(k, list(matches[k]))
    elif way == 2:
        matches[k][1] += 1
    elif way == 3:
        matches[k][0] += 1
    elif len(matches) >= 2 and matches[k] != matches[k - 1]:
        matches[k], matches[k - 1] = matches[k - 1], matches[k]
    else:
        matches.append([matches[-1][0] + 1, 0])
    return matches


def run(args, stdin=None) -> str:
    result = subprocess.run(args, input=stdin, capture_output=True, text=True, cwd=ROOT)
    if result.returncode != 0:
        sys.exit(f"failed: {' '.join(map(str, args))}\n{result.stderr}")
    return result.stdout


def rust_bin(name: str) -> Path:
    run(["cargo", "build", "--quiet", "--release", "-p", "kura-verify", "--bin", name])
    return ROOT / "target" / "release" / name


def verify(name: str, config: dict) -> dict:
    for script, *args in config.get("prepare", []):
        print(run([PYTHON, script, *args]), end="")
    cases = run([PYTHON, f"verify/{name}/reference.py", "cases", str(config["cases"]), str(config["case_seed"])])
    report = json.loads(run([rust_bin(name), "verify"], stdin=cases))
    if "text_case_seed" in config:
        # 実文書のケースも数に含める。コーパスが取れないときは飛ばさずに失敗する（記録した数と比べられなくなるため）
        text = run([PYTHON, f"verify/{name}/reference.py", "text-cases", str(config["text_case_seed"])])
        text_report = json.loads(run([rust_bin(name), "verify-text"], stdin=text))
        print(f"  random: {report['passed']} / {report['cases']}, real text: {text_report['passed']} / {text_report['cases']}")
        report = {key: report[key] + text_report[key] for key in ["cases", "passed", "failures"]}
    for failure in report["failures"]:
        print(f"  mismatch: {failure}", file=sys.stderr)
    print(f"{name}: {report['passed']} / {report['cases']} cases match {config.get('reference_label', config['reference_package'])}")
    if "max_abs_error" in report:
        print(f"  largest difference from the reference: {report['max_abs_error']:g} (relative {report['max_rel_error']:g})")
    if "tamper_field" in config:
        tamper_check(name, config, cases)
    return report


def tamper_check(name: str, config: dict, cases: str, limit: int = 2000) -> None:
    """期待値を1文字だけ変えたケースが1件残らず不一致になるかを見る（検証が本当に比べていることの確認）。"""
    field = config["tamper_field"]
    lines = []
    for i, line in enumerate(cases.splitlines()[:limit]):
        case = json.loads(line)
        value = case[field]
        if "tamper" in config:
            case[field] = config["tamper"](value, i)
        else:
            pos = i % len(value) if value else 0
            case[field] = value[:pos] + ("1" if value[pos:pos + 1] == "0" else "0") + value[pos + 1:]
        lines.append(json.dumps(case))
    report = json.loads(run([rust_bin(name), "verify"], stdin="\n".join(lines) + "\n"))
    if report["cases"] != len(lines) or report["passed"] != 0:
        sys.exit(f"tamper check failed: {report['passed']} / {report['cases']} tampered cases were accepted")
    print(f"{name}: all {report['cases']} tampered cases were rejected")


def bench(name: str, config: dict, bench_args: list | None = None) -> list[dict]:
    sizes = [str(s) for s in config["sizes"]]
    bench_args = [str(a) for a in bench_args or config.get("bench_args", [config.get("num_perm")])]
    rust = json.loads(run([rust_bin(name), "bench", *bench_args, *sizes]))
    reference = json.loads(run([PYTHON, f"verify/{name}/reference.py", "bench", *bench_args, *sizes]))
    points = []
    for r, p in zip(rust, reference, strict=True):
        assert r["input_size"] == p["input_size"]
        assert r.get("text_sha1") == p.get("text_sha1"), f"Rust and Python benchmarked different inputs: {r} {p}"
        points.append({"input_size": r["input_size"], "rust_ms": round(r["rust_ms"], 6), "reference_ms": round(p["reference_ms"], 6)})
        print(f"  {r['input_size']:>10}: rust {r['rust_ms']:10.4f} ms   reference {p['reference_ms']:10.4f} ms")
        # registry の曲線には入れない、ほかの計り方（部品が出していれば表示だけ）
        for key in sorted(({**r, **p}.keys()) - {"input_size", "rust_ms", "reference_ms"}):
            times = [f"{side} {d[key]:10.4f} ms" for side, d in [("rust", r), ("reference", p)] if key in d]
            print(f"  {'':>10}  {key}: {'   '.join(times)}")
    return points


def bench_text(name: str, config: dict) -> list[dict]:
    """実文書のベンチマーク。文書は Python 側で選んでファイルに書き、Rust と Python がそれぞれシングル化して計る。"""
    data = ROOT / "verify" / "data" / name
    data.mkdir(parents=True, exist_ok=True)
    num_perm = str(config["num_perm"])
    docs = data / "bench-docs.json"
    run([PYTHON, f"verify/{name}/reference.py", "bench-docs", docs])
    rust = json.loads(run([rust_bin(name), "bench-text", num_perm, docs]))
    reference = json.loads(run([PYTHON, f"verify/{name}/reference.py", "bench-text", num_perm, docs]))
    points = []
    for r, p in zip(rust, reference, strict=True):
        assert r["input_size"] == p["input_size"], (r, p)
        points.append({"input_size": r["input_size"], "rust_ms": round(r["rust_ms"], 6), "reference_ms": round(p["reference_ms"], 6)})
        print(f"  {r['input_size']:>8}: rust {r['rust_ms']:10.4f} ms   reference {p['reference_ms']:10.4f} ms")
    # たくさんの段落をまとめて署名する場面（registry の曲線には入れず、表示だけ）
    many = data / "many-docs.json"
    run([PYTHON, f"verify/{name}/reference.py", "many-docs", many])
    r = json.loads(run([rust_bin(name), "bench-many", num_perm, many]))
    p = json.loads(run([PYTHON, f"verify/{name}/reference.py", "bench-many", num_perm, many]))
    assert (r["documents"], r["shingles"]) == (p["documents"], p["shingles"]), (r, p)
    print(
        f"  many documents ({r['documents']} paragraphs, {r['shingles']} shingles): "
        f"rust {r['rust_ms']:.2f} ms   reference {p['reference_ms']:.2f} ms   ({p['reference_ms'] / r['rust_ms']:.1f}x)"
    )
    return points


def cpu_name() -> str:
    if platform.system() == "Darwin":
        return subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip()
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return platform.processor() or platform.machine()


def environment(config: dict) -> dict:
    system = f"macOS {platform.mac_ver()[0]}" if platform.system() == "Darwin" else f"{platform.system()} {platform.release()}"
    return {
        "cpu": cpu_name(),
        "os": f"{system} ({platform.machine()})",
        "rust": run(["rustc", "--version"]).split(" (")[0],
        "reference_runtime": config["reference_runtime"]() if "reference_runtime" in config
        else f"Python {platform.python_version()}, numpy {importlib.metadata.version('numpy')}",
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("name", choices=PARTS)
    parser.add_argument("--check", action="store_true", help="verify only and compare with the registry")
    parser.add_argument("--synthetic", action="store_true", help="benchmark synthetic tokens instead of real text")
    args = parser.parse_args()
    config = PARTS[args.name]
    path = ROOT / "registry" / f"{args.name}.json"
    part = json.loads(path.read_text())

    report = verify(args.name, config)
    if args.check:
        recorded = part["verification"]
        if (recorded["cases"], recorded["passed"]) != (report["cases"], report["passed"]):
            sys.exit(f"registry says {recorded['passed']} / {recorded['cases']}, but this run got {report['passed']} / {report['cases']}")
        if report["passed"] != report["cases"]:
            sys.exit("some cases do not match the reference")
        return

    synthetic = args.synthetic or "text_case_seed" not in config
    points = bench(args.name, config) if synthetic else bench_text(args.name, config)
    for extra in config.get("extra_bench_args", []):
        # registry には入れず、表示だけ
        print(f"  {' '.join(extra)} (not recorded):")
        bench(args.name, config, extra)
    updates = {
        "sample": False,
        "verification": {
            "method": "differential",
            "cases": report["cases"],
            "passed": report["passed"],
            "last_run": datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        },
        "benchmarks": points,
        "environment": environment(config),
        "input_unit": config["synthetic_input_unit" if synthetic and "synthetic_input_unit" in config else "input_unit"],
    }
    part["reference"]["version"] = (
        config["reference_version"]() if "reference_version" in config else importlib.metadata.version(config["reference_package"])
    )
    # 既存のキーの順番を保ったまま書き換え、新しいキーは benchmarks の後ろに置く
    out = {}
    for key, value in part.items():
        if key in out:
            continue  # benchmarks の後ろにすでに書いた（古い値で上書きしない）
        out[key] = updates.pop(key, value)
        if key == "benchmarks":
            out.update({k: updates.pop(k) for k in ["environment", "input_unit"] if k in updates})
    out.update(updates)
    path.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n")
    print(f"wrote {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
