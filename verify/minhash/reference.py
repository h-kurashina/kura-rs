"""minhash の参照実装（datasketch）側。verify/run.py から呼ばれる。

cases <n> <seed>      : 差分テストのケースと datasketch の答えを JSON Lines で標準出力に書く
text-cases <seed>     : 実文書（Wikipedia）の差分テストのケースを JSON Lines で書く（シングル列の指紋も含む）
bench-docs <path>     : ベンチマークに使う実文書を JSON で書き出す（Rust 側と同じファイルを読む）
many-docs <path>      : 「たくさんの段落をまとめて署名する」場面の実文書を JSON で書き出す
bench <num_perm> <size>... : 合成トークン（token-0, token-1, …）の数ごとの処理時間（ミリ秒の中央値）を JSON で書く
bench-text <num_perm> <path> : 実文書ごとの処理時間を JSON で書く（シングル化は計測に含めない）
bench-many <num_perm> <path> : 段落をまとめて署名する時間（MinHash.bulk）を JSON で書く
"""

import json
import random
import statistics
import sys
import time

from datasketch import MinHash

import corpus
from shingle import digest, shingles

# 境目になりやすい値を多めに混ぜる
NUM_PERMS = [1, 2, 3, 7, 8, 16, 31, 32, 33, 64, 127, 128, 129, 255, 256, 257, 512, 1024]
SEEDS = [0, 1, 2, 3, 42, 623, 624, 625, 2**16, 2**31 - 1, 2**31, 2**32 - 2, 2**32 - 1]
SIZES = [0, 1, 2, 3, 5, 10, 20, 50, 100, 500, 2000]


def random_token(rng: random.Random) -> bytes:
    kind = rng.random()
    if kind < 0.1:
        return b""  # 空のトークン
    if kind < 0.3:
        return bytes(rng.randrange(256) for _ in range(rng.randrange(1, 64)))  # 任意のバイト列
    if kind < 0.35:
        return "".join(rng.choice("日本語テキスト중복검출тест🦀الْعَرَبِيَّةעִבְרִית\u200b\u0301") for _ in range(rng.randrange(1, 12))).encode()
    if kind < 0.4:
        return bytes([rng.choice([0, 0xFF, 0x80, 0x7F])]) * rng.randrange(1, 300)  # 同じバイトの連続・長い入力
    if kind < 0.45:
        return rng.choice([b"\x00", b"\n", b"\r\n", b" ", b"\t", b"\xef\xbb\xbf", b"\xff\xfe"])  # 制御文字・BOM
    return f"token-{rng.randrange(1000)}".encode()


def cases(n: int, seed: int) -> None:
    rng = random.Random(seed)
    for i in range(n):
        num_perm = rng.choice(NUM_PERMS) if i % 2 == 0 else rng.randrange(1, 300)
        perm_seed = rng.choice(SEEDS) if i % 3 == 0 else rng.randrange(2**32)
        size = rng.choice(SIZES)
        a = [random_token(rng) for _ in range(size)]
        # b は a と一部が重なる集合（類似度がばらけるように）
        b = [t for t in a if rng.random() < rng.random()] + [random_token(rng) for _ in range(rng.randrange(0, size + 1))]
        ma, mb = MinHash(num_perm=num_perm, seed=perm_seed), MinHash(num_perm=num_perm, seed=perm_seed)
        ma.update_batch(a)
        mb.update_batch(b)
        print(json.dumps({
            "num_perm": num_perm,
            "seed": perm_seed,
            "a": [t.hex() for t in a],
            "b": [t.hex() for t in b],
            "expected_a": [int(v) for v in ma.hashvalues],
            "expected_b": [int(v) for v in mb.hashvalues],
            "expected_jaccard": ma.jaccard(mb),
            "expected_perm_a": [int(v) for v in ma.permutations[0]],
            "expected_perm_b": [int(v) for v in ma.permutations[1]],
        }))


def median_ms(f, min_runs=5, min_total_ms=300.0, max_runs=1000) -> float:
    samples, total = [], 0.0
    while len(samples) < min_runs or (total < min_total_ms and len(samples) < max_runs):
        start = time.perf_counter()
        f()
        ms = (time.perf_counter() - start) * 1000
        samples.append(ms)
        total += ms
    return statistics.median(samples)


def bench(num_perm: int, sizes: list[int]) -> None:
    points = []
    for size in sizes:
        tokens = [f"token-{i}".encode() for i in range(size)]
        m = MinHash(num_perm=num_perm, seed=1)  # 置換の生成は計測に含めない（Rust 側と同じ）

        def run():
            m.clear()
            m.update_batch(tokens)  # datasketch で最も速い CPU の経路

        points.append({"input_size": size, "reference_ms": median_ms(run)})
    print(json.dumps(points))


# 記事ごとに試す (num_perm, seed)。いつもの設定に加え、境目の値も混ぜる
ARTICLE_PARAMS = [(128, 1), (256, 42), (64, 2**32 - 1), (1, 0), (129, 623)]
NEAR_DUPLICATES = 2000


