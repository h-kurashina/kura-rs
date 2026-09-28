"""kura_rs.multi_pattern_match を pyahocorasick と、ランダムな入力で突き合わせる。

pyahocorasick（pip の版は str を扱うビルド）の Automaton.iter() は (最後の文字の位置, 値) を返す。
ここでは (pattern_id, start, end) に直して比べる。バイト列は latin-1 で 1 バイト = 1 文字に写して渡す。
最左最長は、iter() のすべての一致から選んだものと比べる（pyahocorasick の iter_long() は一致を取りこぼすことがあるため）。
"""

import random

import ahocorasick
from hypothesis import given
from hypothesis import strategies as st

from kura_rs import multi_pattern_match


def automaton(patterns):
    A = ahocorasick.Automaton()
    for i, p in enumerate(patterns):
        if p and not A.exists(p):  # 空は登録されない。重複は最初の id を残す
            A.add_word(p, i)
    if len(A) == 0:
        return None
    A.make_automaton()
    return A


def reference(patterns, haystack):
    """pyahocorasick の iter() を (pattern_id, start, end) の列にしたもの。"""
    if isinstance(haystack, (bytes, bytearray, memoryview)):
        patterns = [bytes(p).decode("latin-1") for p in patterns]
        haystack = bytes(haystack).decode("latin-1")
    A = automaton(patterns)
    if A is None:
        return []
    return [(v, e - len(patterns[v]) + 1, e + 1) for e, v in A.iter(haystack)]


def leftmost_longest(hits):
    out, pos = [], 0
    for pid, start, end in sorted(hits, key=lambda h: (h[1], -(h[2] - h[1]))):
        if start >= pos:
            out.append((pid, start, end))
            pos = end
    return out


def check(patterns, haystack):
    m = multi_pattern_match.Matcher(patterns)
    expected = reference(patterns, haystack)
    got = m.find_all(haystack)
    assert got == expected
    assert m.find_longest(haystack) == leftmost_longest(expected)
    assert m.count(haystack) == len(expected)
    assert m.is_match(haystack) == bool(expected)
    return got


# 小さなアルファベット：一致が重なり合い、互いに接頭辞・接尾辞のパターンが多い
small_text = st.text(alphabet="ab", max_size=6)
small_patterns = st.lists(st.one_of(small_text, st.text(alphabet="abc", max_size=4)), max_size=25)
small_haystack = st.text(alphabet="abc", max_size=300)


@given(small_patterns, small_haystack)
def test_small_alphabet_str(patterns, haystack):
    check(patterns, haystack)


@given(st.lists(st.text(max_size=6), max_size=20), st.data())
def test_unicode_str(patterns, data):
    # 任意の Unicode（サロゲートは除く）。入力にはパターンを混ぜて、一致が起きるようにする
    pieces = data.draw(st.lists(st.one_of(st.sampled_from(patterns or ["x"]), st.text(max_size=5)), max_size=40))
    check(patterns, "".join(pieces))


multilingual = st.sampled_from(["ログ", "東京", "侵害", "한국어", "пароль", "café", "café", "😀", "👩‍💻", "🇯🇵", "مرحبا", "a", "é", "́", "ﬁ", "\U0010ffff"])


@given(st.lists(multilingual, min_size=1, max_size=10), st.lists(st.one_of(multilingual, st.sampled_from([" ", "x", "\n"])), max_size=100))
def test_multilingual_offsets_are_characters(patterns, pieces):
    haystack = "".join(pieces)
    for pid, start, end in check(patterns, haystack):
        assert haystack[start:end] == patterns[pid]


@given(st.lists(st.binary(max_size=5), max_size=40), st.binary(max_size=500))
def test_arbitrary_bytes(patterns, haystack):
    check(patterns, haystack)


@given(st.lists(st.binary(min_size=1, max_size=4), min_size=1, max_size=20), st.data())
def test_bytes_with_planted_patterns(patterns, data):
    pieces = data.draw(st.lists(st.one_of(st.sampled_from(patterns), st.binary(max_size=4)), max_size=60))
    haystack = b"".join(pieces)
    for pid, start, end in check(patterns, haystack):
        assert haystack[start:end] == patterns[pid]


@given(st.lists(st.binary(max_size=4), max_size=20), st.binary(max_size=300))
def test_bytearray_and_memoryview(patterns, haystack):
    expected = reference(patterns, haystack)
    m = multi_pattern_match.Matcher([bytearray(p) for p in patterns])
    assert m.find_all(bytearray(haystack)) == expected
    assert m.find_all(memoryview(haystack)) == expected
    assert multi_pattern_match.Matcher([memoryview(p) for p in patterns]).find_all(haystack) == expected


