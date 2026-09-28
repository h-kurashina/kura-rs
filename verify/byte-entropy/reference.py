"""byte-entropy の参照実装側。verify/run.py から呼ばれる。

参照実装の定義（部品はこれとビット単位で同じ f64 を返す）:

    counts = numpy.bincount(numpy.frombuffer(data, numpy.uint8), minlength=256)
    scipy.stats.entropy(counts, base=2)

ただし空の入力だけは、SciPy が 0 / 0 で nan を返すところを 0.0 とする（部品の定義。ここでも nan であることを確かめてから置き換える）。
窓（window, step）は、先頭から step ずつずらした window バイトのうち、データに収まるものだけ（末尾の半端な窓は含めない）。

cases <n> <seed>           : 差分テストのケースと答えを JSON Lines で標準出力に書く
bench windows <window> <size>... : 入力の大きさ（バイト）ごとの処理時間（ミリ秒の中央値）を JSON で書く

比べ方: 答えは f64 のビット列（16 進 16 桁）で渡し、Rust 側は 1 ビットでも違えば不一致とする（許容誤差 0）。
大きな入力は JSON に入れず、target/verify-data/byte-entropy/ にファイルとして書く。
"""

import json
import os
import random
import statistics
import struct
import sys
import time
import zlib
from pathlib import Path

import numpy as np
import scipy.special
import scipy.stats

DATA_DIR = Path(__file__).resolve().parents[2] / "target" / "verify-data" / "byte-entropy"

# JSON に16進でそのまま入れる入力の上限（これより大きいものはファイルにする）
INLINE_MAX = 4096
# 1 ケースで SciPy を呼ぶ窓の数の上限（窓が多すぎる組み合わせは作らない）
MAX_WINDOWS = 600


def reference_entropy(data) -> float:
    counts = np.bincount(np.frombuffer(data, np.uint8), minlength=256)
    h = float(scipy.stats.entropy(counts, base=2))
    if len(data) == 0:
        assert np.isnan(h), "scipy.stats.entropy of all-zero counts is expected to be nan"
        return 0.0
    return h


def bits(x: float) -> str:
    """f64 のビット列を 16 進 16 桁で（Rust 側の f64::to_bits と同じ並び）。"""
    return struct.pack(">d", x).hex()


def reference_windows(data: bytes, window: int, step: int) -> list[float]:
    view = memoryview(data)
    return [reference_entropy(view[off:off + window]) for off in range(0, len(data) - window + 1, step)]


def expected(data: bytes, window: int | None, step: int | None) -> str:
    """答えの文字列：「窓の数:」に続けて、各窓のエントロピーのビット列を並べる（窓なしなら全体の1つ）。"""
    values = [reference_entropy(data)] if window is None else reference_windows(data, window, step)
    return f"{len(values)}:" + "".join(bits(v) for v in values)


# --- 入力の種類 ---

WORDS = (
    "the of and to in is that for it as with was on be by this are from at or an have not which "
    "entropy byte window packed section header binary data offset value random compressed text"
).split()


def text_bytes(rng: random.Random, n: int) -> bytes:
    out = []
    size = 0
    while size < n:
        w = rng.choice(WORDS)
        if rng.random() < 0.08:
            w = w.capitalize() + rng.choice([".", ",", ";", "\n"])
        out.append(w)
        size += len(w) + 1
    return " ".join(out).encode()[:n]


def skewed_bytes(rng: random.Random, n: int) -> bytes:
    """少ない値に偏ったバイト列（使う値の数と重みをランダムに選ぶ）。"""
    k = rng.choice([1, 2, 3, 5, 16, 17, 100, 255, 256])
    values = rng.sample(range(256), k)
    weights = [rng.random() ** rng.choice([1, 3, 8]) + 1e-9 for _ in values]
    return bytes(rng.choices(values, weights=weights, k=n))


def zero_runs(rng: random.Random, n: int) -> bytes:
    """ゼロの連続の間にランダムな部分がはさまったもの（パディングの多いバイナリのような形）。"""
    out = bytearray()
    while len(out) < n:
        if rng.random() < 0.5:
            out += bytes(rng.randrange(1, 3000))
        else:
            out += rng.randbytes(rng.randrange(1, 3000))
    return bytes(out[:n])


