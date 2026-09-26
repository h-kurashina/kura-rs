"""file-hash の参照実装側。verify/run.py から呼ばれる。

SHA-256 は hashlib（Python 標準。中身は OpenSSL）、BLAKE3 は blake3 パッケージ（hashlib には BLAKE3 がない）と比べる。

cases <n> <seed>               : 差分テストのケースと参照実装の答えを JSON Lines で標準出力に書く
bench <algorithm> <size>...    : 入力の大きさ（バイト）ごとの処理時間（ミリ秒の中央値）を JSON で書く

大きなランダム入力は JSON に入れず、target/verify-data/file-hash/ にファイルとして書き、Rust 側は hash_file でも読む。
"""

import hashlib
import json
import random
import statistics
import sys
import time
from pathlib import Path

import blake3

DATA_DIR = Path(__file__).resolve().parents[2] / "target" / "verify-data" / "file-hash"

ALGORITHMS = ["sha256", "blake3"]

# ブロックの境目（SHA-256 は 64 バイト、BLAKE3 は 64 バイトのブロックと 1024 バイトのチャンク）と、
# 読み込み用バッファ（64 KiB）の境目の前後
BOUNDARIES = sorted({
    n + d
    for n in [0, 32, 56, 64, 128, 1024, 2048, 4096, 8192, 16384, 65536, 131072, 1 << 20, 3 << 20]
    for d in [-1, 0, 1]
    if n + d >= 0
})

# JSON に16進でそのまま入れるランダム入力の上限（これより大きいものはファイルにする）
INLINE_MAX = 16 * 1024
# ファイルにする大きなランダム入力の数と大きさの上限
LARGE_FILES = 40
LARGE_MAX = 8 << 20


def new(algorithm: str):
    return hashlib.sha256() if algorithm == "sha256" else blake3.blake3()


