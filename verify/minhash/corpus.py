"""minhash の実文書コーパス（Wikipedia）。記事の一覧は corpus.json に固定し、本文はコミットしない。

    python verify/minhash/corpus.py fetch            # キャッシュにない記事を取得する（run.py からも自動で呼ばれる）
    python verify/minhash/corpus.py stats            # 記事ごとの文字数・シングル数を表示する
    python verify/minhash/corpus.py pin              # corpus.json で revid が空の記事に、いまの最新版の revid を書き込み、
                                                     # 全記事の平文の SHA-256 などを書き直す（記事を足すとき・平文化の規則を変えたとき用）

- 記事は版 ID（revid / oldid）で固定する。版の wikitext は後から変わらないので、誰がいつ取っても同じ本文になる。
- 取得は MediaWiki API の prop=revisions で、言語ごとに1回のリクエストにまとめる（最大 50 版）。
  リクエストの間は 1 秒以上あけ、maxlag を付け、連絡先の分かる User-Agent を送る（Wikimedia の方針）。
- 本文（CC BY-SA）はリポジトリに入れず、verify/data/minhash/（.gitignore 済み）にだけ置く。
- wikitext から平文への変換は mwparserfromhell（版は verify/requirements.txt で固定）で行い、
  結果の SHA-256 を corpus.json と突き合わせる。変換が変わったら黙って別の入力で比べずに止まる。
"""

import gzip
import hashlib
import json
import random
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

import mwparserfromhell

from shingle import shingles

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
MANIFEST = HERE / "corpus.json"
CACHE = ROOT / "verify" / "data" / "minhash" / "wikipedia"

USER_AGENT = "kura-rs-verify/0.1 (https://github.com/h-kurashina/kura-rs; benchmark corpus for the minhash part)"
MIN_INTERVAL_S = 1.0
_last_request = 0.0

# 画像・カテゴリなど、本文ではないリンクの名前空間（小文字で比べる）
NON_TEXT_NAMESPACES = {
    "file", "image", "category", "media",
    "ファイル", "画像", "カテゴリ", "カテゴリー",
    "datei", "bild", "kategorie",
    "fichier", "catégorie",
    "파일", "그림", "분류",
    "文件", "檔案", "档案", "图像", "圖像", "分类", "分類",
}

# 中身ごと捨てるタグ（脚注、数式、ソースコード、画像の一覧など、地の文ではないもの）
DROPPED_TAGS = {
    "ref", "references", "gallery", "math", "chem", "ce", "score", "timeline", "imagemap", "graph",
    "syntaxhighlight", "source", "pre", "code", "templatedata", "mapframe", "maplink", "categorytree",
}


def manifest() -> dict:
    return json.loads(MANIFEST.read_text())


def _api(lang: str, params: dict) -> dict:
    """API を1回呼ぶ。前回から MIN_INTERVAL_S 秒あけ、maxlag や 429 のときは待ってやり直す。"""
    global _last_request
    query = urllib.parse.urlencode({**params, "format": "json", "formatversion": "2", "maxlag": "5"})
    url = f"https://{lang}.wikipedia.org/w/api.php?{query}"
    for attempt in range(5):
        wait = _last_request + MIN_INTERVAL_S - time.monotonic()
        if wait > 0:
            time.sleep(wait)
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT, "Accept-Encoding": "gzip"})
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                body = response.read()
                if response.headers.get("Content-Encoding") == "gzip":
                    body = gzip.decompress(body)
                retry_after = response.headers.get("Retry-After")
        except urllib.error.HTTPError as e:
            if e.code not in (429, 503):
                raise
            body, retry_after = None, e.headers.get("Retry-After")
        finally:
            _last_request = time.monotonic()
        if body is not None:
            data = json.loads(body)
            if data.get("error", {}).get("code") != "maxlag":
                if "error" in data:
                    raise RuntimeError(f"{lang}.wikipedia.org: {data['error']}")
                return data
        time.sleep(max(float(retry_after or 5), 5) * (attempt + 1))
    raise RuntimeError(f"{lang}.wikipedia.org: gave up after retries")


def _cache_path(article: dict) -> Path:
    return CACHE / article["lang"] / f"{article['revid']}.wikitext"


