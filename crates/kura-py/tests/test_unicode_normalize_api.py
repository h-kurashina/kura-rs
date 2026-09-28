"""kura_rs.unicode_normalize の使い方（引数・エラー・定数・GIL）の確認。"""

import threading
import unicodedata

import pytest

from kura_rs import unicode_normalize as un


def test_constants():
    assert un.FORMS == ("NFC", "NFD", "NFKC", "NFKD")
    assert un.UNICODE_VERSION == "16.0.0"
    assert len(un.WHITESPACE) == 29 and " " in un.WHITESPACE and "　" in un.WHITESPACE
    assert (0x200B, 0x200F) in un.INVISIBLE and (0xFEFF, 0xFEFF) in un.INVISIBLE
    assert sum(hi - lo + 1 for lo, hi in un.INVISIBLE) == 138


def test_examples():
    raw = "﻿ﾃﾞｰﾀ　　ＡＩ​  "
    assert un.normalize(raw, form="NFKC", fold_whitespace=True, strip_invisible=True) == "データ AI"
    assert un.normalize("é") == "é"  # 既定は NFC
    assert un.normalize("é", "NFD") == "é"
    assert un.normalize("ﬁ", "NFKC") == "fi"
    assert un.normalize("  a  b ", None, fold_whitespace=True) == "a b"
    assert un.normalize(raw, None) == raw
    assert un.fold_whitespace(" a\t\nb　") == "a b"
    assert un.strip_invisible("a­b‍c\U000e0041") == "abc"
    assert un.is_normalized("é") and not un.is_normalized("é")
    assert un.is_normalized("é", "NFD")
    assert un.normalize_many(["ＡＩ", "é", ""], "NFKC") == ["AI", "é", ""]


def test_form_names_are_case_insensitive():
    for form in un.FORMS:
        assert un.normalize("ｶﾞ", form.lower()) == unicodedata.normalize(form, "ｶﾞ")


@pytest.mark.parametrize("form", ["", "NF", "nfkc_cf", "NFC ", "X"])
def test_unknown_form(form):
    with pytest.raises(ValueError, match="unknown normalization form"):
        un.normalize("a", form)
    with pytest.raises(ValueError):
        un.is_normalized("a", form)
    with pytest.raises(ValueError):
        un.normalize_many(["a"], form)


def test_type_errors():
    with pytest.raises(TypeError):
        un.normalize(b"abc")
    with pytest.raises(TypeError):
        un.normalize(None)
    with pytest.raises(TypeError):
        un.fold_whitespace(1)
    with pytest.raises(TypeError, match="not a single str"):
        un.normalize_many("abc")
    with pytest.raises(TypeError, match="items must be str"):
        un.normalize_many(["a", b"b"])
    with pytest.raises(TypeError):
        un.normalize("a", "NFC", True)  # オプションはキーワードでしか渡せない


def test_lone_surrogates_are_rejected():
    for f in (un.normalize, un.fold_whitespace, un.strip_invisible, un.is_normalized):
        with pytest.raises(UnicodeEncodeError):
            f("a\ud800b")
    with pytest.raises(UnicodeEncodeError):
        un.normalize_many(["ok", "\udfff"])


def test_returns_the_same_object_when_nothing_changes():
    text = "already clean テキスト " * 1000
    assert un.normalize(text, "NFKC") is text
    assert un.fold_whitespace("a b") == "a b"
    assert un.normalize_many([text])[0] is text


def test_str_subclass_gives_plain_str():
    class Sub(str):
        pass

    out = un.normalize(Sub("abc"))
    assert type(out) is str and out == "abc"


def test_large_input():
    text = ("ＡＩ ﾃﾞｰﾀ　é " * 200_000) + "end"
    assert un.normalize(text, "NFKC") == unicodedata.normalize("NFKC", text)
    assert un.normalize(text, "NFKC", fold_whitespace=True) == " ".join(unicodedata.normalize("NFKC", text).split())


def test_threads_run_while_normalizing():
    # GIL を手放すので、ほかのスレッドから同時に呼んでも結果が混ざらない
    texts = [("ｶﾞ ＡＩ é %d " % i) * 20_000 for i in range(8)]
    results = [None] * len(texts)

    def work(i):
        results[i] = un.normalize(texts[i], "NFKC", fold_whitespace=True)

    threads = [threading.Thread(target=work, args=(i,)) for i in range(len(texts))]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    assert results == [" ".join(unicodedata.normalize("NFKC", t).split()) for t in texts]