def cycle251(n: int) -> bytes:
    """BLAKE3 公式テストベクタの入力：0, 1, ..., 250 の繰り返し。"""
    return (bytes(range(251)) * (n // 251 + 1))[:n]


def materialize(source: dict) -> bytes:
    kind = source["kind"]
    if kind == "hex":
        return bytes.fromhex(source["hex"])
    if kind == "fill":
        return bytes([source["byte"]]) * source["len"]
    if kind == "cycle251":
        return cycle251(source["len"])
    if kind == "file":
        return Path(source["path"]).read_bytes()
    raise ValueError(kind)


def random_splits(rng: random.Random, n: int) -> list[int]:
    """入力をどう区切って渡すか（0 バイト・1 バイト・境目の値・大きな塊を混ぜる）。"""
    splits = []
    for _ in range(rng.randrange(0, 25)):
        kind = rng.random()
        if kind < 0.15:
            splits.append(0)
        elif kind < 0.3:
            splits.append(1)
        elif kind < 0.5:
            splits.append(rng.choice([63, 64, 65, 127, 128, 129, 1023, 1024, 1025, 4095, 4096, 4097, 65535, 65536, 65537]))
        else:
            splits.append(rng.randrange(0, max(2, n // 4 + 1)))
    return splits


def expected(algorithm: str, data: bytes, chunks: list[int]) -> str:
    one_shot = (hashlib.sha256(data) if algorithm == "sha256" else blake3.blake3(data)).hexdigest()
    # 参照実装側でも、区切って入れても同じ答えになることを確かめておく
    h, rest = new(algorithm), memoryview(data)
    for size in chunks:
        h.update(rest[:size])
        rest = rest[size:]
    h.update(rest)
    assert h.hexdigest() == one_shot, "reference implementation depends on chunking"
    return one_shot


def emit(algorithm: str, source: dict, data: bytes, rng: random.Random) -> None:
    chunks = random_splits(rng, len(data))
    print(json.dumps({
        "algorithm": algorithm,
        "source": source,
        "len": len(data),
        "chunks": chunks,
        "reads": random_splits(rng, len(data)),
        "expected": expected(algorithm, data, chunks),
    }))


def source_for(rng: random.Random, pattern: str, n: int, file_index: list[int]) -> tuple[dict, bytes]:
    if pattern == "zeros":
        source = {"kind": "fill", "byte": 0, "len": n}
    elif pattern == "ones":
        source = {"kind": "fill", "byte": 0xFF, "len": n}
    elif pattern == "byte":
        source = {"kind": "fill", "byte": rng.randrange(256), "len": n}
    elif pattern == "cycle251":
        source = {"kind": "cycle251", "len": n}
    elif n <= INLINE_MAX:
        source = {"kind": "hex", "hex": rng.randbytes(n).hex()}
    else:
        path = DATA_DIR / f"{file_index[0]}.bin"
        file_index[0] += 1
        path.write_bytes(rng.randbytes(n))
        source = {"kind": "file", "path": str(path)}
    return source, materialize(source)


def cases(n: int, seed: int) -> None:
    rng = random.Random(seed)
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    for old in DATA_DIR.glob("*.bin"):
        old.unlink()
    file_index = [0]
    count = 0

    # 1. 境目の長さ × 入力の種類 × アルゴリズム（すべての組み合わせ）
    for size in BOUNDARIES:
        for pattern in ["zeros", "ones", "cycle251", "random"]:
            source, data = source_for(rng, pattern, size, file_index)
            for algorithm in ALGORITHMS:
                emit(algorithm, source, data, rng)
                count += 1

    # 2. 数 MB までの大きなランダム入力（ファイル。両方のアルゴリズムで使う）
    for _ in range(LARGE_FILES):
        size = int(2 ** rng.uniform(17, 23))  # 128 KiB 〜 8 MiB（対数で一様）
        size = min(LARGE_MAX, size + rng.randrange(-70, 70))
        source, data = source_for(rng, "random", size, file_index)
        for algorithm in ALGORITHMS:
            emit(algorithm, source, data, rng)
            count += 1

    # 3. 残りはランダムな長さ・種類
    while count < n:
        algorithm = ALGORITHMS[count % 2]
        r = rng.random()
        if r < 0.45:
            size = rng.randrange(0, 256)
        elif r < 0.8:
            size = rng.randrange(256, 4096)
        elif r < 0.95:
            size = rng.randrange(4096, INLINE_MAX + 1)
        else:
            size = rng.randrange(INLINE_MAX, 1 << 21)
        pattern = rng.choices(["random", "zeros", "ones", "byte", "cycle251"], weights=[80, 5, 5, 5, 5])[0]
        if pattern == "random" and size > INLINE_MAX:
            pattern = "cycle251"  # ファイルを増やしすぎない（大きなランダム入力は 2. で見ている）
        source, data = source_for(rng, pattern, size, file_index)
        emit(algorithm, source, data, rng)
        count += 1


def median_ms(f, min_runs=5, min_total_ms=300.0, max_runs=1000) -> float:
    samples, total = [], 0.0
    while len(samples) < min_runs or (total < min_total_ms and len(samples) < max_runs):
        start = time.perf_counter()
        f()
        ms = (time.perf_counter() - start) * 1000
        samples.append(ms)
        total += ms
    return statistics.median(samples)


def bench(algorithm: str, sizes: list[int]) -> None:
    points = []
    for size in sizes:
        data = cycle251(size)  # Rust 側と同じバイト列（中身は速さに関係しない）
        if algorithm == "sha256":
            def run():
                hashlib.sha256(data).digest()
        else:
            def run():
                blake3.blake3(data).digest()
        points.append({"input_size": size, "reference_ms": median_ms(run)})
    print(json.dumps(points))


if __name__ == "__main__":
    command, *args = sys.argv[1:]
    if command == "cases":
        cases(int(args[0]), int(args[1]))
    elif command == "bench":
        bench(args[0], [int(a) for a in args[1:]])
    else:
        sys.exit(f"unknown command: {command}")
