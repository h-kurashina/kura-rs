"""multi-pattern-match の参照実装側。verify/run.py から呼ばれる。

参照実装は pyahocorasick（C 拡張）。pip で入る版は str（Unicode のコードポイント列）を扱うビルドなので、
バイト列は latin-1 で「1 バイト = 1 文字（U+0000〜U+00FF）」に写してから渡す。これで位置はバイト単位のまま一致する。
UTF-8 の文章のケースだけは str のまま pyahocorasick に渡し、返ってきた文字位置を UTF-8 のバイト位置に直して比べる。

pyahocorasick の決まり（こちらの部品も同じにしている）：
- 空のパターンは add_word が受け付けない（一致しない）
- 同じパターンをもう一度 add_word すると値が上書きされるので、2 回目以降は足さない（id は最初に出てきた位置）
- iter() は (一致の最後の文字の位置, 値) を、終わりの位置の順・同じ終わりなら長い順に返す

最左最長・重なりなし（部品の find_longest）の答えは、iter() が返したすべての一致から
「いちばん左で始まるもののうち最も長いものを取り、その終わりから続ける」で作る。
pyahocorasick には iter_long() があるが、一致を取りこぼすことがあるので答えには使わない
（例：パターン ["b", "abb"] で "ab" を探すと、b があるのに何も返さない。2.3.1 で確認）。

cases <n> <seed>             : 差分テストのケースと参照実装の答えを JSON Lines で標準出力に書く
bench <patterns> <size>...   : Rust 側（先に動く）が書いたパターンとログを読み、大きさごとの時間（ミリ秒の中央値）を JSON で書く
"""

import json
import random
import statistics
import string
import sys
import time
from pathlib import Path

import ahocorasick

ROOT = Path(__file__).resolve().parents[2]
BENCH_DIR = ROOT / "target" / "verify-data" / "multi-pattern-match"

# JSON に 16 進でそのまま入れる入力の上限（大きな繰り返しの入力は "repeat" で表す）
INLINE_MAX = 64 * 1024
# 1 ケースで報告される一致の数の上限の目安（JSON が大きくなりすぎないように）
MAX_MATCHES = 20_000


def automaton(patterns):
    A = ahocorasick.Automaton()
    for i, p in enumerate(patterns):
        if p and not A.exists(p):
            A.add_word(p, i)
    if len(A) == 0:
        return None  # 1 つも登録されていないと make_automaton の後でも iter できない
    A.make_automaton()
    return A


def leftmost_longest(matches: list[tuple[int, int]], lengths: dict[int, int]) -> list[list[int]]:
    """iter() のすべての一致 (末尾, id) から、重ならない最左最長の一致を選ぶ。"""
    spans = sorted(((e - lengths[v] + 1, -lengths[v], e, v) for e, v in matches))
    out, pos = [], 0
    for start, _, e, v in spans:
        if start >= pos:
            out.append([e, v])
            pos = e + 1
    return out


def reference_bytes(patterns: list[bytes], haystack: bytes):
    A = automaton([p.decode("latin-1") for p in patterns])
    if A is None:
        return [], []
    found = list(A.iter(haystack.decode("latin-1")))
    lengths = {v: len(patterns[v]) for _, v in found}
    return [[e, v] for e, v in found], leftmost_longest(found, lengths)


def reference_text(patterns: list[str], haystack: str):
    """str のまま pyahocorasick にかけ、文字位置を UTF-8 のバイト位置に直す。"""
    A = automaton(patterns)
    if A is None:
        return [], []
    # 文字 i の最後のバイトの位置
    last_byte, total = [], 0
    for ch in haystack:
        total += len(ch.encode("utf-8"))
        last_byte.append(total - 1)
    found = list(A.iter(haystack))
    lengths = {v: len(patterns[v]) for _, v in found}  # 文字数（位置を直す前に選ぶ）
    return (
        [[last_byte[e], v] for e, v in found],
        [[last_byte[e], v] for e, v in leftmost_longest(found, lengths)],
    )


def encode_haystack(haystack: bytes) -> dict:
    """大きな入力が短い並びの繰り返し（＋短い残り）なら "repeat" で表して JSON を小さくする。"""
    if len(haystack) > INLINE_MAX:
        for unit in range(1, 9):
            head = haystack[:unit]
            periods = 0
            while haystack.startswith(head * 4096, periods * unit):
                periods += 4096
            while haystack.startswith(head, periods * unit):
                periods += 1
            rest = haystack[periods * unit:]
            if len(rest) <= INLINE_MAX:
                return {"kind": "repeat", "hex": head.hex(), "count": periods, "tail": rest.hex()}
    return {"kind": "hex", "hex": haystack.hex()}


