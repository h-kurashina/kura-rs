"""unicode-normalize の参照実装側。verify/run.py から呼ばれる。

正規化は Python の unicodedata.normalize（中身は C）と、Unicode 公式の NormalizationTest.txt に突き合わせる。
空白のまとめと見えない文字の除去は、下の fold_whitespace / strip_invisible（部品の説明と同じ定義）と比べる。

fetch                    : NormalizationTest.txt（Unicode 16.0.0）を取得して verify/data/ に置く（SHA-256 で固定）
cases <n> <seed>         : 差分テストのケースと答えを JSON Lines で標準出力に書く（n はランダムなケースの数）
bench <what> <size>...   : 入力の大きさ（UTF-8 のバイト数）ごとの処理時間（ミリ秒の中央値）を JSON で書く。
                           what は NFKC（unicodedata.normalize だけ）か pipeline（見えない文字の除去 → NFKC → 空白のまとめ）

NormalizationTest.txt は Unicode License v3 で再配布もできるが、2.8 MB あるのでリポジトリには入れず、
取得したものを URL と SHA-256 で固定して verify/data/（.gitignore 済み）に置く。
"""

import hashlib
import json
import random
import re
import statistics
import sys
import time
import unicodedata
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
DATA_DIR = ROOT / "verify" / "data" / "unicode-normalize"

# 部品（unicode-normalization 0.1.24）と Python 3.14 の unicodedata の Unicode の版
UNICODE_VERSION = "16.0.0"
NORMALIZATION_TEST = {
    "url": f"https://www.unicode.org/Public/{UNICODE_VERSION}/ucd/NormalizationTest.txt",
    "sha256": "d811971453e7075e1ad56fb1b301eece5aa80757b81f6156e74a1bfb3ae5ceb1",
    "license": "Unicode License v3, https://www.unicode.org/license.txt",
}
FORMS = ["NFC", "NFD", "NFKC", "NFKD"]

# ---- 部品と同じ定義（parts/unicode_normalize/mod.rs の説明を参照） ----

# 見えない文字：Unicode 16.0.0 で General_Category = Cf かつ Default_Ignorable_Code_Point のもの（両端を含む範囲）
INVISIBLE = [
    (0x00AD, 0x00AD), (0x061C, 0x061C), (0x180E, 0x180E), (0x200B, 0x200F), (0x202A, 0x202E),
    (0x2060, 0x2064), (0x2066, 0x206F), (0xFEFF, 0xFEFF), (0x1BCA0, 0x1BCA3), (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001), (0xE0020, 0xE007F),
]
# Cf のうち Default_Ignorable ではないもの（目に見える書式文字。消さない）。INVISIBLE と合わせるとちょうど Cf 全体になる
VISIBLE_FORMAT = [
    (0x0600, 0x0605), (0x06DD, 0x06DD), (0x070F, 0x070F), (0x0890, 0x0891), (0x08E2, 0x08E2),
    (0xFFF9, 0xFFFB), (0x110BD, 0x110BD), (0x110CD, 0x110CD), (0x13430, 0x1343F),
]
INVISIBLE_RE = re.compile("[" + "".join(f"\\U{lo:08x}-\\U{hi:08x}" for lo, hi in INVISIBLE) + "]")


def strip_invisible(text: str) -> str:
    return INVISIBLE_RE.sub("", text)


def fold_whitespace(text: str) -> str:
    """空白（str.isspace() が真の文字）の連続を U+0020 1つにし、両端の空白を消す。"""
    return " ".join(text.split())


def pipeline(text: str, form: str | None, fold: bool, strip: bool) -> str:
    """見えない文字の除去 → 正規化 → 空白のまとめ（この順番）。"""
    if strip:
        text = strip_invisible(text)
    if form:
        text = unicodedata.normalize(form, text)
    if fold:
        text = fold_whitespace(text)
    return text


def expand(ranges) -> list[int]:
    return [c for lo, hi in ranges for c in range(lo, hi + 1)]


