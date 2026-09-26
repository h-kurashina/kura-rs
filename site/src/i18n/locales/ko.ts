/**
 * 画面の文言（韓国語）。形は en.ts と同じにする（型で検査される）。
 * 可変部分の {name} などはそのまま残す。
 */
import type { Dictionary } from "@/i18n/dictionaries";

export const ko: Dictionary = {
  meta: {
    tagline: "코드는 복사하고, 증거는 곁에.",
    description:
      "AI 데이터 처리와 보안을 위한 작고 검증된 Rust 부품 모음. 소스를 프로젝트에 복사하거나 Python에서 바로 사용하세요.",
  },
  nav: {
    parts: "부품",
    language: "언어",
    github: "GitHub",
  },
  shelves: {
    ai: { label: "AI", description: "LLM 주변의 데이터 처리: 청킹, 정규화, 중복 제거, 토큰 계산." },
    security: { label: "보안", description: "로그 파싱, 패턴 매칭, 해싱, 바이너리 분석." },
  },
  search: {
    button: "부품 검색...",
    placeholder: "부품 검색...",
    empty: "부품을 찾을 수 없습니다.",
    pages: "페이지",
  },
  sidebar: {
    gettingStarted: "시작하기",
    allParts: "전체 부품",
    noParts: "아직 부품이 없습니다",
  },
  sample: {
    badge: "샘플 데이터",
    badgeTitle: "임시 수치입니다. 실제 측정값이 아닙니다.",
    noticeLead: "이 수치는 임시 값입니다.",
    noticeBody: "아직 측정되지 않았으므로 실제 결과로 해석해서는 안 됩니다. 이 부품은 registry 파일에서 {code}(으)로 표시되어 있습니다.",
    strip: "샘플 데이터입니다. 이 수치는 측정값이 아닌 임시 값입니다.",
  },
  common: {
    copy: "복사",
    copied: "복사됨",
    copyLabel: "클립보드에 복사",
    upTo: "최대",
    faster: "빠름",
    slower: "느림",
    fasterBy: "{factor} 빠름",
    slowerBy: "{factor} 느림",
  },
  home: {
    browse: "부품 둘러보기",
    newPart: "새 부품: {title}",
    github: "GitHub에서 보기",
    showcase: {
      verification: "검증",
      benchmarks: "벤치마크",
      install: "설치",
      usage: "사용법",
      shelves: "선반",
      shelvesLead: "부품은 용도별로 묶여 있습니다.",
      partsCount: "부품 {n}개",
      partCount: "부품 1개",
      open: "{title} 열기",
      principles: "kura-rs의 작동 방식",
    },
    viewShelf: "선반 보기",
    noParts: "이 선반에는 아직 부품이 없습니다.",
    points: [
      {
        title: "코드를 복사하세요",
        body: "Rust 부품은 평범한 소스 파일입니다. {code}(으)로 프로젝트에 복사하면, 그때부터 자유롭게 읽고 고칠 수 있는 여러분의 코드가 됩니다.",
      },
      { title: "또는 Python용으로 설치", body: "같은 부품을 PyO3로 빌드해 하나의 패키지 kura-rs로 배포합니다." },
      {
        title: "증거를 남기세요",
        body: "각 부품에는 비교 테스트에 쓴 참조 구현, 통과한 케이스 수, 참조 구현보다 빨라지는 지점이 함께 기록됩니다.",
      },
    ],
  },
  list: {
    title: "부품",
    lead: "모든 부품은 참조 구현과 대조해 검증하고, 다양한 입력 크기에서 벤치마크합니다. 하나를 골라 복사하고, 증거는 곁에 두세요.",
    empty: "이 선반에는 아직 부품이 없습니다.",
  },
  card: {
    verifiedAgainst: "검증 기준",
  },
  part: {
    breadcrumb: "부품",
    reference: "참조 구현: {name}",
    onThisPage: "이 페이지의 내용",
    previous: "이전 부품",
    added: "{date} 추가",
    next: "다음 부품",
    sparkline: "kura-rs (Rust)와 {name}의 벤치마크 곡선",
    promo: {
      title: "부품 추가하기",
      body: "각 부품은 registry/ 안의 JSON 파일 하나입니다. 페이지는 이 파일에서 생성됩니다.",
      action: "GitHub 열기",
    },
    toc: {
      installation: "설치",
      usage: "사용법",
      verification: "검증",
      benchmarks: "벤치마크",
      source: "소스 및 의존성",
    },
    proof: {
      matches: "{name}와(과) 일치",
      cases: "케이스",
      breakEven: "손익분기점",
      inputs: "입력",
      always: "항상",
      never: "없음",
    },
    install: {
      copiesOne: "소스 파일을 프로젝트에 복사합니다. 이후 코드는 여러분의 것입니다.",
      copiesMany: "소스 파일 {n}개를 프로젝트에 복사합니다. 이후 코드는 여러분의 것입니다.",
      conflict: "다른 도구가 이미 {kura} 명령을 설치했다면 {alt}로 설치하고 {addAlt}를 사용하세요.",
    },
    verification: {
      lead: "{name} {version} ({language})와(과) {method}(으)로 대조했습니다. 두 구현에 같은 입력을 넣고 출력을 비교합니다.",
      reference: "참조 구현",
      cases: "테스트 케이스",
      passed: "통과",
      allMatch: "모든 케이스 일치",
      mismatched: "{n}개 불일치",
      lastRun: "마지막 실행",
      methods: {
        differential: "차분 테스트",
        property: "속성 기반 테스트",
        golden: "골든 파일",
      },
    },
    benchmarks: {
      lead: "다양한 입력 크기에서 kura-rs (Rust)와 {name}을(를) 비교합니다.",
      alwaysFaster: "측정한 모든 입력 크기에서 더 빠름",
      neverFaster: "측정한 어떤 입력 크기에서도 더 빠르지 않음",
      crossover: "입력 약 {size}개부터 더 빠름",
      upTo: ", 최대 {factor} 빠름",
      showTable: "데이터 표 보기",
      environment: "측정 환경: {cpu}, {os}. kura-rs는 {rust}, 참조 구현은 {runtime}.",
      inputUnit: "입력 크기: {unit}.",
      inputSize: "입력 크기",
      speedup: "속도 향상",
    },
    source: {
      files: "파일",
      dependencies: "crate 의존성",
      none: "없음. 표준 라이브러리만 사용합니다.",
      features: "features: {list}",
    },
  },
  chart: {
    legend: "범례",
    yScale: "Y축 스케일",
    log: "로그",
    linear: "선형",
    xLabel: "입력 크기 (로그)",
    breakEven: "손익분기점 ≈ {size}",
    tooltipSize: "입력 크기 {size}",
    caption: "1회 실행당 소요 시간 (낮을수록 좋음).",
    captionShaded: "음영 영역은 참조 구현이 더 빠른 구간입니다.",
  },
  notFound: {
    title: "페이지를 찾을 수 없습니다",
    body: "찾으시는 페이지가 존재하지 않습니다.",
    back: "부품 목록으로 돌아가기",
  },
};
