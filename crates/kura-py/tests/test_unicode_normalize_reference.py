"""kura_rs.unicode_normalize を unicodedata と、ランダムな文字列で突き合わせる。

空白のまとめと見えない文字の除去は、verify/unicode-normalize/reference.py と同じ定義（下の fold / strip）と比べる。
"""

import re
import unicodedata

import pytest
from hypothesis import given
from hypothesis import strategies as st

from kura_rs import unicode_normalize as un

# 部品の Unicode の版（16.0.0）と Python の unicodedata の版が違うと、新しく割り当てられた文字で答えが変わる
pytestmark = pytest.mark.skipif(
    unicodedata.unidata_version != un.UNICODE_VERSION,
    reason=f"unicodedata is Unicode {unicodedata.unidata_version}, kura_rs is {un.UNICODE_VERSION} (use Python 3.14)",
)

FORMS = ["NFC", "NFD", "NFKC", "NFKD"]
INVISIBLE_RE = re.compile("[" + "".join(f"\\U{lo:08x}-\\U{hi:08x}" for lo, hi in un.INVISIBLE) + "]")


def strip(text: str) -> str:
    return INVISIBLE_RE.sub("", text)


def fold(text: str) -> str:
    return " ".join(text.split())


def pipeline(text: str, form, fold_ws: bool, strip_inv: bool) -> str:
    if strip_inv:
        text = strip(text)
    if form:
        text = unicodedata.normalize(form, text)
    if fold_ws:
        text = fold(text)
    return text


def chars(*ranges):
    return st.one_of(*[st.integers(lo, hi).map(chr) for lo, hi in ranges])


# st.text() の既定は、サロゲート以外のすべてのカテゴリ（未割り当て Cn・私用 Co・書式 Cf を含む）
any_text = st.text()
long_text = st.text(max_size=3000)
combining = chars((0x0300, 0x036F), (0x0591, 0x05C7), (0x064B, 0x065F), (0x093C, 0x094D), (0x0F71, 0x0F84), (0x1DC0, 0x1DFF), (0x20D0, 0x20F0), (0x1D165, 0x1D1AD))
tricky = st.one_of(
    combining,
    chars((0x41, 0x7A), (0xC0, 0x24F), (0x1E00, 0x1FFF)),  # ラテン・ギリシャ（合成済み文字）
    chars((0x1100, 0x11FF), (0xAC00, 0xAC60), (0xD7A0, 0xD7FF), (0x3131, 0x318E)),  # ハングル
    chars((0x3040, 0x30FF), (0xFF00, 0xFFEF), (0x3200, 0x33FF)),  # かな・全角半角・組文字
    chars((0xF900, 0xFAFF), (0x2F800, 0x2FA1D)),  # CJK 互換漢字
    chars((0xFB00, 0xFB4F), (0xFB50, 0xFDFF), (0xFE70, 0xFEFF)),  # 合字・ヘブライ・アラビアの表示形
    chars((0x0900, 0x0DFF), (0x05D0, 0x05EA), (0x0621, 0x064A)),  # インド系・ヘブライ・アラビア
    st.sampled_from(un.WHITESPACE),
    chars(*un.INVISIBLE),
    st.sampled_from(["‍", "️", "︎", "͏", "ㅤ", "👨", "👩", "🏽", "🇯", "🇵", "\U000e0067"]),
)
tricky_text = st.one_of(st.text(tricky, max_size=40), st.text(tricky, max_size=500))
texts = st.one_of(any_text, long_text, tricky_text)
forms = st.sampled_from(FORMS)
options = st.tuples(st.one_of(st.none(), forms), st.booleans(), st.booleans())


@given(texts, forms)
def test_normalize_matches_unicodedata(text, form):
    assert un.normalize(text, form) == unicodedata.normalize(form, text)


@given(any_text)
def test_all_forms_on_arbitrary_text(text):
    for form in FORMS:
        assert un.normalize(text, form) == unicodedata.normalize(form, text), form


@given(st.text(combining, min_size=1, max_size=2000), st.sampled_from(["", "a", "א", "가", "ཀ"]), forms)
def test_long_combining_runs(marks, base, form):
    text = base + marks
    assert un.normalize(text, form) == unicodedata.normalize(form, text)