def fetch(quiet: bool = False) -> None:
    """キャッシュにない版だけを取る。言語ごとに1リクエスト（最大 50 版ずつ）。"""
    missing: dict[str, list[dict]] = {}
    for article in manifest()["articles"]:
        if not _cache_path(article).exists():
            missing.setdefault(article["lang"], []).append(article)
    for lang, articles in missing.items():
        for i in range(0, len(articles), 50):
            batch = {a["revid"]: a for a in articles[i : i + 50]}
            if not quiet:
                print(f"  fetching {len(batch)} revisions from {lang}.wikipedia.org", file=sys.stderr)
            data = _api(lang, {
                "action": "query", "prop": "revisions", "rvprop": "ids|sha1|content", "rvslots": "main",
                "revids": "|".join(map(str, batch)),
            })
            if data["query"].get("badrevids"):
                raise RuntimeError(f"unknown revisions: {data['query']['badrevids']}")
            for page in data["query"]["pages"]:
                for rev in page.get("revisions", []):
                    article = batch.pop(rev["revid"])
                    content = rev["slots"]["main"]["content"]
                    got = hashlib.sha1(content.encode()).hexdigest()
                    if got != rev["sha1"] or got != article["sha1"]:
                        raise RuntimeError(f"{lang}:{article['title']} ({rev['revid']}): sha1 {got} does not match")
                    path = _cache_path(article)
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(content)
            if batch:
                raise RuntimeError(f"not returned: {list(batch)}")


def plain_text(wikitext: str) -> str:
    """wikitext を平文にする。テンプレート・脚注・画像・カテゴリを落とし、リンクは表示文字列だけ残す。"""
    code = mwparserfromhell.parse(wikitext)
    for tag in code.filter_tags(recursive=True):
        if str(tag.tag).strip().lower() in DROPPED_TAGS:
            try:
                code.remove(tag)
            except ValueError:
                pass  # すでに親ごと消えている
    for link in code.filter_wikilinks(recursive=True):
        prefix = str(link.title).strip().lstrip(":").split(":", 1)
        if len(prefix) == 2 and prefix[0].strip().lower() in NON_TEXT_NAMESPACES:
            try:
                code.remove(link)
            except ValueError:
                pass  # すでに親ごと消えている
    text = code.strip_code(normalize=True, collapse=True, keep_template_params=False)
    text = re.sub(r"[ \t]+\n", "\n", text)
    return re.sub(r"\n{3,}", "\n\n", text).strip()


def articles(download: bool = True) -> list[dict]:
    """記事ごとに {lang, mode, title, revid, text} を返す。キャッシュがなければ取得する（download=False なら失敗）。"""
    m = manifest()
    missing = [a for a in m["articles"] if not _cache_path(a).exists()]
    if missing:
        if not download:
            raise FileNotFoundError(f"{len(missing)} articles are not cached; run: python verify/minhash/corpus.py fetch")
        fetch(quiet=False)
    out = []
    for a in m["articles"]:
        text = plain_text(_cache_path(a).read_text())
        digest = hashlib.sha256(text.encode()).hexdigest()
        if a.get("text_sha256") and digest != a["text_sha256"]:
            raise RuntimeError(
                f"{a['lang']}:{a['title']} ({a['revid']}): plain text differs from corpus.json "
                f"(mwparserfromhell の版が違う可能性)"
            )
        out.append({**a, "mode": m["languages"][a["lang"]]["mode"], "text": text, "text_sha256": digest})
    return out


def paragraphs(text: str) -> list[str]:
    """空行で区切った段落。見出しだけの行などを除くため、40 文字未満のものは捨てる。"""
    return [p.strip() for p in re.split(r"\n\s*\n", text) if len(p.strip()) >= 40]


def all_paragraphs(arts: list[dict]) -> list[dict]:
    out = []
    for a in arts:
        for i, p in enumerate(paragraphs(a["text"])):
            out.append({"id": f"{a['lang']}:{a['revid']}:p{i}", "lang": a["lang"], "mode": a["mode"], "text": p})
    return out


# ベンチマークの目標サイズ（シングル数）。実在の段落・記事・記事の連結から、いちばん近いものを選ぶ
BENCH_TARGETS = [10, 30, 100, 300, 1000, 3000, 10000, 30000, 100000, 300000]
BENCH_LANG = "en"


def bench_documents(arts: list[dict]) -> list[dict]:
    """英語の実文書から、目標サイズに近いものを1つずつ選ぶ（小さい順・サイズの重複なし）。

    候補: 段落、記事全体、記事を長い順に k 本つないだもの（大きいサイズ用）、
    単語で区切る言語の全記事をつないだもの（最大サイズ用。ここだけ英語以外も混ざる）。
    """
    en = [a for a in arts if a["lang"] == BENCH_LANG]
    candidates = []
    for p in all_paragraphs(en):
        candidates.append({"id": p["id"], "kind": "paragraph", "text": p["text"]})
    for a in en:
        candidates.append({"id": f"{a['lang']}:{a['revid']}", "kind": "article", "text": a["text"]})
    # 長い記事から順につなぐと、少ない本数で大きなサイズに届く
    by_length = sorted(en, key=lambda a: -len(a["text"]))
    for k in range(2, len(by_length) + 1):
        candidates.append({
            "id": "+".join(f"{a['lang']}:{a['revid']}" for a in by_length[:k]),
            "kind": f"{k} articles concatenated",
            "text": "\n\n".join(a["text"] for a in by_length[:k]),
        })
    # いちばん大きいサイズ用: 単語で区切る言語（英・独・仏・韓）の記事をすべてつないだもの
    word_articles = [a for a in arts if a["mode"] == "word"]
    candidates.append({
        "id": "all word-mode articles",
        "kind": f"all {len(word_articles)} {'/'.join(dict.fromkeys(a['lang'] for a in word_articles))} articles concatenated",
        "text": "\n\n".join(a["text"] for a in word_articles),
    })
    for c in candidates:
        c["size"] = len(shingles(c["text"], "word"))
    chosen, used = [], set()
    for target in BENCH_TARGETS:
        best = min(candidates, key=lambda c: (abs(_log_ratio(c["size"], target)), c["id"]))
        if best["size"] in used or abs(_log_ratio(best["size"], target)) > 0.35:
            continue  # 近いものがない目標は飛ばす
        used.add(best["size"])
        chosen.append({"id": best["id"], "kind": best["kind"], "lang": BENCH_LANG, "mode": "word", "size": best["size"], "text": best["text"]})
    return sorted(chosen, key=lambda d: d["size"])


