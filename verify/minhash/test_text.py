"""実文書まわり（シングル化・コーパス）の単体テスト。ネットワークは使わない。

    python -m unittest discover -s verify/minhash -v
"""

import hashlib
import json
import random
import re
import subprocess
import unittest

import mwparserfromhell.parser

import corpus
from shingle import GOLDEN, MODES, PUNCTUATION, SEPARATORS, WHITESPACE, digest, golden_text, shingles, words


class ShingleTest(unittest.TestCase):
    def test_golden_file_is_up_to_date(self):
        # Rust 側のテストも同じファイルと比べるので、ここが通れば両者は同じ規則
        self.assertEqual(GOLDEN.read_text(), golden_text())

    def test_golden_covers_both_modes(self):
        cases = json.loads(GOLDEN.read_text())["cases"]
        self.assertEqual({c["mode"] for c in cases}, set(MODES))
        self.assertGreaterEqual(len(cases), 90)

    def test_separator_lists_have_no_duplicates(self):
        self.assertEqual(len(set(WHITESPACE)), len(WHITESPACE))
        self.assertEqual(len(set(PUNCTUATION)), len(PUNCTUATION))
        self.assertFalse(set(WHITESPACE) & set(PUNCTUATION))
        self.assertEqual(len(SEPARATORS), len(WHITESPACE) + len(PUNCTUATION))

    def test_all_ascii_punctuation_and_whitespace_are_separators(self):
        for c in map(chr, range(128)):
            expected = (not c.isalnum() and c.isprintable() and c != " ") or c in " \t\n\x0b\x0c\r"
            self.assertEqual(c in SEPARATORS, expected, repr(c))

    def test_whitespace_is_a_subset_of_python_isspace(self):
        # White_Space の文字は Python でも空白（逆は成り立たない: \x1c-\x1f は Python だけが空白とみなす）
        self.assertTrue(all(c.isspace() for c in WHITESPACE))
        self.assertTrue("\x1c".isspace())
        self.assertNotIn("\x1c", SEPARATORS)

    def test_punctuation_is_not_alphanumeric(self):
        self.assertFalse(any(c.isalnum() for c in PUNCTUATION))

    def test_word_shingles(self):
        self.assertEqual(shingles("The quick brown fox", "word"), ["the quick brown", "quick brown fox"])
        self.assertEqual(shingles("one, two; three", "word"), ["one two three"])
        self.assertEqual(shingles("one two", "word"), ["one two"])
        self.assertEqual(shingles("one", "word"), ["one"])
        self.assertEqual(shingles(" ,. ", "word"), [])

    def test_char_shingles(self):
        self.assertEqual(shingles("日本語の文", "char"), ["日本語", "本語の", "語の文"])
        self.assertEqual(shingles("東京、 大阪。", "char"), ["東京大", "京大阪"])
        self.assertEqual(shingles("日本", "char"), ["日本"])
        self.assertEqual(shingles("、。", "char"), [])

    def test_duplicates_removed_in_first_seen_order(self):
        self.assertEqual(shingles("a b c a b c a b c", "word"), ["a b c", "b c a", "c a b"])
        self.assertEqual(shingles("ああああああ", "char"), ["あああ"])

    def test_lowercase_rules(self):
        self.assertEqual(words("ΟΔΟΣ ΣΑΣ"), ["οδος", "σας"])
        self.assertEqual(words("İ"), ["i̇"])
        self.assertEqual(words("STRASSE Straße"), ["strasse", "straße"])

    def test_unknown_mode_is_rejected(self):
        with self.assertRaises(ValueError):
            shingles("x", "sentence")

    def test_digest(self):
        self.assertEqual(digest([]), hashlib.sha1(b"").hexdigest())
        self.assertEqual(digest(["abc"]), "03cfd743661f07975fa2f1220c5194cbaff48451")
        self.assertNotEqual(digest(["ab", "c"]), digest(["a", "bc"]))
        self.assertNotEqual(digest(["a", "b"]), digest(["b", "a"]))

    def test_shingles_never_contain_separators(self):
        rng = random.Random(1)
        alphabet = "ab日本 ,。\t\n—…" + WHITESPACE
        for _ in range(500):
            text = "".join(rng.choice(alphabet) for _ in range(rng.randrange(40)))
            for mode in MODES:
                for g in shingles(text, mode):
                    self.assertTrue(g)
                    if mode == "char":
                        self.assertFalse(set(g) & SEPARATORS)
                    else:
                        self.assertFalse(set(g) & (SEPARATORS - {" "}))
                        self.assertLessEqual(g.count(" "), 2)


WIKITEXT = """{{Infobox thing|name=X|size=3}}
'''MinHash''' is a [[Hash function|technique]] for estimating<ref>A reference.</ref> [[Jaccard index]] similarity.<!-- comment -->
[[File:Example.png|thumb|A caption]]
[[ファイル:例.png|thumb|説明]]

== History ==
It was invented by [https://example.org Andrei Broder] in 1997.<ref name="b" />

[[Category:Hashing]]
[[カテゴリ:ハッシュ]]
"""