ALL_CODE_POINTS = [c for c in range(0x110000) if not 0xD800 <= c <= 0xDFFF]  # サロゲートは Rust の文字列に入らない


# Unicode 17.0.0 で正規化の結果が変わる文字（unicode-normalization 0.1.24 と 0.1.25 を全符号位置で比べて見つけたもの）。
# U+A7F1 は互換分解（NFKC で "S"）が新しく付き、残りは結合クラスが 0 でなくなる（16.0 では未割り当て）。
# 版のずれを見張るため、これらを結合文字と並べたケースを入れる（0.1.25 を使うと不一致になる）
UNICODE_17_CHANGES = [*range(0x1ACF, 0x1ADE), *range(0x1AE0, 0x1AEC), 0xA7F1, 0x10EFA, 0x10EFB, 0x1E6E3, 0x1E6E6, 0x1E6EE, 0x1E6EF, 0x1E6F5]


def check_environment() -> None:
    if unicodedata.unidata_version != UNICODE_VERSION:
        sys.exit(
            f"this Python's unicodedata implements Unicode {unicodedata.unidata_version}, "
            f"but the part (unicode-normalization 0.1.24) implements Unicode {UNICODE_VERSION}; "
            "use Python 3.14 (Unicode 16.0.0), or move both to the same version"
        )
    # 見えない文字の定義を、2 通りの書き方（範囲の表と「Cf から見える書式文字を除いたもの」）で突き合わせる
    cf = {c for c in ALL_CODE_POINTS if unicodedata.category(chr(c)) == "Cf"}
    assert set(expand(INVISIBLE)) == cf - set(expand(VISIBLE_FORMAT)), "INVISIBLE is not Cf minus VISIBLE_FORMAT"
    assert set(expand(VISIBLE_FORMAT)) <= cf


# ---- NormalizationTest.txt ----

def normalization_test_path() -> Path:
    return DATA_DIR / f"NormalizationTest-{UNICODE_VERSION}.txt"


def fetch() -> None:
    path = normalization_test_path()
    if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == NORMALIZATION_TEST["sha256"]:
        return
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    request = urllib.request.Request(NORMALIZATION_TEST["url"], headers={"User-Agent": "kura-rs verify (https://github.com/h-kurashina/kura-rs)"})
    with urllib.request.urlopen(request, timeout=60) as response:
        data = response.read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != NORMALIZATION_TEST["sha256"]:
        sys.exit(f"{NORMALIZATION_TEST['url']}: SHA-256 is {digest}, expected {NORMALIZATION_TEST['sha256']}")
    path.write_bytes(data)
    print(f"fetched {NORMALIZATION_TEST['url']} -> {path.relative_to(ROOT)}")


def normalization_test() -> tuple[list[tuple[str, list[str]]], set[int]]:
    """(Part 名, [c1..c5]) の列と、Part 1 に載っている文字の集合。"""
    path = normalization_test_path()
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != NORMALIZATION_TEST["sha256"]:
        sys.exit(f"{path} is missing or changed; run: python verify/unicode-normalize/reference.py fetch")
    lines, part1, part = [], set(), None
    for raw in data.decode("utf-8").splitlines():
        line = raw.split("#")[0].strip()
        if line.startswith("@"):
            part = line[1:].strip()
            continue
        if not line:
            continue
        cols = ["".join(chr(int(h, 16)) for h in col.split()) for col in line.split(";")[:5]]
        lines.append((part, cols))
        if part == "Part1":
            part1.add(ord(cols[0]))
    return lines, part1


def conformance(cols: list[str]) -> list[tuple[str, str, str]]:
    """NormalizationTest.txt の先頭に書かれた条件を (入力, 形, 期待値) の組にしたもの。"""
    c1, c2, c3, c4, c5 = cols
    out = []
    out += [(x, "NFC", c2) for x in (c1, c2, c3)] + [(x, "NFC", c4) for x in (c4, c5)]
    out += [(x, "NFD", c3) for x in (c1, c2, c3)] + [(x, "NFD", c5) for x in (c4, c5)]
    out += [(x, "NFKC", c4) for x in cols]
    out += [(x, "NFKD", c5) for x in cols]
    return out


