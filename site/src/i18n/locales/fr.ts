/**
 * 画面の文言（フランス語）。形は en.ts と同じにする（型で検査される）。
 * 可変部分の {name} などはそのまま残す。
 * {method} は小文字にされずにそのまま入るので、文頭に置ける形にしている。
 */
import type { Dictionary } from "@/i18n/dictionaries";

export const fr: Dictionary = {
  meta: {
    tagline: "Copiez le code, gardez la preuve.",
    description:
      "De petits composants Rust vérifiés pour le traitement de données IA et la sécurité. Copiez le code source dans votre projet, ou utilisez-les depuis Python.",
  },
  nav: {
    parts: "Composants",
    language: "Langue",
    github: "GitHub",
  },
  shelves: {
    ai: {
      label: "IA",
      description: "Traitement de données autour des LLM : découpage, normalisation, déduplication, comptage de tokens.",
    },
    security: {
      label: "Sécurité",
      description: "Analyse de logs, recherche de motifs, hachage, analyse de binaires.",
    },
  },
  search: {
    button: "Rechercher un composant...",
    placeholder: "Rechercher un composant...",
    empty: "Aucun composant trouvé.",
    pages: "Pages",
  },
  sidebar: {
    gettingStarted: "Premiers pas",
    allParts: "Tous les composants",
    noParts: "Aucun composant pour l'instant",
  },
  sample: {
    badge: "Données d'exemple",
    badgeTitle: "Valeurs fictives. Il ne s'agit pas d'une mesure réelle.",
    noticeLead: "Ces valeurs sont fictives.",
    noticeBody:
      "Elles n'ont pas encore été mesurées et ne doivent pas être lues comme des résultats réels. Ce composant est marqué {code} dans son fichier de registre.",
    strip: "Données d'exemple. Ces chiffres sont fictifs et ne proviennent pas de mesures.",
  },
  common: {
    copy: "Copier",
    copied: "Copié",
    copyLabel: "Copier dans le presse-papiers",
    upTo: "Jusqu'à",
    faster: "plus rapide",
    slower: "plus lent",
    fasterBy: "{factor} plus rapide",
    slowerBy: "{factor} plus lent",
  },
  home: {
    browse: "Parcourir les composants",
    newPart: "Nouveau : {title}",
    github: "Voir sur GitHub",
    showcase: {
      verification: "Vérification",
      benchmarks: "Benchmarks",
      install: "Installation",
      usage: "Utilisation",
      shelves: "Rayons",
      shelvesLead: "Les composants sont regroupés par usage.",
      partsCount: "{n} composants",
      partCount: "1 composant",
      open: "Ouvrir {title}",
      principles: "Comment fonctionne kura-rs",
    },
    viewShelf: "Voir le rayon",
    noParts: "Aucun composant dans ce rayon pour l'instant.",
    points: [
      {
        title: "Copiez le code",
        body: "Les composants Rust sont de simples fichiers source. {code} les copie dans votre projet ; ensuite, ils vous appartiennent : lisez-les et modifiez-les librement.",
      },
      {
        title: "Ou installez-les pour Python",
        body: "Les mêmes composants sont compilés avec PyO3 et publiés dans un seul paquet, kura-rs.",
      },
      {
        title: "Gardez la preuve",
        body: "Chaque composant indique la référence à laquelle il a été comparé, le nombre de cas réussis et à partir de quand il devient plus rapide que la référence.",
      },
    ],
  },
  list: {
    title: "Composants",
    lead: "Chaque composant est vérifié par rapport à une implémentation de référence et mesuré sur plusieurs tailles d'entrée. Choisissez-en un, copiez-le, gardez la preuve.",
    empty: "Aucun composant dans ce rayon pour l'instant.",
  },
  card: {
    verifiedAgainst: "Vérifié par rapport à",
  },
  part: {
    breadcrumb: "Composants",
    reference: "Référence : {name}",
    onThisPage: "Sur cette page",
    previous: "Composant précédent",
    added: "Ajouté le {date}",
    next: "Composant suivant",
    sparkline: "Courbe de benchmark de kura-rs (Rust) et de {name}",
    promo: {
      title: "Ajouter un composant",
      body: "Chaque composant est un fichier JSON dans registry/. Les pages sont générées à partir de celui-ci.",
      action: "Ouvrir GitHub",
    },
    toc: {
      installation: "Installation",
      usage: "Utilisation",
      verification: "Vérification",
      benchmarks: "Benchmarks",
      source: "Code source et dépendances",
    },
    proof: {
      matches: "Conforme à {name}",
      cases: "cas",
      breakEven: "Seuil de rentabilité",
      inputs: "entrées",
      always: "Toujours",
      never: "Jamais",
    },
    install: {
      copiesOne: "Copie le fichier source dans votre projet. Le code vous appartient ensuite.",
      copiesMany: "Copie {n} fichiers source dans votre projet. Le code vous appartient ensuite.",
      conflict: "Si un autre outil installe déjà une commande {kura}, lance {alt} et utilise {addAlt} à la place.",
    },
    verification: {
      lead: "Vérifié par rapport à {name} {version} ({language}). Méthode : {method}. Les mêmes entrées sont fournies aux deux implémentations, puis leurs sorties sont comparées.",
      reference: "Référence",
      cases: "Cas de test",
      passed: "Réussis",
      allMatch: "Tous les cas concordent",
      mismatched: "{n} divergence(s)",
      lastRun: "Dernière exécution",
      methods: {
        differential: "Tests différentiels",
        property: "Tests basés sur les propriétés",
        golden: "Fichiers de référence (golden files)",
      },
    },
    benchmarks: {
      lead: "kura-rs (Rust) comparé à {name} sur plusieurs tailles d'entrée.",
      alwaysFaster: "Plus rapide pour toutes les tailles d'entrée mesurées",
      neverFaster: "Plus rapide pour aucune des tailles d'entrée mesurées",
      crossover: "Plus rapide à partir d'environ {size} entrées",
      upTo: ", jusqu'à {factor} plus rapide",
      showTable: "Afficher le tableau de données",
      environment: "Mesuré sur {cpu}, {os}. kura-rs avec {rust} ; la référence avec {runtime}.",
      inputUnit: "Taille d'entrée : {unit}.",
      inputSize: "Taille d'entrée",
      speedup: "Accélération",
    },
    source: {
      files: "Fichiers",
      dependencies: "Dépendances (crates)",
      none: "Aucune. Uniquement la bibliothèque standard.",
      features: "features : {list}",
    },
  },
  chart: {
    legend: "Légende",
    yScale: "Échelle de l'axe Y",
    log: "Log",
    linear: "Linéaire",
    xLabel: "Taille d'entrée (log)",
    breakEven: "Seuil de rentabilité ≈ {size}",
    tooltipSize: "Taille d'entrée {size}",
    caption: "Temps d'exécution par itération (plus bas = meilleur).",
    captionShaded: "La zone grisée indique où l'implémentation de référence est plus rapide.",
  },
  notFound: {
    title: "Page introuvable",
    body: "La page que vous recherchez n'existe pas.",
    back: "Retour aux composants",
  },
};