def text_cases(seed: int) -> None:
    """実文書の差分テスト。

    - 記事すべて × ARTICLE_PARAMS
    - 段落すべて（num_perm = 128, seed = 1）
    - ベンチマークに使う文書すべて（記事の連結を含む）
    - 段落に小さな編集を加えた近似重複の組 NEAR_DUPLICATES 件（jaccard も比べる）
    - となり合う記事どうしの組（似ていない文書の jaccard）
    """
    rng = random.Random(seed)
    arts = corpus.articles()
    paras = corpus.all_paragraphs(arts)
    empty = {}  # (num_perm, seed) ごとの空の MinHash。copy() で置換の生成を1回で済ませる

    def minhash(grams: list[str], num_perm: int, perm_seed: int) -> MinHash:
        key = (num_perm, perm_seed)
        if key not in empty:
            empty[key] = MinHash(num_perm=num_perm, seed=perm_seed)
        m = empty[key].copy()
        m.update_batch([g.encode() for g in grams])
        return m

    def emit(case_id: str, mode: str, num_perm: int, perm_seed: int, a: str, b: str | None = None) -> None:
        ga = shingles(a, mode)
        ma = minhash(ga, num_perm, perm_seed)
        case = {
            "id": case_id, "mode": mode, "num_perm": num_perm, "seed": perm_seed,
            "a": a, "shingles_a": len(ga), "digest_a": digest(ga), "expected_a": [int(v) for v in ma.hashvalues],
            "b": None, "shingles_b": None, "digest_b": None, "expected_b": None, "expected_jaccard": None,
        }
        if b is not None:
            gb = shingles(b, mode)
            mb = minhash(gb, num_perm, perm_seed)
            case.update({
                "b": b, "shingles_b": len(gb), "digest_b": digest(gb),
                "expected_b": [int(v) for v in mb.hashvalues], "expected_jaccard": ma.jaccard(mb),
            })
        print(json.dumps(case, ensure_ascii=False))

    for a in arts:
        for num_perm, perm_seed in ARTICLE_PARAMS:
            emit(f"{a['lang']}:{a['revid']}", a["mode"], num_perm, perm_seed, a["text"])
    for p in paras:
        emit(p["id"], p["mode"], 128, 1, p["text"])
    for d in corpus.bench_documents(arts):
        emit(f"bench:{d['id']}", d["mode"], 128, 1, d["text"])
    for p in rng.sample(paras, min(NEAR_DUPLICATES, len(paras))):
        edited, ops = corpus.near_duplicate(p["text"], p["mode"], rng)
        emit(f"{p['id']}~{ops}", p["mode"], rng.choice([64, 128, 128, 256]), rng.choice([1, 1, 42, 2**32 - 1]), p["text"], edited)
    for x, y in zip(arts, arts[1:]):
        if x["mode"] == y["mode"]:
            emit(f"{x['lang']}:{x['revid']}|{y['lang']}:{y['revid']}", x["mode"], 128, 1, x["text"], y["text"])


def write_docs(path: str, docs: list[dict]) -> None:
    with open(path, "w") as f:
        json.dump([{"id": d["id"], "mode": d["mode"], "text": d["text"]} for d in docs], f, ensure_ascii=False)


def load_docs(path: str) -> list[tuple[str, list[bytes]]]:
    with open(path) as f:
        return [(d["id"], [g.encode() for g in shingles(d["text"], d["mode"])]) for d in json.load(f)]


def bench_text(num_perm: int, path: str) -> None:
    points = []
    m = MinHash(num_perm=num_perm, seed=1)  # 置換の生成は計測に含めない（Rust 側と同じ）
    for _, grams in load_docs(path):

        def run():
            m.clear()
            m.update_batch(grams)

        points.append({"input_size": len(grams), "reference_ms": median_ms(run)})
    print(json.dumps(points))


def bench_many(num_perm: int, path: str) -> None:
    docs = [grams for _, grams in load_docs(path)]
    # datasketch が「たくさんの文書」向けに用意している MinHash.bulk（置換の生成は最初の1回だけ）
    ms = median_ms(lambda: MinHash.bulk(docs, num_perm=num_perm, seed=1), min_runs=5, min_total_ms=1000.0)
    print(json.dumps({"documents": len(docs), "shingles": sum(map(len, docs)), "reference_ms": ms}))


if __name__ == "__main__":
    command, *args = sys.argv[1:]
    if command == "cases":
        cases(int(args[0]), int(args[1]))
    elif command == "text-cases":
        text_cases(int(args[0]))
    elif command == "bench-docs":
        write_docs(args[0], corpus.bench_documents(corpus.articles()))
    elif command == "many-docs":
        write_docs(args[0], corpus.all_paragraphs(corpus.articles()))
    elif command == "bench":
        bench(int(args[0]), [int(a) for a in args[1:]])
    elif command == "bench-text":
        bench_text(int(args[0]), args[1])
    elif command == "bench-many":
        bench_many(int(args[0]), args[1])
    else:
        sys.exit(f"unknown command: {command}")