def emit(kind: str, patterns: list[bytes], haystack: bytes, text: tuple[list[str], str] | None = None) -> None:
    while True:
        if text is not None:
            expected, expected_long = reference_text(*text)
        else:
            expected, expected_long = reference_bytes(patterns, haystack)
        if len(expected) <= MAX_MATCHES:
            break
        # 一致が多すぎるケースは、入力を半分にして作り直す（JSON が大きくなりすぎないように）
        if text is not None:
            text = (text[0], text[1][: len(text[1]) // 2])
            haystack = text[1].encode("utf-8")
        else:
            haystack = haystack[: len(haystack) // 2]
    print(json.dumps({
        "kind": kind,
        "patterns": [p.hex() for p in patterns],
        "haystack": encode_haystack(haystack),
        "expected": expected,
        "expected_long": expected_long,
    }))


def emit_text(kind: str, patterns: list[str], haystack: str) -> None:
    emit(kind, [p.encode("utf-8") for p in patterns], haystack.encode("utf-8"), (patterns, haystack))


# --- ケースの作り方 ---

def with_noise(rng: random.Random, patterns: list[bytes]) -> list[bytes]:
    """重複と空パターンをときどき混ぜる。"""
    out = list(patterns)
    if out and rng.random() < 0.3:
        for _ in range(rng.randrange(1, 4)):
            out.insert(rng.randrange(len(out) + 1), rng.choice(out))
    if rng.random() < 0.15:
        out.insert(rng.randrange(len(out) + 1), b"")
    return out


def plant(rng: random.Random, haystack: bytearray, patterns: list[bytes], count: int) -> bytes:
    """パターンをいくつか入力に埋め込む（ランダムな入力だけだと長いパターンがほとんど一致しないため）。"""
    for _ in range(count):
        p = rng.choice(patterns)
        if p and len(p) <= len(haystack):
            at = rng.randrange(len(haystack) - len(p) + 1)
            haystack[at:at + len(p)] = p
    return bytes(haystack)


def random_over(rng: random.Random, alphabet: bytes, n: int) -> bytearray:
    return bytearray(rng.choices(alphabet, k=n))


def dense(rng: random.Random) -> None:
    """2〜4 種類のバイトだけを使う。一致が重なり合い、互いに接頭辞・接尾辞になるパターンが多い。"""
    alphabet = bytes(rng.sample(range(256), rng.randrange(2, 5)))
    patterns = [bytes(random_over(rng, alphabet, rng.randrange(1, 7))) for _ in range(rng.randrange(1, 31))]
    haystack = bytes(random_over(rng, alphabet, rng.randrange(0, 1000)))
    emit("dense", with_noise(rng, patterns), haystack)


def many(rng: random.Random) -> None:
    """1〜5,000 個のパターン（個数は対数で一様）。"""
    count = int(2 ** rng.uniform(0, 12.3))  # 1 〜 5,000
    alphabet = rng.choice([string.ascii_lowercase.encode(), b"0123456789abcdef", bytes(range(256)), b"ab"])
    lo = 1 if len(alphabet) > 2 else 4
    patterns = [bytes(random_over(rng, alphabet, rng.randrange(lo, 13))) for _ in range(count)]
    n = int(2 ** rng.uniform(0, 14))
    haystack = plant(rng, random_over(rng, alphabet, n), patterns, rng.randrange(0, 50))
    emit("many", with_noise(rng, patterns), haystack)


def prefix_chain(rng: random.Random) -> None:
    """a, aa, aaa, ... と aaaa... の入力。一致の数は 入力の長さ × パターンの数 に近くなる。"""
    b = rng.randrange(256)
    k = rng.randrange(1, 300)
    n = min(rng.randrange(0, 5000), MAX_MATCHES // k)
    patterns = [bytes([b]) * i for i in range(1, k + 1)]
    rng.shuffle(patterns)
    haystack = bytearray([b]) * n
    for _ in range(rng.randrange(0, 4)):  # 途中に別のバイトを挟んで、連続を切る
        if n:
            haystack[rng.randrange(n)] = (b + 1) % 256
    emit("prefix-chain", with_noise(rng, patterns), bytes(haystack))


def affixes(rng: random.Random) -> None:
    """1 本の文字列の接頭辞・接尾辞・部分文字列をすべてパターンにし、その文字列を並べた入力を探す。"""
    s = bytes(random_over(rng, rng.choice([b"ab", b"abc", string.ascii_lowercase.encode()]), rng.randrange(1, 40)))
    which = rng.choice(["prefixes", "suffixes", "substrings", "both"])
    if which == "prefixes":
        patterns = [s[:i] for i in range(1, len(s) + 1)]
    elif which == "suffixes":
        patterns = [s[i:] for i in range(len(s))]
    elif which == "both":
        patterns = [s[:i] for i in range(1, len(s) + 1)] + [s[i:] for i in range(len(s))]
    else:
        patterns = [s[i:j] for i in range(len(s)) for j in range(i + 1, len(s) + 1)]
    rng.shuffle(patterns)
    per_byte = max(1, len(patterns) // max(1, len(s)) * 2)
    haystack = s * rng.randrange(0, max(1, min(200, MAX_MATCHES // (per_byte * len(s) + 1))))
    emit(f"affixes-{which}", with_noise(rng, patterns), haystack)


def repetitive(rng: random.Random) -> None:
    """同じ短い並びを繰り返した入力。

    短いもの：繰り返しの単位の何倍かのパターン（どこでも一致する）。
    中くらい：入力とほぼ同じ長さの周期的なパターン（pyahocorasick は周期的な長いパターンで遅くなるので 8 KiB まで）。
    長いもの（1 MiB まで）：最後に 1 か所だけ違うバイトを入れ、そこにかかるパターンだけが一致する。
    """
    unit = bytes(random_over(rng, b"ab", rng.randrange(1, 5)))
    r = rng.random()
    if r < 0.4:
        n = rng.randrange(0, 1000)
        haystack = (unit * (n // len(unit) + 1))[:n]
        patterns = [unit * rng.randrange(1, 4), unit + b"c", b"c", unit[:1] * rng.randrange(1, 20)]
    elif r < 0.7:
        periods = rng.randrange(20, 2000)
        haystack = unit * periods
        d = rng.randrange(0, 20)  # 一致するのは d + 1 回だけ
        patterns = [unit + b"c", b"c" + unit, unit * max(1, periods - d), unit * (periods + 1)]
    else:
        periods = rng.randrange(1000, (1 << 20) // len(unit))
        tail = rng.randrange(0, 5)
        haystack = unit * periods + b"c" + unit * tail
        patterns = [unit * rng.randrange(1, 50) + b"c", b"c" + unit, unit + b"cc", b"d", unit * rng.randrange(1, 50) + b"d"]
    emit("repetitive", with_noise(rng, patterns), haystack)


def full_bytes(rng: random.Random) -> None:
    """0x00〜0xFF のどのバイトも出る入力とパターン（NUL や 0xFF、UTF-8 として不正な並びも含む）。"""
    patterns = [rng.randbytes(rng.randrange(1, 5)) for _ in range(rng.randrange(1, 200))]
    haystack = plant(rng, bytearray(rng.randbytes(rng.randrange(0, 20000))), patterns, rng.randrange(0, 30))
    emit("bytes", with_noise(rng, patterns), haystack)


def long_patterns(rng: random.Random) -> None:
    """1,000〜20,000 バイトのパターン。入力には本物と、最後の 1 バイトだけ違う偽物を埋め込む。"""
    patterns = [rng.randbytes(rng.randrange(1000, 20001)) for _ in range(rng.randrange(1, 5))]
    patterns += [p[: rng.randrange(1, len(p))] for p in patterns[:2]]  # 長いパターンの接頭辞
    pieces = []
    for _ in range(rng.randrange(1, 6)):
        p = rng.choice(patterns)
        pieces.append(rng.randbytes(rng.randrange(0, 500)))
        pieces.append(p if rng.random() < 0.6 else p[:-1] + bytes([(p[-1] + 1) % 256]))
    emit("long-patterns", with_noise(rng, patterns), b"".join(pieces))


TEXT_POOL = [
    "ログ", "東京", "侵害", "認証失敗", "パスワード", "管理者", "ｱｲｳ", "ﾛｸﾞ", "中文", "日志", "攻击", "한국어", "로그인", "비밀번호",
    "Привет", "пароль", "admin", "café", "café", "naïve", "Straße", "ǅ", "ﬁ", "😀", "🔥", "👩‍💻", "🇯🇵",
    "مرحبا", "שלום", "नमस्ते", "é", "e", "́", "a", "ab", "ба", " ", "　", "﻿", "𝔘𝔫𝔦", "\U0010ffff",
]


def text(rng: random.Random) -> None:
    """UTF-8 の文章。pyahocorasick には str のまま渡し、文字位置をバイト位置に直して比べる。"""
    words = rng.sample(TEXT_POOL, rng.randrange(1, len(TEXT_POOL)))
    patterns = []
    for _ in range(rng.randrange(1, 40)):
        w = rng.choice(words)
        if rng.random() < 0.3 and len(w) > 1:
            i = rng.randrange(len(w))
            w = w[i:] if rng.random() < 0.5 else w[: i + 1]  # 1 文字だけのパターンや、単語の一部も入れる
        patterns.append(w)
    if rng.random() < 0.3:
        patterns.append(rng.choice(patterns))
    if rng.random() < 0.1:
        patterns.append("")
    haystack = "".join(rng.choice(words + [" ", "\n", "x", "0"]) for _ in range(rng.randrange(0, 400)))
    emit_text("text", patterns, haystack)


def ioc_patterns(rng: random.Random, n: int) -> list[str]:
    out = []
    for _ in range(n):
        kind = rng.randrange(4)
        if kind == 0:
            out.append(f"{''.join(rng.choices(string.ascii_lowercase, k=rng.randrange(3, 10)))}.{rng.choice(['com', 'net', 'ru', 'xyz', 'top'])}")
        elif kind == 1:
            out.append(".".join(str(rng.randrange(256)) for _ in range(4)))
        elif kind == 2:
            out.append("".join(rng.choices("0123456789abcdef", k=rng.choice([32, 40, 64]))))
        else:
            out.append(f"/{rng.choice(['wp-admin', 'cgi-bin', 'tmp', '.git'])}/{''.join(rng.choices(string.ascii_lowercase, k=5))}.php")
    return out


def iocs(rng: random.Random) -> None:
    """IOC（ドメイン・IP・ハッシュ・パス）とアクセスログ風の入力。IP は互いに接頭辞になりやすい（1.2.3.4 と 1.2.3.45）。"""
    patterns = ioc_patterns(rng, rng.randrange(1, 500))
    lines = []
    for _ in range(rng.randrange(0, 200)):
        ip = rng.choice(patterns) if rng.random() < 0.1 else ".".join(str(rng.randrange(256)) for _ in range(4))
        path = rng.choice(patterns) if rng.random() < 0.1 else "/index.html"
        lines.append(f'{ip} - - [27/Sep/2026:10:00:00 +0000] "GET {path} HTTP/1.1" 200 {rng.randrange(99999)} "{rng.choice(patterns)}"')
    emit_text("iocs", patterns, "\n".join(lines))


def edge_cases() -> int:
    cases = [
        ([], b""), ([], b"abc"), ([b""], b"abc"), ([b"", b""], b""), ([b"a"], b""), ([b"abc"], b"ab"),
        ([b"abc"], b"abc"), ([b"a", b"a", b"a"], b"aaa"), ([b"he", b"she", b"his", b"hers"], b"ushers"),
        ([b"a", b"aa", b"aaa", b"b", b"ab", b"bab"], b"aaaabab"), ([b"abc", b"ab", b"a", b"bc", b"c"], b"abcabc"),
        ([b"\x00"], b"\x00\x00\x00"), ([b"\xff\xfe", b"\xfe"], b"\xff\xfe\xff\xfe"), ([b"ab", b"b", b"", b"ab"], b"abab"),
        ([b"x" * 1000], b"x" * 999), ([b"x" * 1000], b"x" * 1001), ([b"aab", b"ab", b"b"], b"aaab"),
        ([b"abcd", b"bc"], b"abce"), ([b"abcd", b"bcx", b"c"], b"abcx"), ([b"b", b"abc"], b"abc"),
    ]
    for patterns, haystack in cases:
        emit("edge", patterns, haystack)
    return len(cases)


GENERATORS = [dense, many, prefix_chain, affixes, repetitive, full_bytes, long_patterns, text, iocs]
WEIGHTS = [30, 20, 8, 10, 5, 12, 3, 8, 4]


def cases(n: int, seed: int) -> None:
    rng = random.Random(seed)
    count = edge_cases()
    # どの作り方も最低 20 回は使う
    for generator in GENERATORS:
        for _ in range(20):
            generator(rng)
            count += 1
    while count < n:
        rng.choices(GENERATORS, weights=WEIGHTS)[0](rng)
        count += 1


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


def bench(num_patterns: int, sizes: list[int]) -> None:
    patterns = json.loads((BENCH_DIR / "patterns.json").read_text())
    assert len(patterns) == num_patterns, "patterns.json does not match (run the Rust bench first)"
    data = (BENCH_DIR / "haystack.bin").read_bytes()
    rust_counts = {p["input_size"]: p["matches"] for p in json.loads((BENCH_DIR / "rust-counts.json").read_text())}
    A = automaton(patterns)
    points = []
    for size in sizes:
        haystack = data[:size].decode("latin-1")  # ログは ASCII なので、そのまま str になる

        def run():
            return list(A.iter(haystack))

        # 同じものを数えているかの確認（ずれていたら速さを比べる意味がない）
        count = len(run())
        if count != rust_counts[size]:
            sys.exit(f"match count differs at {size} bytes: pyahocorasick {count}, Rust {rust_counts[size]}")
        points.append({"input_size": size, "reference_ms": median_ms(run), "matches": count})
    print(json.dumps(points))


if __name__ == "__main__":
    command, *args = sys.argv[1:]
    if command == "cases":
        cases(int(args[0]), int(args[1]))
    elif command == "bench":
        bench(int(args[0]), [int(a) for a in args[1:]])
    else:
        sys.exit(f"unknown command: {command}")
