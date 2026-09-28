"""kura_rs.multi_pattern_match の使い方（型・エラー・スレッド）のテスト。"""

import array
import threading
from concurrent.futures import ThreadPoolExecutor

import pytest

from kura_rs import multi_pattern_match
from kura_rs.multi_pattern_match import Matcher


def test_he_she_his_hers():
    m = Matcher(["he", "she", "his", "hers"])
    assert m.find_all("ushers") == [(1, 1, 4), (0, 2, 4), (3, 2, 6)]
    assert m.find_longest("ushers") == [(1, 1, 4)]
    assert m.count("ushers") == 3
    assert m.is_match("ushers")
    assert not m.is_match("user")


def test_str_offsets_are_characters():
    m = Matcher(["東京", "😀", "log"])
    text = "ログ😀東京 log"
    hits = m.find_all(text)
    assert hits == [(1, 2, 3), (0, 3, 5), (2, 6, 9)]
    assert [text[s:e] for _, s, e in hits] == ["😀", "東京", "log"]


def test_bytes_offsets_are_bytes():
    m = Matcher(["東京".encode(), b"\x00\xff"])
    data = "ログ東京".encode() + b"\x00\xff"
    assert m.find_all(data) == [(0, 6, 12), (1, 12, 14)]
    assert m.kind == "bytes"
    assert m.patterns == ("東京".encode(), b"\x00\xff")


def test_empty_and_duplicate_patterns():
    m = Matcher(["", "x", "y", "x", ""])
    assert len(m) == 5
    assert m.find_all("xyx") == [(1, 0, 1), (2, 1, 2), (1, 2, 3)]
    assert Matcher([""]).find_all("abc") == []
    assert not Matcher([""]).is_match("abc")


def test_no_patterns_accepts_any_haystack():
    m = Matcher([])
    assert m.kind == "empty"
    assert len(m) == 0
    assert m.find_all("abc") == [] and m.find_all(b"abc") == [] and m.find_longest(bytearray(b"a")) == []
    assert m.count("") == 0
    with pytest.raises(TypeError):
        m.find_all(123)


def test_empty_haystack():
    m = Matcher(["a"])
    assert m.find_all("") == []
    assert m.find_longest("") == []
    assert not m.is_match("")


def test_any_iterable_of_patterns():
    assert Matcher(p for p in ["a", "b"]).find_all("ab") == [(0, 0, 1), (1, 1, 2)]
    assert Matcher(("a",)).patterns == ("a",)
    assert Matcher({"a": 1}).find_all("a") == [(0, 0, 1)]


def test_buffer_types():
    m = Matcher([b"ab"])
    assert m.find_all(bytearray(b"xab")) == [(0, 1, 3)]
    assert m.find_all(memoryview(b"xxab")[1:]) == [(0, 1, 3)]
    assert m.find_all(array.array("B", b"ab")) == [(0, 0, 2)]
    assert Matcher([bytearray(b"ab")]).patterns == (b"ab",)


def test_mixing_str_and_bytes_is_an_error():
    with pytest.raises(TypeError, match="all str or all bytes-like"):
        Matcher(["a", b"b"])
    with pytest.raises(TypeError, match="all str or all bytes-like"):
        Matcher([b"a", "b"])
    with pytest.raises(TypeError, match="must be str"):
        Matcher(["a"]).find_all(b"a")
    with pytest.raises(TypeError, match="bytes-like"):
        Matcher([b"a"]).find_all("a")


def test_bad_arguments():
    with pytest.raises(TypeError, match="iterable of patterns"):
        Matcher("abc")
    with pytest.raises(TypeError, match="iterable of patterns"):
        Matcher(b"abc")
    with pytest.raises(TypeError):
        Matcher(5)
    with pytest.raises(TypeError, match="pattern 1"):
        Matcher([b"a", 5])
    with pytest.raises(TypeError):
        Matcher(["a"]).find_all(None)
    with pytest.raises(OverflowError):
        Matcher(["a"]).find_all("a", limit=-1)
    with pytest.raises(TypeError):
        Matcher(["a"]).find_all("a", 1)  # limit はキーワードだけ


def test_surrogates_cannot_be_searched():
    # UTF-8 にできない文字列（孤立したサロゲート）はエラーにする
    with pytest.raises(UnicodeEncodeError):
        Matcher(["\ud800"])
    with pytest.raises(UnicodeEncodeError):
        Matcher(["a"]).find_all("a\udc00")


def test_limit():
    m = Matcher(["a", "aa"])
    assert m.find_all("aaa", limit=0) == []
    assert m.find_all("aaa", limit=2) == [(0, 0, 1), (1, 0, 2)]
    assert m.find_all("aaa", limit=None) == m.find_all("aaa")


def test_adversarial_prefix_chain_counts_without_a_list():
    m = Matcher(["a" * k for k in range(1, 101)])
    n = 100_000
    assert m.count("a" * n) == sum(min(end, 100) for end in range(1, n + 1))
    assert m.find_all("a" * n, limit=5) == [(0, 0, 1), (1, 0, 2), (0, 1, 2), (2, 0, 3), (1, 1, 3)]


def test_repr_and_properties():
    m = Matcher(["a", "b"])
    assert repr(m) == "Matcher(2 str patterns)"
    assert m.kind == "str"
    assert m.memory_usage > 0
    assert multi_pattern_match.__all__ == ["Matcher"]


def test_threads_share_one_matcher():
    # 1 つのマッチャーを複数のスレッドで同時に使っても同じ結果（大きな入力では GIL を手放している）
    m = Matcher([f"ioc-{i}.example" for i in range(1000)])
    log = " ".join(f"ioc-{i % 1000}.example" if i % 7 == 0 else f"host-{i}" for i in range(20_000))
    expected = m.find_all(log)
    with ThreadPoolExecutor(8) as pool:
        # 最左最長の automaton は最初に使うときに作るので、それも同時に起きるようにする
        results = list(pool.map(lambda _: (m.find_longest(log), m.find_all(log), m.count(log)), range(32)))
    for longest, all_hits, count in results:
        assert all_hits == expected
        assert count == len(expected)
        assert longest == results[0][0]
    assert len(expected) > 1000


def test_gil_is_released_for_large_haystacks():
    # 大きな入力を探している間に、別のスレッドが Python のコードを動かせる。
    # 入力のどのバイトもパターンに含まれる（先読みで飛ばせない）ので、探すのに数十ミリ秒かかる
    m = Matcher([b"abababababababb"])
    haystack = b"ab" * (32 << 20)
    ticks = []
    started = threading.Event()
    done = threading.Event()

    def tick():
        started.set()
        while not done.is_set():
            ticks.append(1)

    t = threading.Thread(target=tick)
    t.start()
    started.wait()
    try:
        before = len(ticks)
        assert m.count(haystack) == 0
        after = len(ticks)
    finally:
        done.set()
        t.join()
    assert after > before
