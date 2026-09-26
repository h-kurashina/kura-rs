/**
 * 画面の文言（簡体字中国語）。形は en.ts と同じにする（型で検査される）。
 * 可変部分の {name} などはそのまま残す。
 */
import type { Dictionary } from "@/i18n/dictionaries";

export const zh: Dictionary = {
  meta: {
    tagline: "代码拿走，证据留存。",
    description:
      "面向 AI 数据处理与安全的小型 Rust 组件，均经过验证。可将源码复制到你的项目中，也可以在 Python 中直接使用。",
  },
  nav: {
    parts: "组件",
    language: "语言",
    github: "GitHub",
  },
  shelves: {
    ai: { label: "AI", description: "LLM 相关的数据处理：分块、规范化、去重、token 计数。" },
    security: { label: "安全", description: "日志解析、模式匹配、哈希计算、二进制分析。" },
  },
  search: {
    button: "搜索组件...",
    placeholder: "搜索组件...",
    empty: "未找到组件。",
    pages: "页面",
  },
  sidebar: {
    gettingStarted: "快速开始",
    allParts: "全部组件",
    noParts: "暂无组件",
  },
  sample: {
    badge: "示例数据",
    badgeTitle: "占位数值，并非实测结果。",
    noticeLead: "以下数值仅为占位。",
    noticeBody: "这些数值尚未经过实测，请勿当作真实结果。该组件在其 registry 文件中标记为 {code}。",
    strip: "示例数据。这些数值仅为占位，并非实测结果。",
  },
  common: {
    copy: "复制",
    copied: "已复制",
    copyLabel: "复制到剪贴板",
    upTo: "最高",
    faster: "更快",
    slower: "更慢",
    fasterBy: "快 {factor}",
    slowerBy: "慢 {factor}",
  },
  home: {
    browse: "浏览组件",
    newPart: "新增：{title}",
    github: "在 GitHub 上查看",
    showcase: {
      verification: "验证",
      benchmarks: "基准测试",
      install: "安装",
      usage: "用法",
      shelves: "分类",
      shelvesLead: "组件按用途分类。",
      partsCount: "{n} 个组件",
      partCount: "1 个组件",
      open: "打开 {title}",
      principles: "kura-rs 的工作方式",
    },
    viewShelf: "查看分类",
    noParts: "该分类下暂无组件。",
    points: [
      {
        title: "复制代码",
        body: "Rust 组件就是普通的源码文件。{code} 会将它们复制到你的项目中，之后你可以自由阅读和修改。",
      },
      { title: "或通过 Python 安装", body: "同样的组件使用 PyO3 构建，并作为一个包 kura-rs 发布。" },
      {
        title: "留存证据",
        body: "每个组件都会列出用于对照测试的参考实现、通过的用例数量，以及从何种规模起比参考实现更快。",
      },
    ],
  },
  list: {
    title: "组件",
    lead: "每个组件都与参考实现进行对照验证，并在不同输入规模下进行基准测试。选一个，复制它，证据一并带走。",
    empty: "该分类下暂无组件。",
  },
  card: {
    verifiedAgainst: "对照验证",
  },
  part: {
    breadcrumb: "组件",
    reference: "参考实现：{name}",
    onThisPage: "本页内容",
    previous: "上一个组件",
    added: "添加于 {date}",
    next: "下一个组件",
    sparkline: "kura-rs（Rust）与 {name} 的基准测试曲线",
    promo: {
      title: "添加组件",
      body: "每个组件就是 registry/ 中的一个 JSON 文件，页面由它自动生成。",
      action: "打开 GitHub",
    },
    toc: {
      installation: "安装",
      usage: "用法",
      verification: "验证",
      benchmarks: "基准测试",
      source: "源码与依赖",
    },
    proof: {
      matches: "与 {name} 一致",
      cases: "个用例",
      breakEven: "盈亏平衡点",
      inputs: "条输入",
      always: "始终",
      never: "无",
    },
    install: {
      copiesOne: "将源码文件复制到你的项目中。此后代码完全归你所有。",
      copiesMany: "将 {n} 个源码文件复制到你的项目中。此后代码完全归你所有。",
      conflict: "如果其他工具已经安装了 {kura} 命令，请运行 {alt}，并改用 {addAlt}。",
    },
    verification: {
      lead: "使用{method}与 {name} {version}（{language}）进行对照：向两个实现输入相同数据，并比较输出结果。",
      reference: "参考实现",
      cases: "测试用例",
      passed: "通过",
      allMatch: "全部一致",
      mismatched: "{n} 个不一致",
      lastRun: "最近运行",
      methods: {
        differential: "差分测试",
        property: "基于属性的测试",
        golden: "黄金文件比对",
      },
    },
    benchmarks: {
      lead: "在不同输入规模下对比 kura-rs（Rust）与 {name}。",
      alwaysFaster: "在所有测量的输入规模下均更快",
      neverFaster: "在所有测量的输入规模下均未更快",
      crossover: "输入约 {size} 条起更快",
      upTo: "，最高快 {factor}",
      showTable: "显示数据表",
      environment: "测量环境：{cpu}，{os}。kura-rs 使用 {rust}，参考实现使用 {runtime}。",
      inputUnit: "输入规模：{unit}。",
      inputSize: "输入规模",
      speedup: "加速比",
    },
    source: {
      files: "文件",
      dependencies: "crate 依赖",
      none: "无，仅使用标准库。",
      features: "features：{list}",
    },
  },
  chart: {
    legend: "图例",
    yScale: "Y 轴刻度",
    log: "对数",
    linear: "线性",
    xLabel: "输入规模（对数）",
    breakEven: "盈亏平衡点 ≈ {size}",
    tooltipSize: "输入规模 {size}",
    caption: "每次运行的耗时（越低越好）。",
    captionShaded: "阴影区域表示参考实现更快的范围。",
  },
  notFound: {
    title: "页面未找到",
    body: "你访问的页面不存在。",
    back: "返回组件列表",
  },
};
