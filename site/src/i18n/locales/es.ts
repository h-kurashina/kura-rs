/**
 * 画面の文言（スペイン語）。形は en.ts と同じにする（型で検査される）。
 * 可変部分の {name} などはそのまま残す。
 */
import type { Dictionary } from "@/i18n/dictionaries";

export const es: Dictionary = {
  meta: {
    tagline: "Copia el código, conserva la prueba.",
    description:
      "Piezas pequeñas y verificadas de Rust para procesamiento de datos de IA y seguridad. Copia el código fuente en tu proyecto o úsalas desde Python.",
  },
  nav: {
    parts: "Piezas",
    language: "Idioma",
    github: "GitHub",
  },
  shelves: {
    ai: {
      label: "IA",
      description: "Procesamiento de datos en torno a LLM: fragmentación, normalización, deduplicación y conteo de tokens.",
    },
    security: {
      label: "Seguridad",
      description: "Análisis de logs, coincidencia de patrones, hashing y análisis de binarios.",
    },
  },
  search: {
    button: "Buscar piezas...",
    placeholder: "Buscar piezas...",
    empty: "No se encontraron piezas.",
    pages: "Páginas",
  },
  sidebar: {
    gettingStarted: "Primeros pasos",
    allParts: "Todas las piezas",
    noParts: "Aún no hay piezas",
  },
  sample: {
    badge: "Datos de ejemplo",
    badgeTitle: "Cifras provisionales. No son una medición real.",
    noticeLead: "Estas cifras son provisionales.",
    noticeBody:
      "Todavía no se han medido y no deben interpretarse como resultados reales. Esta pieza está marcada con {code} en su archivo del registro.",
    strip: "Datos de ejemplo. Estas cifras son provisionales, no mediciones.",
  },
  common: {
    copy: "Copiar",
    copied: "Copiado",
    copyLabel: "Copiar al portapapeles",
    upTo: "Hasta",
    faster: "más rápido",
    slower: "más lento",
    fasterBy: "{factor} más rápido",
    slowerBy: "{factor} más lento",
  },
  home: {
    browse: "Ver piezas",
    newPart: "Nuevo: {title}",
    github: "Ver en GitHub",
    showcase: {
      verification: "Verificación",
      benchmarks: "Benchmarks",
      install: "Instalación",
      usage: "Uso",
      shelves: "Estantes",
      shelvesLead: "Las piezas se agrupan según su propósito.",
      partsCount: "{n} piezas",
      partCount: "1 pieza",
      open: "Abrir {title}",
      principles: "Cómo funciona kura-rs",
    },
    viewShelf: "Ver estante",
    noParts: "Aún no hay piezas en este estante.",
    points: [
      {
        title: "Copia el código",
        body: "Las piezas de Rust son archivos de código fuente sin más. {code} las copia en tu proyecto y, desde ese momento, son tuyas para leerlas y modificarlas.",
      },
      {
        title: "O instálalas para Python",
        body: "Las mismas piezas se compilan con PyO3 y se publican en un único paquete, kura-rs.",
      },
      {
        title: "Conserva la prueba",
        body: "Cada pieza indica la referencia con la que se probó, cuántos casos pasaron y a partir de qué punto es más rápida que la referencia.",
      },
    ],
  },
  list: {
    title: "Piezas",
    lead: "Cada pieza se verifica frente a una implementación de referencia y se mide con distintos tamaños de entrada. Elige una, cópiala y conserva la prueba.",
    empty: "Aún no hay piezas en este estante.",
  },
  card: {
    verifiedAgainst: "Verificado frente a",
  },
  part: {
    breadcrumb: "Piezas",
    reference: "Referencia: {name}",
    onThisPage: "En esta página",
    previous: "Pieza anterior",
    added: "Añadido el {date}",
    next: "Pieza siguiente",
    sparkline: "Curva de benchmark de kura-rs (Rust) y {name}",
    promo: {
      title: "Añade una pieza",
      body: "Cada pieza es un único archivo JSON en registry/. Las páginas se generan a partir de él.",
      action: "Abrir GitHub",
    },
    toc: {
      installation: "Instalación",
      usage: "Uso",
      verification: "Verificación",
      benchmarks: "Benchmarks",
      source: "Código fuente y dependencias",
    },
    proof: {
      matches: "Coincide con {name}",
      cases: "casos",
      breakEven: "Punto de equilibrio",
      inputs: "entradas",
      always: "Siempre",
      never: "Nunca",
    },
    install: {
      copiesOne: "Copia el archivo de código fuente en tu proyecto. A partir de ahí, el código es tuyo.",
      copiesMany: "Copia {n} archivos de código fuente en tu proyecto. A partir de ahí, el código es tuyo.",
      conflict: "Si otra herramienta ya instala un comando {kura}, ejecuta {alt} y usa {addAlt} en su lugar.",
    },
    verification: {
      lead: "Verificado frente a {name} {version} ({language}). Método: {method}. Las mismas entradas se pasan a ambas implementaciones y se comparan las salidas.",
      reference: "Referencia",
      cases: "Casos de prueba",
      passed: "Superados",
      allMatch: "Todos los casos coinciden",
      mismatched: "{n} no coinciden",
      lastRun: "Última ejecución",
      methods: {
        differential: "Pruebas diferenciales",
        property: "Pruebas basadas en propiedades",
        golden: "Archivos golden",
      },
    },
    benchmarks: {
      lead: "kura-rs (Rust) frente a {name} con distintos tamaños de entrada.",
      alwaysFaster: "Más rápido en todos los tamaños de entrada medidos",
      neverFaster: "No es más rápido en ningún tamaño de entrada medido",
      crossover: "Más rápido a partir de unas {size} entradas",
      upTo: ", hasta {factor} más rápido",
      showTable: "Mostrar tabla de datos",
      environment: "Medido en {cpu}, {os}. kura-rs con {rust}; la referencia con {runtime}.",
      inputUnit: "Tamaño de entrada: {unit}.",
      inputSize: "Tamaño de entrada",
      speedup: "Aceleración",
    },
    source: {
      files: "Archivos",
      dependencies: "Dependencias (crates)",
      none: "Ninguna. Solo la biblioteca estándar.",
      features: "features: {list}",
    },
  },
  chart: {
    legend: "Leyenda",
    yScale: "Escala del eje Y",
    log: "Logarítmica",
    linear: "Lineal",
    xLabel: "Tamaño de entrada (log)",
    breakEven: "Punto de equilibrio ≈ {size}",
    tooltipSize: "Tamaño de entrada {size}",
    caption: "Tiempo real por ejecución (menos es mejor).",
    captionShaded: "La zona sombreada indica dónde la implementación de referencia es más rápida.",
  },
  notFound: {
    title: "Página no encontrada",
    body: "La página que buscas no existe.",
    back: "Volver a las piezas",
  },
};
