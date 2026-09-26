/**
 * 画面の文言（ポルトガル語・ブラジル）。形は en.ts と同じにする（型で検査される）。
 * 可変部分の {name} などはそのまま残す。
 */
import type { Dictionary } from "@/i18n/dictionaries";

export const pt: Dictionary = {
  meta: {
    tagline: "Copie o código, guarde a prova.",
    description:
      "Pequenas peças em Rust, verificadas, para processamento de dados de IA e segurança. Copie o código-fonte para o seu projeto ou use a partir do Python.",
  },
  nav: {
    parts: "Peças",
    language: "Idioma",
    github: "GitHub",
  },
  shelves: {
    ai: {
      label: "IA",
      description: "Processamento de dados em torno de LLMs: chunking, normalização, deduplicação, contagem de tokens.",
    },
    security: { label: "Segurança", description: "Parsing de logs, busca de padrões, hashing, análise de binários." },
  },
  search: {
    button: "Buscar peças...",
    placeholder: "Buscar peças...",
    empty: "Nenhuma peça encontrada.",
    pages: "Páginas",
  },
  sidebar: {
    gettingStarted: "Primeiros passos",
    allParts: "Todas as peças",
    noParts: "Nenhuma peça ainda",
  },
  sample: {
    badge: "Dados de exemplo",
    badgeTitle: "Números provisórios. Não é uma medição real.",
    noticeLead: "Estes números são provisórios.",
    noticeBody:
      "Eles ainda não foram medidos e não devem ser lidos como resultados reais. Esta peça está marcada como {code} no seu arquivo do registry.",
    strip: "Dados de exemplo. Estes valores são provisórios, não medições.",
  },
  common: {
    copy: "Copiar",
    copied: "Copiado",
    copyLabel: "Copiar para a área de transferência",
    upTo: "Até",
    faster: "mais rápido",
    slower: "mais lento",
    fasterBy: "{factor} mais rápido",
    slowerBy: "{factor} mais lento",
  },
  home: {
    browse: "Ver peças",
    newPart: "Novo: {title}",
    github: "Ver no GitHub",
    showcase: {
      verification: "Verificação",
      benchmarks: "Benchmarks",
      install: "Instalação",
      usage: "Uso",
      shelves: "Prateleiras",
      shelvesLead: "As peças são agrupadas pela sua finalidade.",
      partsCount: "{n} peças",
      partCount: "1 peça",
      open: "Abrir {title}",
      principles: "Como o kura-rs funciona",
    },
    viewShelf: "Ver prateleira",
    noParts: "Nenhuma peça nesta prateleira ainda.",
    points: [
      {
        title: "Copie o código",
        body: "As peças em Rust são arquivos de código-fonte comuns. O {code} os copia para o seu projeto e, a partir daí, eles são seus para ler e modificar.",
      },
      {
        title: "Ou instale para Python",
        body: "As mesmas peças são compiladas com PyO3 e publicadas em um único pacote, o kura-rs.",
      },
      {
        title: "Guarde a prova",
        body: "Cada peça informa a referência com que foi testada, quantos casos passaram e a partir de onde fica mais rápida que a referência.",
      },
    ],
  },
  list: {
    title: "Peças",
    lead: "Cada peça é verificada contra uma implementação de referência e passa por benchmarks em vários tamanhos de entrada. Escolha uma, copie e guarde a prova.",
    empty: "Nenhuma peça nesta prateleira ainda.",
  },
  card: {
    verifiedAgainst: "Verificado com",
  },
  part: {
    breadcrumb: "Peças",
    reference: "Referência: {name}",
    onThisPage: "Nesta página",
    previous: "Peça anterior",
    added: "Adicionado em {date}",
    next: "Próxima peça",
    sparkline: "Curva de benchmark do kura-rs (Rust) e de {name}",
    promo: {
      title: "Adicione uma peça",
      body: "Cada peça é um arquivo JSON em registry/. As páginas são geradas a partir dele.",
      action: "Abrir o GitHub",
    },
    toc: {
      installation: "Instalação",
      usage: "Uso",
      verification: "Verificação",
      benchmarks: "Benchmarks",
      source: "Código-fonte e dependências",
    },
    proof: {
      matches: "Igual ao {name}",
      cases: "casos",
      breakEven: "Ponto de equilíbrio",
      inputs: "entradas",
      always: "Sempre",
      never: "Nunca",
    },
    install: {
      copiesOne: "Copia o arquivo de código-fonte para o seu projeto. A partir daí, o código é seu.",
      copiesMany: "Copia {n} arquivos de código-fonte para o seu projeto. A partir daí, o código é seu.",
      conflict: "Se outra ferramenta já instala um comando {kura}, execute {alt} e use {addAlt} no lugar.",
    },
    verification: {
      lead: "Verificado com {name} {version} ({language}). Método: {method}. As mesmas entradas são enviadas às duas implementações e as saídas são comparadas.",
      reference: "Referência",
      cases: "Casos de teste",
      passed: "Aprovados",
      allMatch: "Todos os casos batem",
      mismatched: "{n} divergentes",
      lastRun: "Última execução",
      methods: {
        differential: "Teste diferencial",
        property: "Teste baseado em propriedades",
        golden: "Arquivos golden",
      },
    },
    benchmarks: {
      lead: "kura-rs (Rust) comparado com {name} em vários tamanhos de entrada.",
      alwaysFaster: "Mais rápido em todos os tamanhos de entrada medidos",
      neverFaster: "Não é mais rápido em nenhum tamanho de entrada medido",
      crossover: "Mais rápido a partir de cerca de {size} entradas",
      upTo: ", até {factor} mais rápido",
      showTable: "Mostrar tabela de dados",
      environment: "Medido em {cpu}, {os}. kura-rs com {rust}; a referência com {runtime}.",
      inputUnit: "Tamanho da entrada: {unit}.",
      inputSize: "Tamanho da entrada",
      speedup: "Aceleração",
    },
    source: {
      files: "Arquivos",
      dependencies: "Dependências (crates)",
      none: "Nenhuma. Apenas a biblioteca padrão.",
      features: "features: {list}",
    },
  },
  chart: {
    legend: "Legenda",
    yScale: "Escala do eixo Y",
    log: "Log",
    linear: "Linear",
    xLabel: "Tamanho da entrada (log)",
    breakEven: "Ponto de equilíbrio ≈ {size}",
    tooltipSize: "Tamanho da entrada {size}",
    caption: "Tempo real por execução (menor é melhor).",
    captionShaded: "A área sombreada é onde a implementação de referência é mais rápida.",
  },
  notFound: {
    title: "Página não encontrada",
    body: "A página que você procura não existe.",
    back: "Voltar para as peças",
  },
};