class CorpusTest(unittest.TestCase):
    def test_plain_text_drops_markup(self):
        text = corpus.plain_text(WIKITEXT)
        self.assertIn("MinHash is a technique for estimating Jaccard index similarity.", text)
        self.assertIn("It was invented by Andrei Broder in 1997.", text)
        for noise in ["Infobox", "reference", "comment", "caption", "説明", "Hashing", "ハッシュ", "[[", "{{", "'''", "<ref"]:
            self.assertNotIn(noise, text)

    def test_plain_text_is_the_same_without_the_c_tokenizer(self):
        # CI などで mwparserfromhell の C 拡張が使えなくても、平文が変わらないこと
        with_c = corpus.plain_text(WIKITEXT)
        saved = mwparserfromhell.parser.use_c
        try:
            mwparserfromhell.parser.use_c = False
            self.assertEqual(corpus.plain_text(WIKITEXT), with_c)
        finally:
            mwparserfromhell.parser.use_c = saved

    def test_plain_text_has_no_runs_of_blank_lines(self):
        text = corpus.plain_text(WIKITEXT + "\n\n\n\n\nEnd.   \n")
        self.assertNotRegex(text, r"\n{3,}")
        self.assertNotRegex(text, r"[ \t]\n")
        self.assertEqual(text, text.strip())

    def test_paragraphs(self):
        text = "Short.\n\n" + "a" * 40 + "\n \n" + "b" * 39 + "\n\n\n" + "c" * 50 + "\nsame paragraph"
        self.assertEqual(corpus.paragraphs(text), ["a" * 40, "c" * 50 + "\nsame paragraph"])

    def test_near_duplicate_is_deterministic(self):
        text = "the quick brown fox jumps over the lazy dog " * 5
        a = corpus.near_duplicate(text, "word", random.Random(7))
        b = corpus.near_duplicate(text, "word", random.Random(7))
        self.assertEqual(a, b)

    def test_near_duplicate_changes_little(self):
        rng = random.Random(3)
        for mode, text in [("word", "the quick brown fox jumps over the lazy dog " * 10), ("char", "日本語の文章を少しだけ変える。" * 10)]:
            for _ in range(200):
                edited, ops = corpus.near_duplicate(text, mode, rng)
                self.assertTrue(ops)
                self.assertLessEqual(len(ops.split("+")), 3)
                a, b = set(shingles(text, mode)), set(shingles(edited, mode))
                if ops == "case":
                    self.assertEqual(a, b)  # 大文字にしても小文字化で元に戻る
                self.assertGreater(len(a & b) / len(a | b), 0.3)

    def test_bench_documents_pick_increasing_sizes_near_targets(self):
        rng = random.Random(5)
        vocab = [f"w{i}" for i in range(5000)]

        def article(n_words, revid):
            paras = ["\n\n".join(" ".join(rng.choice(vocab) for _ in range(k)) for k in [12, 32, 102, 302])]
            body = " ".join(rng.choice(vocab) for _ in range(n_words))
            return {"lang": "en", "mode": "word", "revid": revid, "text": "\n\n".join(paras + [body])}

        arts = [article(n, i) for i, n in enumerate([900, 2500, 9000, 12000, 30000])]
        docs = corpus.bench_documents(arts)
        sizes = [d["size"] for d in docs]
        self.assertEqual(sizes, sorted(set(sizes)))
        self.assertGreaterEqual(len(docs), 6)
        for d in docs:
            self.assertEqual(d["size"], len(shingles(d["text"], "word")))
            self.assertLess(min(abs(corpus._log_ratio(d["size"], t)) for t in corpus.BENCH_TARGETS), 0.35)


class ManifestTest(unittest.TestCase):
    m = corpus.manifest()

    def test_languages(self):
        self.assertTrue({"en", "ja"} <= set(self.m["languages"]))
        self.assertEqual(self.m["languages"]["en"]["mode"], "word")
        self.assertEqual(self.m["languages"]["ja"]["mode"], "char")
        for lang in self.m["languages"].values():
            self.assertIn(lang["mode"], MODES)

    def test_articles_are_pinned(self):
        seen = set()
        for a in self.m["articles"]:
            self.assertIn(a["lang"], self.m["languages"])
            self.assertIsInstance(a["revid"], int)
            self.assertRegex(a["sha1"], r"^[0-9a-f]{40}$")
            self.assertRegex(a["text_sha256"], r"^[0-9a-f]{64}$")
            self.assertGreater(a["chars"], 0)
            self.assertGreater(a["shingles"], 0)
            self.assertTrue(a["title"])
            self.assertNotIn((a["lang"], a["revid"]), seen)
            seen.add((a["lang"], a["revid"]))
        self.assertGreaterEqual(len(self.m["articles"]), 30)

    def test_every_language_has_articles(self):
        self.assertEqual({a["lang"] for a in self.m["articles"]}, set(self.m["languages"]))

    def test_license_is_recorded_and_text_is_not_committed(self):
        self.assertIn("CC BY-SA", self.m["license"])
        for a in self.m["articles"]:
            self.assertFalse({"text", "wikitext", "content"} & set(a))
        ignored = subprocess.run(
            ["git", "check-ignore", "-q", str(corpus.CACHE / "en" / "1.wikitext")], cwd=corpus.ROOT
        )
        self.assertEqual(ignored.returncode, 0, "verify/data/ must be gitignored")

    def test_user_agent_identifies_the_project(self):
        self.assertRegex(corpus.USER_AGENT, r"^kura-rs-verify/\S+ \(https://github\.com/")
        self.assertGreaterEqual(corpus.MIN_INTERVAL_S, 1.0)

    @unittest.skipUnless(all(corpus._cache_path(a).exists() for a in corpus.manifest()["articles"]), "corpus is not cached")
    def test_cached_corpus_matches_manifest(self):
        for a in corpus.articles(download=False):
            raw = corpus._cache_path(a).read_text()
            self.assertEqual(hashlib.sha1(raw.encode()).hexdigest(), a["sha1"])
            self.assertEqual(len(a["text"]), a["chars"])
            self.assertEqual(len(shingles(a["text"], a["mode"])), a["shingles"])
            self.assertFalse(re.search(r"\{\{|\}\}|<ref[ >]", a["text"]), a["title"])  # 元の wikitext の書き損じ（<ref? など）は残りうる


if __name__ == "__main__":
    unittest.main()