# ---- ランダム・意地悪な入力 ----

def pool(*ranges) -> list[int]:
    return [c for c in expand(ranges) if not 0xD800 <= c <= 0xDFFF]


COMBINING = [c for c in ALL_CODE_POINTS if unicodedata.combining(chr(c))]
WHITESPACE = [c for c in ALL_CODE_POINTS if chr(c).isspace()]
UNASSIGNED = [c for c in ALL_CODE_POINTS if unicodedata.category(chr(c)) == "Cn"]
POOLS = {
    "ascii": pool((0x00, 0x7F)),
    "latin": pool((0xA0, 0x24F), (0x1E00, 0x1EFF)),
    "combining": COMBINING,
    "hangul_syllable": pool((0xAC00, 0xD7A3)),
    "hangul_jamo": pool((0x1100, 0x11FF), (0xA960, 0xA97F), (0xD7B0, 0xD7FF), (0x3131, 0x318E), (0xFFA0, 0xFFDC)),
    "kana": pool((0x3040, 0x30FF), (0x31F0, 0x31FF)),
    "cjk": pool((0x4E00, 0x4E80), (0x3400, 0x3420), (0x20000, 0x20020)),
    "cjk_compat": pool((0xF900, 0xFAFF), (0x2F800, 0x2FA1F)),
    "fullwidth": pool((0xFF01, 0xFF5E), (0xFF61, 0xFF9F), (0xFFE0, 0xFFEE)),
    "ligature": pool((0xFB00, 0xFB06), (0xFB13, 0xFB17)),
    "arabic": pool((0x0600, 0x06FF), (0x0750, 0x077F), (0x08A0, 0x08FF), (0xFB50, 0xFDFF), (0xFE70, 0xFEFC)),
    "hebrew": pool((0x0591, 0x05F4), (0xFB1D, 0xFB4F)),
    "indic": pool((0x0900, 0x0DFF), (0x11000, 0x1137F)),
    "tibetan": pool((0x0F00, 0x0FFF)),
    "greek": pool((0x0370, 0x03FF), (0x1F00, 0x1FFF)),
    "emoji": pool((0x1F300, 0x1FAFF), (0x2600, 0x27BF), (0x1F1E6, 0x1F1FF)),
    "compat_misc": pool((0x2070, 0x209F), (0x2100, 0x218F), (0x2460, 0x24FF), (0x3200, 0x33FF), (0x1D400, 0x1D7FF), (0x1F100, 0x1F1AD)),
    "whitespace": WHITESPACE,
    "invisible": expand(INVISIBLE),
    # 見えないが消さない文字（異体字セレクタ・CGJ・フィラー・見える書式文字・未割り当ての Default_Ignorable など）
    "near_invisible": pool((0xFE00, 0xFE0F), (0xE0100, 0xE01EF), (0x034F, 0x034F), (0x115F, 0x1160), (0x3164, 0x3164),
                           (0xFFA0, 0xFFA0), (0x180B, 0x180F), (0x2065, 0x2065), (0xE0000, 0xE0000), (0xE0002, 0xE001F),
                           (0xE0080, 0xE00FF), (0x17B4, 0x17B5), *VISIBLE_FORMAT),
    "unassigned": UNASSIGNED,
}
POOL_NAMES = list(POOLS)
EMOJI_SEQUENCES = [
    "👨‍👩‍👧‍👦", "👩🏽‍💻", "🏳️‍🌈", "❤️‍🔥", "🧑🏿‍🤝‍🧑🏻",
    "🏴\U000E0067\U000E0062\U000E0065\U000E006E\U000E0067\U000E007F", "🇯🇵🇺🇸", "1️⃣", "#️⃣", "👍🏽", "☺︎", "☺️",
]


