/**
 * 画面の文言（日本語）。形は en.ts と同じにする（型で検査される）。
 * 可変部分の {name} などはそのまま残す。
 */
import type { Dictionary } from "@/i18n/dictionaries";

export const ja: Dictionary = {
  meta: {
    tagline: "コードはコピーして、証拠は手元に。",
    description:
      "AI のデータ処理とセキュリティのための、検証済みの小さな Rust 部品集。ソースを自分のプロジェクトにコピーするか、Python から使えます。",
  },
  nav: {
    parts: "部品",
    language: "言語",
    github: "GitHub",
  },
  shelves: {
    ai: { label: "AI", description: "LLM 周辺のデータ処理：チャンク分割、正規化、重複除去、トークン計測。" },
    security: { label: "セキュリティ", description: "ログのパース、パターンマッチ、ハッシュ計算、バイナリ解析。" },
  },
  search: {
    button: "部品を検索...",
    placeholder: "部品を検索...",
    empty: "該当する部品はありません。",
    pages: "ページ",
  },
  sidebar: {
    gettingStarted: "はじめに",
    allParts: "すべての部品",
    noParts: "まだ部品はありません",
  },
  sample: {
    badge: "サンプルデータ",
    badgeTitle: "仮の数値です。実測ではありません。",
    noticeLead: "この数値は仮のものです。",
    noticeBody: "まだ計測していないため、実際の結果として読まないでください。この部品のレジストリファイルには {code} が付いています。",
    strip: "サンプルデータです。この数値は仮のもので、実測ではありません。",
  },
  common: {
    copy: "コピー",
    copied: "コピーしました",
    copyLabel: "クリップボードにコピー",
    upTo: "最大",
    faster: "速い",
    slower: "遅い",
    fasterBy: "{factor} 速い",
    slowerBy: "{factor} 遅い",
  },
  home: {
    browse: "部品を見る",
    newPart: "新着: {title}",
    github: "GitHub で見る",
    showcase: {
      verification: "検証",
      benchmarks: "ベンチマーク",
      install: "インストール",
      usage: "使い方",
      shelves: "棚",
      shelvesLead: "部品は用途ごとの棚に並んでいます。",
      partsCount: "{n} 個の部品",
      partCount: "1 個の部品",
      open: "{title} を開く",
      principles: "kura-rs の考え方",
    },
    viewShelf: "この棚を見る",
    noParts: "この棚にはまだ部品がありません。",
    points: [
      {
        title: "コードをコピーする",
        body: "Rust の部品はただのソースファイルです。{code} で自分のプロジェクトにコピーしたら、あとは自由に読んで書き換えられます。",
      },
      { title: "Python からも使える", body: "同じ部品を PyO3 でビルドし、kura-rs という1つのパッケージとして配布します。" },
      {
        title: "証拠を手元に残す",
        body: "部品ごとに、比較した参照実装、成功したテストケースの数、どの入力サイズから参照実装より速くなるかを示します。",
      },
    ],
  },
  list: {
    title: "部品",
    lead: "どの部品も参照実装と突き合わせて検証し、入力サイズごとに計測しています。選んで、コピーして、証拠ごと持ち帰ってください。",
    empty: "この棚にはまだ部品がありません。",
  },
  card: {
    verifiedAgainst: "比較対象",
  },
  part: {
    breadcrumb: "部品",
    reference: "参照実装: {name}",
    onThisPage: "このページの内容",
    previous: "前の部品",
    added: "{date} に追加",
    next: "次の部品",
    sparkline: "kura-rs（Rust）と {name} のベンチマーク曲線",
    promo: {
      title: "部品を追加する",
      body: "部品は registry/ にある JSON ファイル1つです。ページはそこから自動で作られます。",
      action: "GitHub を開く",
    },
    toc: {
      installation: "インストール",
      usage: "使い方",
      verification: "検証",
      benchmarks: "ベンチマーク",
      source: "ソースと依存関係",
    },
    proof: {
      matches: "{name} と一致",
      cases: "ケース",
      breakEven: "損益分岐点",
      inputs: "件",
      always: "常に",
      never: "なし",
    },
    install: {
      copiesOne: "ソースファイルを自分のプロジェクトにコピーします。以後、コードはあなたのものです。",
      copiesMany: "{n} 個のソースファイルを自分のプロジェクトにコピーします。以後、コードはあなたのものです。",
      conflict: "別のツールの {kura} コマンドとぶつかる場合は、{alt} で入れて {addAlt} を使ってください。",
    },
    verification: {
      lead: "{name} {version}（{language}）と{method}で比較しました。同じ入力を両方の実装に与え、出力を突き合わせています。",
      reference: "参照実装",
      cases: "テストケース",
      passed: "成功",
      allMatch: "すべて一致",
      mismatched: "{n} 件不一致",
      lastRun: "最終実行",
      methods: {
        differential: "差分テスト",
        property: "性質ベースのテスト",
        golden: "期待値ファイルとの比較",
      },
    },
    benchmarks: {
      lead: "入力サイズごとに kura-rs（Rust）と {name} を比べています。",
      alwaysFaster: "計測したすべての入力サイズで速い",
      neverFaster: "計測したどの入力サイズでも速くない",
      crossover: "入力が約 {size} 件を超えると速い",
      upTo: "。最大 {factor} 速い",
      showTable: "数値の表を見る",
      environment: "計測環境: {cpu}、{os}。kura-rs は {rust}、参照実装は {runtime}。",
      inputUnit: "入力サイズ: {unit}。",
      inputSize: "入力サイズ",
      speedup: "倍率",
    },
    source: {
      files: "ファイル",
      dependencies: "依存する crate",
      none: "なし。標準ライブラリのみです。",
      features: "feature: {list}",
    },
  },
  chart: {
    legend: "凡例",
    yScale: "縦軸の目盛り",
    log: "対数",
    linear: "線形",
    xLabel: "入力サイズ（対数）",
    breakEven: "損益分岐点 ≈ {size}",
    tooltipSize: "入力サイズ {size}",
    caption: "1回あたりの処理時間（短いほど良い）。",
    captionShaded: "網掛けの範囲では参照実装の方が速い。",
  },
  notFound: {
    title: "ページが見つかりません",
    body: "お探しのページは存在しません。",
    back: "部品一覧へ戻る",
  },
};
