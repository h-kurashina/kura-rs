"""実文書を MinHash に入れる前のシングル化（Python 側）。

Rust 側（crates/kura-verify/src/shingle.rs）と一字一句同じ規則で作る。規則がずれると
「同じ入力で比べた」ことにならないので、差分テストでは両者のシングル列のハッシュも突き合わせる。

規則:
1. 文字列全体を小文字にする（Python の str.lower() と Rust の str::to_lowercase() は同じ Unicode の規則）
2. 区切り文字（SEPARATORS: Unicode の White_Space と、よく使う句読点・記号）で切る
3. word モード（英語など分かち書きする言語）: 切り出した単語の連続3つを " " でつなぐ（単語 3-shingle）
   char モード（日本語・中国語）: 区切り文字を取り除いた文字列の連続3文字（文字 3-gram）
   3つに満たないときは、あるものを全部つないだ1つだけ。何もなければ空
4. 最初に出た順で重複を除く（MinHash は集合なので結果は変わらないが、件数を「異なるシングルの数」にそろえる）
"""

import hashlib
import json
import sys
from pathlib import Path

# Unicode の White_Space 属性を持つ文字（Rust の char::is_whitespace と同じ集合）。
# Python の str.isspace() は \x1c-\x1f も含むなど少し違うので、明示的に並べる。
WHITESPACE = (
    "\u0009\u000a\u000b\u000c\u000d \u0085  "
    "           "
    "    　"
)

# 句読点・記号。Unicode の分類は版で変わりうるので、分類には頼らず明示的に並べる
PUNCTUATION = (
    "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"  # ASCII の記号すべて
    "¡§«¶·»¿"  # ¡ § « ¶ · » ¿
    "‐‑‒–—―"  # ハイフン・ダッシュ類
    "‘’‚‛“”„‟"  # 引用符
    "†‡•…‰′″‹›※"  # † ‡ • … ‰ ′ ″ ‹ › ※
    "、。〃〈〉《》「」『』"  # 、。〃〈〉《》「」『』
    "【】〔〕〖〗〜〝〞〟・"  # 【】〔〕〖〗〜〝〞〟・
    "！＂＃％＆＇（）＊，－．／"  # 全角 ！＂＃％＆＇（）＊，－．／
    "：；？＠［＼］＿｛｝～｡､"  # 全角 ：；？＠［＼］＿｛｝～ 半角 ｡ ､
)

SEPARATORS = frozenset(WHITESPACE + PUNCTUATION)

MODES = ("word", "char")


def words(text: str) -> list[str]:
    out, current = [], []
    for c in text.lower():
        if c in SEPARATORS:
            if current:
                out.append("".join(current))
                current = []
        else:
            current.append(c)
    if current:
        out.append("".join(current))
    return out


def _grams(units: list[str], joiner: str, n: int = 3) -> list[str]:
    if not units:
        return []
    if len(units) < n:
        return [joiner.join(units)]
    return [joiner.join(units[i : i + n]) for i in range(len(units) - n + 1)]


def shingles(text: str, mode: str) -> list[str]:
    if mode == "word":
        grams = _grams(words(text), " ")
    elif mode == "char":
        grams = _grams([c for c in text.lower() if c not in SEPARATORS], "")
    else:
        raise ValueError(f"unknown mode: {mode}")
    return list(dict.fromkeys(grams))  # 順番を保って重複を除く


def digest(grams: list[str]) -> str:
    """シングル列の指紋。各シングルの UTF-8 の後ろに \\n を付けてつないだものの SHA-1（シングルは区切り文字を含まないので曖昧さがない）。"""
    h = hashlib.sha1()
    for g in grams:
        h.update(g.encode())
        h.update(b"\n")
    return h.hexdigest()


# ゴールデン（期待値）ファイル。Rust 側のテスト（crates/kura-verify/src/shingle.rs）も同じファイルを読む
GOLDEN = Path(__file__).resolve().parent / "shingle_golden.json"

# 境目になりやすい入力。大文字小文字の特殊な規則、全角・半角、結合文字、区切り文字ではない空白類など
GOLDEN_INPUTS = [
    "", " ", "   \t\n", "a", "a b", "a b c", "a b c d", "a a a a a",
    "Hello, World! Hello world.", "The quick brown fox jumps over the lazy dog",
    "don't stop—believing… “quoted” «guillemets» ‹single›",
    "(parenthesized) [bracketed] {braced} <angled> a/b\\c|d",
    "snake_case kebab-case dot.case camelCase PascalCase",
    "1,000.5 3.14 2026-09-26 50% $100 #tag @user",
    "ΣΑΣ ΟΔΟΣ Σ ΣΑ. ὈΔΥΣΣΕΎΣ", "İstanbul IĞDIR ıi", "Straße STRASSE ẞ", "ǅungla ǈ ǋ Ǳ",
    "K Å Ω Kelvin Ångström Ohm", "ﬁnance ﬂow ﬀ", "ᾼ ῌ ῼ Ὰͅ", "Ａｂｃ ＡＢＣ ｱｲｳｴｵ",
    "é é café café", "a​b c‌d e⁠f", "x\x1cy\x1dz\x1e w\x1f",
    "tab\tnew\nline\r\ncr\rff\x0cvt\x0bnel\x85nbsp\xa0end",
    "　全角スペース　と em space thin",
    "日本語のテキスト。", "東京都は、日本の首都である。", "「かぎかっこ」『二重』（丸括弧）【隅付き】",
    "Rust は速い、Python は書きやすい。", "ｶﾀｶﾅ ﾃｷｽﾄ｡ ﾃﾞｽ､",
    "我爱北京天安门。长城！", "대한민국은 동아시아의 한반도 군사 분계선 남부에 위치한 나라이다.",
    "🦀🦀🦀🦀 🦀 crab 🦀", "👨‍👩‍👧 family", "العربية نص", "עִבְרִית טקסט",
    "Москва ПРИВЕТ мир", "ÀÉÎÕÜ àéîõü", "a\u0000b", "﻿bom start",
    "~!@#$%^&*()_+`-={}|[]\\:\";'<>?,./", "…", "a…b…c…d", "・・・中黒・区切り・",
]


def golden() -> dict:
    cases = []
    for text in GOLDEN_INPUTS:
        for mode in MODES:
            grams = shingles(text, mode)
            cases.append({"mode": mode, "text": text, "shingles": grams, "digest": digest(grams)})
    return {"separators": sorted(ord(c) for c in SEPARATORS), "cases": cases}


def golden_text() -> str:
    return json.dumps(golden(), ensure_ascii=False, indent=1) + "\n"


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "golden":  # 規則を変えたときだけ使う
        GOLDEN.write_text(golden_text())
    elif command == "check":
        if GOLDEN.read_text() != golden_text():
            sys.exit(
                "shingle_golden.json does not match shingle.py; "
                "if the rule changed on purpose, run: python verify/minhash/shingle.py golden"
            )
        print(f"shingle: {len(golden()['cases'])} golden cases match")
    else:
        sys.exit("usage: shingle.py golden | check")