def any_char(rng: random.Random) -> int:
    c = rng.randrange(0x110000 - 0x800)
    return c + 0x800 if c >= 0xD800 else c


def pick(rng: random.Random, name: str) -> str:
    return chr(rng.choice(POOLS[name]))


def random_text(rng: random.Random) -> tuple[str, str]:
    kind = rng.choices(
        ["mix", "combining", "long_combining", "lone_combining", "hangul", "emoji", "hebrew", "arabic",
         "whitespace", "all_planes", "japanese", "same_pool"],
        weights=[20, 10, 2, 3, 8, 6, 5, 5, 10, 8, 8, 6],
    )[0]
    length = rng.choice([0, 1, 2, 3, rng.randrange(0, 20), rng.randrange(0, 100), rng.randrange(0, 1000)])
    if kind == "mix":
        text = "".join(pick(rng, rng.choice(POOL_NAMES)) for _ in range(length))
    elif kind == "same_pool":
        name = rng.choice(POOL_NAMES)
        text = "".join(pick(rng, name) for _ in range(length))
    elif kind == "combining":
        # 土台の文字 + 結合文字をいくつか（順番はでたらめ）の繰り返し
        parts = []
        for _ in range(max(1, length // 4)):
            base = pick(rng, rng.choice(["ascii", "latin", "greek", "hebrew", "arabic", "indic", "kana", "hangul_syllable"]))
            parts.append(base + "".join(pick(rng, "combining") for _ in range(rng.randrange(0, 8))))
        text = "".join(parts)
    elif kind == "long_combining":
        # とても長い結合文字の並び（数百〜数千個。同じ文字の繰り返しと、クラスの違う文字の混ぜ合わせ）
        n = rng.randrange(1000, 5000) if rng.random() < 0.1 else rng.randrange(30, 1000)
        marks = [pick(rng, "combining") for _ in range(rng.randrange(1, 6))]
        text = rng.choice(["", "a", "e", "א", "あ", "가"]) + "".join(rng.choice(marks) for _ in range(n))
    elif kind == "lone_combining":
        text = "".join(pick(rng, "combining") for _ in range(max(1, length)))
    elif kind == "hangul":
        # L・V・T 字母と音節を、正しい順番とでたらめな順番の両方で
        jamo = [pool((0x1100, 0x1112)), pool((0x1161, 0x1175)), pool((0x11A8, 0x11C2)),
                pool((0x1113, 0x115F), (0x1176, 0x11A7), (0x11C3, 0x11FF)), POOLS["hangul_syllable"], pool((0x3131, 0x318E))]
        parts = []
        for _ in range(max(1, length // 2)):
            if rng.random() < 0.5:
                parts.append(chr(rng.choice(jamo[0])) + chr(rng.choice(jamo[1])) + (chr(rng.choice(jamo[2])) if rng.random() < 0.5 else ""))
            else:
                parts.append(chr(rng.choice(rng.choice(jamo))))
        text = "".join(parts)
    elif kind == "emoji":
        parts = []
        for _ in range(max(1, length // 3)):
            r = rng.random()
            if r < 0.4:
                parts.append(rng.choice(EMOJI_SEQUENCES))
            elif r < 0.7:
                parts.append(pick(rng, "emoji") + rng.choice(["", "‍", "️", "︎", chr(rng.randrange(0x1F3FB, 0x1F400))]))
            else:
                parts.append(pick(rng, rng.choice(["emoji", "invisible", "near_invisible", "whitespace", "ascii"])))
        text = "".join(parts)
    elif kind == "hebrew":
        letters = pool((0x05D0, 0x05EA))
        points = pool((0x0591, 0x05C7))
        text = "".join(chr(rng.choice(letters)) + "".join(chr(rng.choice(points)) for _ in range(rng.randrange(0, 4)))
                       for _ in range(max(1, length // 2)))
    elif kind == "arabic":
        letters = pool((0x0621, 0x064A), (0xFB50, 0xFDFF), (0xFE70, 0xFEFC))
        marks = pool((0x064B, 0x065F), (0x0670, 0x0670))
        text = "".join(chr(rng.choice(letters)) + "".join(chr(rng.choice(marks)) for _ in range(rng.randrange(0, 3)))
                       for _ in range(max(1, length // 2)))
    elif kind == "whitespace":
        text = "".join(pick(rng, rng.choices(["whitespace", "invisible", "near_invisible", "ascii", "kana", "combining"],
                                             weights=[5, 3, 1, 4, 2, 1])[0]) for _ in range(length))
    elif kind == "all_planes":
        text = "".join(chr(any_char(rng)) for _ in range(length))
    else:  # japanese
        fragments = json.loads((HERE / "bench_fragments.json").read_text())["fragments"]
        text = "".join(rng.choice(fragments) for _ in range(max(1, length // 30)))
        if rng.random() < 0.5:  # ところどころに癖のある文字を差し込む
            chars = list(text)
            for _ in range(rng.randrange(1, 10)):
                chars.insert(rng.randrange(len(chars) + 1), pick(rng, rng.choice(POOL_NAMES)))
            text = "".join(chars)
    return kind, text


def case(source: str, text: str, form: str | None, fold: bool, strip: bool, expected: str) -> dict:
    return {
        "source": source,
        "text": text,
        "form": form,
        "fold_whitespace": fold,
        "strip_invisible": strip,
        "unicode_version": unicodedata.unidata_version,
        "expected": expected,
    }


def per_char_case(source: str, lo: int, hi: int, form: str | None, fold: bool, strip: bool, expected: str) -> dict:
    """lo..=hi のすべての文字（サロゲートを除く）を1文字ずつ処理してつなげたものが expected になるケース。
    JSON を小さくするため、入力は範囲で、期待値は UTF-8 の SHA-1（16 進）で渡す。"""
    return {
        "source": source,
        "range": [lo, hi],
        "form": form,
        "fold_whitespace": fold,
        "strip_invisible": strip,
        "unicode_version": unicodedata.unidata_version,
        "expected": hashlib.sha1(expected.encode()).hexdigest(),
    }


def cases(n: int, seed: int) -> None:
    check_environment()
    rng = random.Random(seed)
    out = []

    # 1. 公式の NormalizationTest.txt（重複を除いた (入力, 形, 期待値)）。Python もこの答えと一致するかを確かめる
    lines, part1 = normalization_test()
    seen = set()
    python_mismatch = []
    for part, cols in lines:
        for text, form, expected in conformance(cols):
            if (text, form) in seen:
                continue
            seen.add((text, form))
            if unicodedata.normalize(form, text) != expected:
                python_mismatch.append((part, form, text))
            out.append(case(f"NormalizationTest {part}", text, form, False, False, expected))
    if python_mismatch:
        sys.exit(f"unicodedata disagrees with NormalizationTest.txt in {len(python_mismatch)} cases, e.g. {python_mismatch[:3]!r}")

    # 2. すべての符号位置を1文字ずつ（4096 文字ずつまとめる）。
    #    正規化は Part 1 の答え（載っていない文字は変わらない）と、unicodedata の答えの両方と比べる
    part1_answers = {}
    for part, cols in lines:
        if part == "Part1":
            c1, c2, c3, c4, c5 = cols
            part1_answers[c1] = {"NFC": c2, "NFD": c3, "NFKC": c4, "NFKD": c5}
    per_char_ops = [(form, False, False) for form in FORMS] + [
        (None, True, False), (None, False, True), ("NFKC", True, True), ("NFC", True, True),
    ]
    for start in range(0, len(ALL_CODE_POINTS), 4096):
        chunk = [chr(c) for c in ALL_CODE_POINTS[start:start + 4096]]
        lo, hi = ord(chunk[0]), ord(chunk[-1])
        label = f"U+{lo:04X}..U+{hi:04X}"
        for form in FORMS:
            expected = "".join(part1_answers.get(ch, {}).get(form, ch) for ch in chunk)
            out.append(per_char_case(f"NormalizationTest Part1 invariance {label}", lo, hi, form, False, False, expected))
        for form, fold, strip in per_char_ops:
            expected = "".join(pipeline(ch, form, fold, strip) for ch in chunk)
            out.append(per_char_case(f"every code point {label}", lo, hi, form, fold, strip, expected))

    # 3. Unicode 16.0 と 17.0 で答えが変わる文字を、結合文字の前後に置いたもの
    for c in UNICODE_17_CHANGES:
        x = chr(c)
        for text in [x, "a" + x + "\u0323", "a\u0323" + x, "\u05d0\u05b8" + x + "\u05b4", "S" + x + "\u0301"]:
            for form in FORMS:
                out.append(case("Unicode 17 changes", text, form, False, False, unicodedata.normalize(form, text)))

    # 4. ランダム・意地悪な入力。1つの文字列を 4 つの形・空白のまとめ・見えない文字の除去・でたらめな組み合わせ 2 つで
    count = 0
    while count < n:
        kind, text = random_text(rng)
        ops = [(form, False, False) for form in FORMS] + [(None, True, False), (None, False, True)]
        ops += [(rng.choice([None, *FORMS]), rng.random() < 0.5, rng.random() < 0.5) for _ in range(2)]
        for form, fold, strip in ops[: n - count]:
            out.append(case(f"random {kind}", text, form, fold, strip, pipeline(text, form, fold, strip)))
            count += 1

    # 期待値を壊すチェック（run.py は先頭の 2000 件を使う）がすべての種類に当たるよう、順番を混ぜる
    rng.shuffle(out)
    for c in out:
        print(json.dumps(c))


# ---- ベンチマーク ----

def bench_text(max_size: int) -> bytes:
    """Rust 側（crates/kura-verify/src/bin/unicode-normalize.rs）と同じ手順で、max_size バイト以上の文章を作る。"""
    spec = json.loads((HERE / "bench_fragments.json").read_text())
    fragments = [f.encode() for f in spec["fragments"]]
    state, parts, size = spec["seed"], [], 0
    while size < max_size:
        state = (state * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
        fragment = fragments[(state >> 33) % len(fragments)]
        parts.append(fragment)
        size += len(fragment)
    return b"".join(parts)


def prefix(data: bytes, size: int) -> str:
    """先頭 size バイト（文字の途中で切れる場合はその文字の前まで）。"""
    return data[:size].decode("utf-8", errors="ignore")


def median_ms(f, min_runs=5, min_total_ms=300.0, max_runs=1000) -> float:
    samples, total = [], 0.0
    while len(samples) < min_runs or (total < min_total_ms and len(samples) < max_runs):
        start = time.perf_counter()
        f()
        ms = (time.perf_counter() - start) * 1000
        samples.append(ms)
        total += ms
    return statistics.median(samples)


def bench(what: str, sizes: list[int]) -> None:
    data = bench_text(max(sizes))
    points = []
    for size in sizes:
        text = prefix(data, size)
        if what == "NFKC":
            def run():
                unicodedata.normalize("NFKC", text)
        elif what == "pipeline":
            def run():
                pipeline(text, "NFKC", True, True)
        else:
            sys.exit(f"unknown benchmark: {what}")
        points.append({
            "input_size": size,
            "reference_ms": median_ms(run),
            "text_sha1": hashlib.sha1(text.encode()).hexdigest(),
        })
    print(json.dumps(points))


if __name__ == "__main__":
    command, *args = sys.argv[1:]
    if command == "fetch":
        fetch()
    elif command == "cases":
        cases(int(args[0]), int(args[1]))
    elif command == "bench":
        bench(args[0], [int(a) for a in args[1:]])
    else:
        sys.exit(f"unknown command: {command}")