@given(st.integers(0, 255).map(lambda b: chr(b)), st.integers(1, 60), st.integers(0, 400))
def test_prefix_chain_over_repetitive_haystack(ch, k, n):
    # a, aa, ..., a×k を a×n から探す（一致は位置ごとに最大 k 個）
    patterns = [ch * i for i in range(1, k + 1)]
    random.Random(k * 1000 + n).shuffle(patterns)
    got = check(patterns, ch * n)
    assert len(got) == sum(min(end, k) for end in range(1, n + 1))


@given(st.text(alphabet="abc", min_size=1, max_size=25), st.integers(0, 20), st.sampled_from(["prefixes", "suffixes", "substrings"]))
def test_affixes_of_one_string(s, repeat, which):
    if which == "prefixes":
        patterns = [s[:i] for i in range(1, len(s) + 1)]
    elif which == "suffixes":
        patterns = [s[i:] for i in range(len(s))]
    else:
        patterns = [s[i:j] for i in range(len(s)) for j in range(i + 1, len(s) + 1)]
    check(patterns, s * repeat)


@given(st.lists(st.text(alphabet="abcdef0123456789.", min_size=1, max_size=12), min_size=1, max_size=300), st.data())
def test_many_patterns(patterns, data):
    pieces = data.draw(st.lists(st.one_of(st.sampled_from(patterns), st.text(alphabet="abcdef0123456789. ", max_size=8)), max_size=100))
    check(patterns, "".join(pieces))


@given(small_patterns, small_haystack)
def test_large_pattern_sets_use_the_same_semantics(patterns, haystack):
    # 入力より長いパターンを足してパターンの合計を大きくすると、中の automaton が DFA から NFA に変わるが、結果は同じ
    check(patterns + ["z" * 40_000], haystack)


@given(small_patterns, small_haystack, st.randoms(use_true_random=False))
def test_independent_of_pattern_order(patterns, haystack, rnd):
    shuffled = list(patterns)
    rnd.shuffle(shuffled)
    a = multi_pattern_match.Matcher(patterns)
    b = multi_pattern_match.Matcher(shuffled)
    spans = lambda m, ps: [(s, e, ps[pid]) for pid, s, e in m.find_all(haystack)]  # noqa: E731
    assert spans(a, patterns) == spans(b, shuffled)
    longest = lambda m, ps: [(s, e, ps[pid]) for pid, s, e in m.find_longest(haystack)]  # noqa: E731
    assert longest(a, patterns) == longest(b, shuffled)


@given(small_patterns, small_haystack, st.integers(0, 50))
def test_limit_is_a_prefix(patterns, haystack, limit):
    m = multi_pattern_match.Matcher(patterns)
    assert m.find_all(haystack, limit=limit) == m.find_all(haystack)[:limit]


@given(st.lists(st.text(alphabet="ab", max_size=4), max_size=10), st.text(alphabet="ab", max_size=50), st.integers(1, 3000))
def test_long_ascii_and_non_ascii_haystacks(patterns, unit, repeat):
    # GIL を手放す大きさ（4 KiB）を超える入力でも同じ。非 ASCII の文字を混ぜて、文字位置の変換も通す
    check(patterns, unit * repeat)
    check(patterns + ["é"], ("é" + unit) * (repeat // 10 + 1))


def test_iocs_in_access_logs():
    rng = random.Random(20260927)
    iocs = sorted({f"{rng.randrange(256)}.{rng.randrange(256)}.{rng.randrange(256)}.{rng.randrange(256)}" for _ in range(500)})
    iocs += [f"evil-{i}.example" for i in range(500)]
    lines = []
    for i in range(5000):
        ip = rng.choice(iocs[:500]) if i % 37 == 0 else f"10.0.{rng.randrange(256)}.{rng.randrange(256)}"
        host = rng.choice(iocs[500:]) if i % 53 == 0 else "www.example.com"
        lines.append(f'{ip} - - [27/Sep/2026:10:00:00 +0000] "GET http://{host}/ HTTP/1.1" 200 {rng.randrange(9999)}')
    log = "\n".join(lines)
    hits = check(iocs, log)
    assert len(hits) > 100
    check([s.encode() for s in iocs], log.encode())