@given(texts, forms)
def test_is_normalized_matches_unicodedata(text, form):
    assert un.is_normalized(text, form) == unicodedata.is_normalized(form, text)
    assert un.is_normalized(unicodedata.normalize(form, text), form)


@given(texts)
def test_fold_whitespace_matches_split_join(text):
    assert un.fold_whitespace(text) == fold(text)


@given(texts)
def test_strip_invisible_matches_reference(text):
    assert un.strip_invisible(text) == strip(text)


@given(texts, options)
def test_pipeline_matches_reference(text, opts):
    form, fold_ws, strip_inv = opts
    assert un.normalize(text, form, fold_whitespace=fold_ws, strip_invisible=strip_inv) == pipeline(text, form, fold_ws, strip_inv)


@given(st.lists(texts, max_size=20), options)
def test_normalize_many_matches_normalize(items, opts):
    form, fold_ws, strip_inv = opts
    expected = [pipeline(t, form, fold_ws, strip_inv) for t in items]
    assert un.normalize_many(items, form, fold_whitespace=fold_ws, strip_invisible=strip_inv) == expected
    assert un.normalize_many(iter(items), form, fold_whitespace=fold_ws, strip_invisible=strip_inv) == expected


@given(texts, options)
def test_pipeline_output_is_clean_and_idempotent(text, opts):
    form, fold_ws, strip_inv = opts
    out = un.normalize(text, form, fold_whitespace=fold_ws, strip_invisible=strip_inv)
    assert type(out) is str
    if form:
        assert unicodedata.is_normalized(form, out)
    if fold_ws:
        assert "  " not in out and out == out.strip() and all(c == " " or not c.isspace() for c in out)
    if strip_inv:
        assert INVISIBLE_RE.search(out) is None
    assert un.normalize(out, form, fold_whitespace=fold_ws, strip_invisible=strip_inv) == out


@given(texts)
def test_form_relations(text):
    nfc, nfd, nfkc, nfkd = (un.normalize(text, f) for f in FORMS)
    assert un.normalize(nfd, "NFC") == nfc
    assert un.normalize(nfc, "NFD") == nfd
    assert un.normalize(nfc, "NFKC") == nfkc
    assert un.normalize(nfkd, "NFKC") == nfkc
    assert un.normalize(nfkc, "NFKD") == nfkd
    assert un.is_normalized(nfkc, "NFC") and un.is_normalized(nfkd, "NFD")


@given(texts, forms)
def test_unchanged_text_is_returned_as_is(text, form):
    out = un.normalize(text, form)
    if out == text:
        assert out is text


def test_every_code_point_alone():
    # すべての符号位置（サロゲートを除く）を1文字ずつ、4 つの形・空白のまとめ・見えない文字の除去で比べる
    every = [chr(c) for c in range(0x110000) if not 0xD800 <= c <= 0xDFFF]
    for form in FORMS:
        assert un.normalize_many(every, form) == [unicodedata.normalize(form, c) for c in every], form
    assert un.normalize_many(every, None, fold_whitespace=True) == [fold(c) for c in every]
    assert un.normalize_many(every, None, strip_invisible=True) == [strip(c) for c in every]


def test_whitespace_is_str_isspace():
    assert un.WHITESPACE == "".join(chr(c) for c in range(0x110000) if chr(c).isspace())


def test_invisible_is_default_ignorable_format_characters():
    # Cf のうち、目に見える書式文字（アラビア数字記号・ヒエログリフの書式など）を除いたもの
    visible_format = [(0x0600, 0x0605), (0x06DD, 0x06DD), (0x070F, 0x070F), (0x0890, 0x0891), (0x08E2, 0x08E2),
                      (0xFFF9, 0xFFFB), (0x110BD, 0x110BD), (0x110CD, 0x110CD), (0x13430, 0x1343F)]
    cf = {c for c in range(0x110000) if unicodedata.category(chr(c)) == "Cf"}
    invisible = {c for lo, hi in un.INVISIBLE for c in range(lo, hi + 1)}
    assert invisible == cf - {c for lo, hi in visible_format for c in range(lo, hi + 1)}