def _log_ratio(a: int, b: int) -> float:
    import math

    return math.log10(max(a, 1) / b)


def near_duplicate(text: str, mode: str, rng: random.Random) -> tuple[str, str]:
    """小さな編集を 1〜3 回加えた近似重複を作る。返り値は (編集後の本文, 何をしたか)。"""
    units = text.split(" ") if mode == "word" else list(text)
    joiner = " " if mode == "word" else ""
    ops = []
    for _ in range(rng.randint(1, 3)):
        if len(units) < 2:
            break
        op = rng.choice(["delete", "insert", "replace", "swap", "duplicate", "case", "truncate"])
        i = rng.randrange(len(units))
        if op == "delete":
            del units[i]
        elif op == "insert":
            units.insert(i, rng.choice(["the", "kura", "not", "日本", "の", "und", "과", "de", "🦀"]))
        elif op == "replace":
            units[i] = rng.choice(["minhash", "Rust", "テスト", "中文", "é", "ß", "İ", "ΣΑΣ"])
        elif op == "swap" and i + 1 < len(units):
            units[i], units[i + 1] = units[i + 1], units[i]
        elif op == "duplicate":
            j = min(len(units), i + rng.randint(1, 8))
            units[j:j] = units[i:j]
        elif op == "case":
            units[i] = units[i].upper()  # 小文字化で元に戻るので、シングルは変わらないはず
        elif op == "truncate":
            units = units[: max(1, len(units) - rng.randint(1, 5))]
        ops.append(op)
    return joiner.join(units), "+".join(ops)


def pin() -> None:
    """revid が空の記事に、いまの最新版の revid・sha1 を書き込む。"""
    m = manifest()
    for article in m["articles"]:
        if article.get("revid"):
            continue
        data = _api(article["lang"], {
            "action": "query", "prop": "revisions", "rvprop": "ids|sha1", "titles": article["title"], "redirects": "1",
        })
        page = data["query"]["pages"][0]
        if page.get("missing"):
            raise RuntimeError(f"{article['lang']}:{article['title']} does not exist")
        article["title"] = page["title"]
        article["revid"] = page["revisions"][0]["revid"]
        article["sha1"] = page["revisions"][0]["sha1"]
        print(f"  {article['lang']}:{article['title']} -> {article['revid']}", file=sys.stderr)
    # 平文の SHA-256 と大きさは、平文化の規則を変えたときも書き直す
    for article in m["articles"]:
        article.pop("text_sha256", None)
    MANIFEST.write_text(json.dumps(m, indent=2, ensure_ascii=False) + "\n")
    fetch()
    # 平文の SHA-256 と大きさを書き込む（取得した本文が同じかを、以後はこれで確かめる）
    for article, full in zip(m["articles"], articles(download=False), strict=True):
        article["text_sha256"] = full["text_sha256"]
        article["chars"] = len(full["text"])
        article["shingles"] = len(shingles(full["text"], full["mode"]))
    MANIFEST.write_text(json.dumps(m, indent=2, ensure_ascii=False) + "\n")


def stats() -> None:
    arts = articles()
    paras = all_paragraphs(arts)
    for a in arts:
        print(f"{a['lang']}  {a['revid']:>10}  {len(a['text']):>8} chars  {len(shingles(a['text'], a['mode'])):>7} shingles  {a['title']}")
    total = sum(len(shingles(a["text"], a["mode"])) for a in arts)
    print(f"{len(arts)} articles, {len(paras)} paragraphs, {total} shingles in total")
    for d in bench_documents(arts):
        print(f"bench {d['size']:>7}  {d['kind']}  {d['id']}")


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "fetch":
        fetch()
    elif command == "pin":
        pin()
    elif command == "stats":
        stats()
    else:
        sys.exit("usage: corpus.py fetch | stats | pin")
