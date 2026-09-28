import Link from "next/link";
import type { Metadata } from "next";
import { CodeBlock } from "@/components/code-block";
import { PartCard } from "@/components/part-card";
import { JsonLd } from "@/components/json-ld";
import { buttonClass } from "@/components/ui";
import { fill, getDictionary, type Locale } from "@/i18n/dictionaries";
import { fillNode } from "@/i18n/fill-node";
import { localePath, localizeText } from "@/i18n/locale";
import { getLatestPart, getPart, getPartsByShelf } from "@/lib/parts";
import { absoluteUrl, pageMetadata } from "@/lib/seo";
import { CLI_PACKAGE, SITE } from "@/lib/site";

export function homeMetadata(locale: Locale): Metadata {
  return pageMetadata({ locale, path: "/", description: getDictionary(locale).meta.description, image: "/og/site.png" });
}

export function HomeView({ locale }: { locale: Locale }) {
  const t = getDictionary(locale);
  const latest = getLatestPart();
  const featured = latest ? getPart(latest.name) : undefined;
  const shelves = getPartsByShelf();

  return (
    <div className="mx-auto max-w-6xl px-5 sm:px-8">
      <JsonLd data={{ "@type": "WebSite", name: SITE.name, url: absoluteUrl(localePath(locale, "/")), description: t.meta.description, inLanguage: locale }} />
      <section className="grid items-center gap-10 border-b py-14 sm:py-20 lg:grid-cols-[1.2fr_1fr] lg:gap-16">
        <div className="space-y-6">
          {featured && (
            <Link href={localePath(locale, `/parts/${featured.name}/`)} className="inline-flex items-center gap-2 text-xs font-medium text-muted-foreground hover:text-foreground">
              {fill(t.home.newPart, { title: localizeText(featured, locale).title })}
              <span aria-hidden="true">→</span>
            </Link>
          )}
          <h1 className="max-w-2xl text-4xl leading-[1.25] font-semibold tracking-tight text-balance sm:text-5xl">{t.meta.tagline}</h1>
          <p className="max-w-xl text-base leading-8 text-muted-foreground">{t.meta.description}</p>
          <div className="flex flex-wrap gap-3">
            <Link href={localePath(locale, "/parts/")} className={buttonClass("primary")}>
              {t.home.browse}<span aria-hidden="true">→</span>
            </Link>
            <a href={SITE.github} className={buttonClass("outline")}>{t.home.github}</a>
          </div>
        </div>
        {featured && (
          <div className="min-w-0 space-y-5 rounded-xl border bg-muted/30 p-5 sm:p-6">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <h2 className="text-sm font-medium">{t.home.showcase.install}</h2>
              <span className="font-mono text-xs text-muted-foreground">{featured.name}</span>
            </div>
            <CodeBlock icon="terminal" labels={t.common} sources={[
              { id: "rust", label: "cargo", code: `cargo install ${CLI_PACKAGE}\nkura add ${featured.name}`, lang: "bash" },
              ...(featured.python ? [{ id: "python", label: "pip", code: `pip install ${featured.python.package}`, lang: "bash" as const }] : []),
            ]} />
            <Link href={localePath(locale, `/parts/${featured.name}/#usage`)} className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground">
              {t.home.showcase.usage}<span aria-hidden="true">→</span>
            </Link>
          </div>
        )}
      </section>

      <section className="space-y-8 py-12 sm:py-16">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <h2 className="text-2xl font-semibold tracking-tight">{t.nav.parts}</h2>
          <Link href={localePath(locale, "/parts/")} className="text-sm text-muted-foreground hover:text-foreground">{t.home.browse} <span aria-hidden="true">→</span></Link>
        </div>
        <div className="space-y-8">
          {shelves.map(({ shelf, parts }) => (
            <div key={shelf} className="grid gap-4 lg:grid-cols-[200px_1fr] lg:gap-8">
              <div className="space-y-2 pt-1">
                <Link href={localePath(locale, `/parts/#${shelf}`)} className="text-sm font-medium hover:underline underline-offset-4">{t.shelves[shelf].label}</Link>
                <p className="text-xs leading-6 text-muted-foreground">{t.shelves[shelf].description}</p>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                {parts.slice(0, 2).map((part) => <PartCard key={part.name} part={part} locale={locale} t={t} />)}
                {parts.length === 0 && <p className="col-span-full rounded-lg border border-dashed p-6 text-sm text-muted-foreground">{t.home.noParts}</p>}
                {parts.length > 2 && <Link href={localePath(locale, `/parts/#${shelf}`)} className="col-span-full text-sm text-muted-foreground hover:text-foreground">{t.home.viewShelf} →</Link>}
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="border-t py-12 sm:py-16">
        <h2 className="mb-8 text-xl font-semibold tracking-tight">{t.home.showcase.principles}</h2>
        <div className="grid gap-8 md:grid-cols-3 md:gap-12">
          {t.home.points.map((point, i) => (
            <div key={point.title} className="space-y-3">
              <span className="font-mono text-xs text-muted-foreground">0{i + 1}</span>
              <h3 className="text-sm font-medium">{point.title}</h3>
              <p className="text-sm leading-7 text-muted-foreground">{fillNode(point.body, { code: <code className="font-mono text-xs text-foreground">kura add &lt;name&gt;</code> })}</p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
