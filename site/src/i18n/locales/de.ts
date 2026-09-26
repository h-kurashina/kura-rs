/**
 * 画面の文言（ドイツ語）。形は en.ts と同じにする（型で検査される）。
 * 可変部分の {name} などはそのまま残す。
 */
import type { Dictionary } from "@/i18n/dictionaries";

export const de: Dictionary = {
  meta: {
    tagline: "Code kopieren, Beweis behalten.",
    description:
      "Kleine, verifizierte Rust-Bausteine für KI-Datenverarbeitung und Sicherheit. Kopiere den Quellcode in dein Projekt oder nutze sie aus Python.",
  },
  nav: {
    parts: "Bausteine",
    language: "Sprache",
    github: "GitHub",
  },
  shelves: {
    ai: { label: "KI", description: "Datenverarbeitung rund um LLMs: Chunking, Normalisierung, Deduplizierung, Token-Zählung." },
    security: { label: "Sicherheit", description: "Log-Parsing, Mustererkennung, Hashing, Binäranalyse." },
  },
  search: {
    button: "Bausteine suchen...",
    placeholder: "Bausteine suchen...",
    empty: "Keine Bausteine gefunden.",
    pages: "Seiten",
  },
  sidebar: {
    gettingStarted: "Erste Schritte",
    allParts: "Alle Bausteine",
    noParts: "Noch keine Bausteine",
  },
  sample: {
    badge: "Beispieldaten",
    badgeTitle: "Platzhalterwerte. Keine echte Messung.",
    noticeLead: "Diese Werte sind Platzhalter.",
    noticeBody: "Sie wurden noch nicht gemessen und dürfen nicht als echte Ergebnisse gelesen werden. Dieser Baustein ist in seiner Registry-Datei mit {code} markiert.",
    strip: "Beispieldaten. Diese Werte sind Platzhalter, keine Messungen.",
  },
  common: {
    copy: "Kopieren",
    copied: "Kopiert",
    copyLabel: "In die Zwischenablage kopieren",
    upTo: "Bis zu",
    faster: "schneller",
    slower: "langsamer",
    fasterBy: "{factor} schneller",
    slowerBy: "{factor} langsamer",
  },
  home: {
    browse: "Bausteine ansehen",
    newPart: "Neu: {title}",
    github: "Auf GitHub ansehen",
    showcase: {
      verification: "Verifikation",
      benchmarks: "Benchmarks",
      install: "Installation",
      usage: "Verwendung",
      shelves: "Regale",
      shelvesLead: "Bausteine sind nach Einsatzzweck gruppiert.",
      partsCount: "{n} Bausteine",
      partCount: "1 Baustein",
      open: "{title} öffnen",
      principles: "So funktioniert kura-rs",
    },
    viewShelf: "Regal ansehen",
    noParts: "Noch keine Bausteine in diesem Regal.",
    points: [
      {
        title: "Code kopieren",
        body: "Rust-Bausteine sind einfache Quelldateien. {code} kopiert sie in dein Projekt – ab dann gehören sie dir: lesen und ändern erlaubt.",
      },
      { title: "Oder für Python installieren", body: "Dieselben Bausteine werden mit PyO3 gebaut und als ein Paket veröffentlicht: kura-rs." },
      {
        title: "Beweis behalten",
        body: "Jeder Baustein nennt die Referenz, gegen die er getestet wurde, wie viele Fälle bestanden wurden und ab wann er schneller ist als die Referenz.",
      },
    ],
  },
  list: {
    title: "Bausteine",
    lead: "Jeder Baustein wird gegen eine Referenzimplementierung geprüft und über verschiedene Eingabegrößen gebenchmarkt. Wähle einen aus, kopiere ihn, behalte den Beweis.",
    empty: "Noch keine Bausteine in diesem Regal.",
  },
  card: {
    verifiedAgainst: "Verifiziert gegen",
  },
  part: {
    breadcrumb: "Bausteine",
    reference: "Referenz: {name}",
    onThisPage: "Auf dieser Seite",
    previous: "Vorheriger Baustein",
    added: "Hinzugefügt am {date}",
    next: "Nächster Baustein",
    sparkline: "Benchmark-Kurve von kura-rs (Rust) und {name}",
    promo: {
      title: "Baustein beitragen",
      body: "Jeder Baustein ist eine JSON-Datei in registry/. Die Seiten werden daraus generiert.",
      action: "GitHub öffnen",
    },
    toc: {
      installation: "Installation",
      usage: "Verwendung",
      verification: "Verifikation",
      benchmarks: "Benchmarks",
      source: "Quellcode & Abhängigkeiten",
    },
    proof: {
      matches: "Stimmt mit {name} überein",
      cases: "Fälle",
      breakEven: "Break-even",
      inputs: "Eingaben",
      always: "Immer",
      never: "Nie",
    },
    install: {
      copiesOne: "Kopiert die Quelldatei in dein Projekt. Ab dann gehört der Code dir.",
      copiesMany: "Kopiert {n} Quelldateien in dein Projekt. Ab dann gehört der Code dir.",
      conflict: "Wenn ein anderes Tool bereits einen {kura}-Befehl installiert, führe {alt} aus und nutze stattdessen {addAlt}.",
    },
    verification: {
      lead: "Geprüft gegen {name} {version} ({language}). Verfahren: {method}. Beide Implementierungen erhalten dieselben Eingaben, die Ausgaben werden verglichen.",
      reference: "Referenz",
      cases: "Testfälle",
      passed: "Bestanden",
      allMatch: "Alle Fälle stimmen überein",
      mismatched: "{n} Abweichungen",
      lastRun: "Letzter Lauf",
      methods: {
        differential: "Differenzielles Testen",
        property: "Property-based Testing",
        golden: "Golden Files",
      },
    },
    benchmarks: {
      lead: "kura-rs (Rust) im Vergleich zu {name} über verschiedene Eingabegrößen.",
      alwaysFaster: "Bei jeder gemessenen Eingabegröße schneller",
      neverFaster: "Bei keiner gemessenen Eingabegröße schneller",
      crossover: "Schneller ab etwa {size} Eingaben",
      upTo: ", bis zu {factor} schneller",
      showTable: "Datentabelle anzeigen",
      environment: "Gemessen auf {cpu}, {os}. kura-rs mit {rust}; Referenz mit {runtime}.",
      inputUnit: "Eingabegröße: {unit}.",
      inputSize: "Eingabegröße",
      speedup: "Speedup",
    },
    source: {
      files: "Dateien",
      dependencies: "Crate-Abhängigkeiten",
      none: "Keine. Nur die Standardbibliothek.",
      features: "Features: {list}",
    },
  },
  chart: {
    legend: "Legende",
    yScale: "Skala der Y-Achse",
    log: "Log",
    linear: "Linear",
    xLabel: "Eingabegröße (log)",
    breakEven: "Break-even ≈ {size}",
    tooltipSize: "Eingabegröße {size}",
    caption: "Laufzeit pro Durchlauf (niedriger ist besser).",
    captionShaded: "Im schattierten Bereich ist die Referenzimplementierung schneller.",
  },
  notFound: {
    title: "Seite nicht gefunden",
    body: "Die gesuchte Seite existiert nicht.",
    back: "Zurück zu den Bausteinen",
  },
};