def compressed_bytes(rng: random.Random, n: int) -> bytes:
    """圧縮済みデータ（zlib）。圧縮したものはランダムに近く、エントロピーが高い。"""
    out = bytearray()
    while len(out) < n:
        out += zlib.compress(text_bytes(rng, 4 * n + 64), level=rng.choice([1, 6, 9]))
    return bytes(out[:n])


def uniform_bytes(rng: random.Random, n: int) -> bytes:
    """0..255 をちょうど同じ回数ずつ（並びはランダム）。エントロピーはちょうど 8。"""
    reps = max(1, n // 256)
    data = bytearray(bytes(range(256)) * reps)
    rng.shuffle(data)
    return bytes(data)


def native_binaries() -> list[Path]:
    """実際の機械語のファイル（numpy と scipy の拡張モジュール）。環境で中身は違うが、ケースの数は変わらない。"""
    import numpy._core._multiarray_umath as umath
    import scipy.special._ufuncs as ufuncs

    return [Path(umath.__file__), Path(ufuncs.__file__)]


class Writer:
    def __init__(self) -> None:
        DATA_DIR.mkdir(parents=True, exist_ok=True)
        for old in DATA_DIR.glob("*.bin"):
            old.unlink()
        self.files = 0
        self.count = 0

    def source(self, data: bytes) -> dict:
        if len(data) <= INLINE_MAX:
            return {"kind": "hex", "hex": data.hex()}
        path = DATA_DIR / f"{self.files}.bin"
        self.files += 1
        path.write_bytes(data)
        return {"kind": "file", "path": str(path)}

    def emit(self, source: dict, data: bytes, window: int | None = None, step: int | None = None) -> None:
        print(json.dumps({
            "source": source,
            "len": len(data),
            "window": window,
            "step": step,
            "expected": expected(data, window, step),
        }))
        self.count += 1


def window_configs(rng: random.Random, n: int) -> list[tuple[int, int]]:
    """窓の大きさと刻みの組（窓より長い・ちょうど・1 バイト短いデータ、重なる窓、すき間のある窓を含む）。"""
    windows = [1, 2, 3, 7, 16, 64, 255, 256, 257, 1000, 4096, 4097, 65536]
    windows += [w for w in [n - 1, n, n + 1, max(1, n // 2), max(1, n // 3)] if w > 0]
    configs = []
    for _ in range(rng.randrange(1, 4)):
        window = rng.choice(windows)
        step = rng.choice([1, 2, 3, max(1, window // 2), max(1, window - 1), window, window + 1, 3 * window, rng.randrange(1, 5000)])
        count = (n - window) // step + 1 if n >= window else 0
        if count > MAX_WINDOWS:
            # 窓が多すぎるときは、刻みを広げて数を抑える（重なりの有無の組み合わせは残す）
            step = max(step, (n - window) // MAX_WINDOWS + 1)
        configs.append((window, step))
    return configs


def cases(n: int, seed: int) -> None:
    rng = random.Random(seed)
    w = Writer()

    def emit_all(data: bytes) -> None:
        source = w.source(data)
        w.emit(source, data)
        for window, step in window_configs(rng, len(data)):
            w.emit(source, data, window, step)

    # 1. 決まった形（空・1 バイト・1 種類の値・全 256 値が同じ回数・2^k 種類の値）
    for data in [b"", b"\x00", b"\xff", b"a", b"ab", b"abc", bytes(range(256)), bytes(range(256)) * 16, bytes(4096), b"\xff" * 5000]:
        emit_all(data)
    for k in range(9):
        for reps in [1, 3, 17]:
            emit_all(bytes(i % (1 << k) for i in range((1 << k) * reps)))

    # 2. 実際のバイナリと、OS の乱数（暗号化されたデータのように見える。実行ごとに中身は変わるが数は同じ）
    for path in native_binaries():
        data = path.read_bytes()
        source = {"kind": "file", "path": str(path)}
        w.emit(source, data)
        for window, step in [(4096, 4096), (4096, 1024), (256, 256), (65536, 65536), (1000, 777)]:
            w.emit(source, data, window, min(max(step, (len(data) - window) // MAX_WINDOWS + 1), len(data)))
    for size in [1, 255, 256, 4096, 65536, 1 << 20]:
        emit_all(os.urandom(size))

    # 3. 残りはランダムな種類と長さ
    makers = [
        (lambda r, k: r.randbytes(k), 30),
        (skewed_bytes, 20),
        (text_bytes, 15),
        (zero_runs, 10),
        (compressed_bytes, 10),
        (uniform_bytes, 5),
        (lambda r, k: bytes([r.randrange(256)]) * k, 5),
        (lambda r, k: (bytes(range(251)) * (k // 251 + 1))[:k], 5),
    ]
    while w.count < n:
        r = rng.random()
        if r < 0.45:
            size = rng.randrange(0, 300)
        elif r < 0.88:
            size = rng.randrange(300, INLINE_MAX + 1)
        elif r < 0.995:
            size = rng.randrange(INLINE_MAX, 1 << 17)
        else:
            size = rng.randrange(1 << 17, 4 << 20)
        maker = rng.choices([m for m, _ in makers], weights=[wt for _, wt in makers])[0]
        data = maker(rng, size)
        source = w.source(data)
        w.emit(source, data)
        for window, step in window_configs(rng, len(data)):
            if w.count >= n:
                break
            w.emit(source, data, window, step)


# --- ベンチマーク ---

def median_ms(f, min_runs=5, min_total_ms=300.0, max_runs=1000) -> float:
    samples, total = [], 0.0
    while len(samples) < min_runs or (total < min_total_ms and len(samples) < max_runs):
        start = time.perf_counter()
        f()
        ms = (time.perf_counter() - start) * 1000
        samples.append(ms)
        total += ms
    return statistics.median(samples)


def windows_vectorized(arr: np.ndarray, window: int, chunk: int = 1024) -> np.ndarray:
    """重ならない窓をまとめて SciPy にかける（NumPy らしい書き方で、いちばん速い参照実装）。

    窓ごとに 256 ずつずらした値にして1回の bincount で全部の窓を数え、scipy.stats.entropy(axis=1) にかける。
    一度に全部を int64 にするとメモリが入力の 8 倍要るので、chunk 個の窓ずつ数える。
    """
    k = len(arr) // window
    rows = arr[: k * window].reshape(k, window)
    counts = np.empty((k, 256), dtype=np.int64)
    for start in range(0, k, chunk):
        part = rows[start:start + chunk]
        shifted = part + (np.arange(len(part), dtype=np.int64) * 256)[:, None]
        counts[start:start + chunk] = np.bincount(shifted.ravel(), minlength=len(part) * 256).reshape(-1, 256)
    return scipy.stats.entropy(counts, base=2, axis=1)


def windows_loop(arr: np.ndarray, window: int) -> list[float]:
    """窓ごとに numpy.bincount と scipy.stats.entropy を呼ぶ（よく見かける書き方）。"""
    return [
        scipy.stats.entropy(np.bincount(arr[off:off + window], minlength=256), base=2)
        for off in range(0, len(arr) - window + 1, window)
    ]


def bench(mode: str, window: int, sizes: list[int]) -> None:
    assert mode == "windows"
    points = []
    rng = np.random.default_rng(20260927)
    for size in sizes:
        arr = rng.integers(0, 256, size, dtype=np.uint8)  # 中身は速さにほとんど関係しない（Rust 側は別の乱数）
        # 念のため、まとめて計算したものが窓ごとの計算と（丸めの差を除いて）同じことを確かめる
        vec = windows_vectorized(arr, window)
        loop = windows_loop(arr[: min(size, 64 * window)], window)
        assert np.allclose(vec[: len(loop)], loop, rtol=1e-12, atol=0)
        points.append({
            "input_size": size,
            "reference_ms": median_ms(lambda: windows_vectorized(arr, window)),
            "loop_ms": median_ms(lambda: windows_loop(arr, window), min_runs=3, min_total_ms=0.0),
            "whole_ms": median_ms(lambda: scipy.stats.entropy(np.bincount(arr, minlength=256), base=2)),
        })
    print(json.dumps(points))


if __name__ == "__main__":
    command, *args = sys.argv[1:]
    if command == "cases":
        cases(int(args[0]), int(args[1]))
    elif command == "bench":
        bench(args[0], int(args[1]), [int(a) for a in args[2:]])
    else:
        sys.exit(f"unknown command: {command}")
