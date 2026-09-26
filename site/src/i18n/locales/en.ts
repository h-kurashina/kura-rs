/**
 * 画面の文言（英語・既定）。ほかの言語はこのファイルと同じ形で作る。
 * クライアント部品にもそのまま渡すので、関数は入れずに文字列だけにする。
 * 可変部分は {name} のように書き、fill() で埋める。
 */

export const en = {
  meta: {
    tagline: "Copy the code, keep the proof.",
    description:
      "Small, verified Rust parts for AI data processing and security. Copy the source into your project, or use them from Python.",
  },
  nav: {
    parts: "Parts",
    language: "Language",
    github: "GitHub",
  },
  shelves: {
    ai: { label: "AI", description: "Data processing around LLMs: chunking, normalization, dedup, token counting." },
    security: { label: "Security", description: "Log parsing, pattern matching, hashing, binary analysis." },
  },
  search: {
    button: "Search parts...",
    placeholder: "Search parts...",
    empty: "No parts found.",
    pages: "Pages",
  },
  sidebar: {
    gettingStarted: "Getting started",
    allParts: "All parts",
    noParts: "No parts yet",
  },
  sample: {
    badge: "Sample data",
    badgeTitle: "Placeholder numbers. Not a real measurement.",
    noticeLead: "These numbers are placeholders.",
    noticeBody: "They have not been measured yet and must not be read as real results. This part is marked {code} in its registry file.",
    strip: "Sample data. These figures are placeholders, not measurements.",
  },
  common: {
    copy: "Copy",
    copied: "Copied",
    copyLabel: "Copy to clipboard",
    upTo: "Up to",
    faster: "faster",
    slower: "slower",
    fasterBy: "{factor} faster",
    slowerBy: "{factor} slower",
  },
  home: {
    browse: "Browse parts",
    newPart: "New: {title}",
    github: "View on GitHub",
    showcase: {
      verification: "Verification",
      benchmarks: "Benchmarks",
      install: "Install",
      usage: "Usage",
      shelves: "Shelves",
      shelvesLead: "Parts are grouped by what they are for.",
      partsCount: "{n} parts",
      partCount: "1 part",
      open: "Open {title}",
      principles: "How kura-rs works",
    },
    viewShelf: "View shelf",
    noParts: "No parts on this shelf yet.",
    points: [
      {
        title: "Copy the code",
        body: "Rust parts are plain source files. {code} copies them into your project, and from then on they are yours to read and change.",
      },
      { title: "Or install for Python", body: "The same parts are built with PyO3 and published as one package, kura-rs." },
      {
        title: "Keep the proof",
        body: "Each part lists the reference it was tested against, how many cases passed, and where it gets faster than the reference.",
      },
    ],
  },
  list: {
    title: "Parts",
    lead: "Every part is checked against a reference implementation and benchmarked across input sizes. Pick one, copy it, keep the proof.",
    empty: "No parts on this shelf yet.",
  },
  card: {
    verifiedAgainst: "Verified against",
  },
  part: {
    breadcrumb: "Parts",
    reference: "Reference: {name}",
    onThisPage: "On this page",
    previous: "Previous part",
    added: "Added {date}",
    next: "Next part",
    sparkline: "Benchmark curve of kura-rs (Rust) and {name}",
    promo: {
      title: "Add a part",
      body: "Every part is one JSON file in registry/. The pages are generated from it.",
      action: "Open GitHub",
    },
    toc: {
      installation: "Installation",
      usage: "Usage",
      verification: "Verification",
      benchmarks: "Benchmarks",
      source: "Source & dependencies",
    },
    proof: {
      matches: "Matches {name}",
      cases: "cases",
      breakEven: "Break-even",
      inputs: "inputs",
      always: "Always",
      never: "Never",
    },
    install: {
      copiesOne: "Copies the source file into your project. You own the code from then on.",
      copiesMany: "Copies {n} source files into your project. You own the code from then on.",
      conflict: "If another tool already installs a {kura} command, run {alt} and use {addAlt} instead.",
    },
    verification: {
      lead: "Checked against {name} {version} ({language}) using {method}: the same inputs go to both implementations and the outputs are compared.",
      reference: "Reference",
      cases: "Test cases",
      passed: "Passed",
      allMatch: "All cases match",
      mismatched: "{n} mismatched",
      lastRun: "Last run",
      methods: {
        differential: "Differential testing",
        property: "Property-based testing",
        golden: "Golden files",
      },
    },
    benchmarks: {
      lead: "kura-rs (Rust) against {name} across input sizes.",
      alwaysFaster: "Faster at every measured input size",
      neverFaster: "Not faster at any measured input size",
      crossover: "Faster from about {size} inputs",
      upTo: ", up to {factor} faster",
      showTable: "Show data table",
      environment: "Measured on {cpu}, {os}. kura-rs with {rust}; reference on {runtime}.",
      inputUnit: "Input size: {unit}.",
      inputSize: "Input size",
      speedup: "Speedup",
    },
    source: {
      files: "Files",
      dependencies: "Crate dependencies",
      none: "None. Only the standard library.",
      features: "features: {list}",
    },
  },
  chart: {
    legend: "Legend",
    yScale: "Y axis scale",
    log: "Log",
    linear: "Linear",
    xLabel: "Input size (log)",
    breakEven: "Break-even ≈ {size}",
    tooltipSize: "Input size {size}",
    caption: "Wall time per run (lower is better).",
    captionShaded: "The shaded region is where the reference implementation is faster.",
  },
  notFound: {
    title: "Page not found",
    body: "The page you are looking for does not exist.",
    back: "Back to parts",
  },
};
