import Link from "next/link";
import type { CSSProperties } from "react";
import { CodeBlock } from "@/components/code-block";
import { SampleBadge } from "@/components/sample-badge";
import { Sparkline } from "@/components/sparkline";
import { buttonClass, Card, CardHeader, Pill } from "@/components/ui";
import type { Part } from "@/generated/Part";
import { fill, getDictionary, type Dictionary, type Locale } from "@/i18n/dictionaries";
import { fillNode } from "@/i18n/fill-node";
import { localePath, localizeText } from "@/i18n/locale";
import { breakEven } from "@/lib/benchmarks";
import { formatFactor, formatNumber, formatSize } from "@/lib/format";
import { getIndex, getLatestPart, getPart, getPartsByShelf } from "@/lib/parts";
import { JsonLd } from "@/components/json-ld";
import { absoluteUrl, pageMetadata } from "@/lib/seo";
import { CLI_PACKAGE, SITE } from "@/lib/site";
import type { Metadata } from "next";

export function homeMetadata(locale: Locale): Metadata {
  return pageMetadata({ locale, path: "/", description: getDictionary(locale).meta.description, image: "/og/site.png" });
}

export function HomeView({ locale }: { locale: Locale }) {
  const t = getDictionary(locale);
  const latest = getLatestPart();
  const featured = latest ? getPart(latest.name) : undefined;

  return (
    <div className="mx-auto max-w-screen-xl px-4 sm:px-6">
      <JsonLd
        data={{
          "@type": "WebSite",
          name: SITE.name,
          url: absoluteUrl(localePath(locale, "/")),
          description: t.meta.description,
          inLanguage: locale,
        }}
      />
      <section className="flex flex-col items-center gap-3 py-16 text-center sm:py-24">
        {featured && (
          <Link
            href={localePath(locale, `/parts/${featured.name}/`)}
            className="anim-enter mb-2 inline-flex items-center gap-1 rounded-full bg-secondary px-3 py-1 text-xs font-medium transition-colors hover:bg-secondary/80"
          >
            {fill(t.home.newPart, { title: localizeText(featured, locale).title })}
            <span aria-hidden="true">→</span>
          </Link>
        )}
        <h1 style={{ "--i": 1 } as CSSProperties} className="anim-enter max-w-3xl text-4xl leading-tight font-semibold tracking-tight text-balance sm:text-5xl sm:leading-[1.1]">
          {t.meta.tagline}
        </h1>
        <p style={{ "--i": 2 } as CSSProperties} className="anim-enter max-w-2xl text-base text-balance text-muted-foreground sm:text-lg">
          {t.meta.description}
        </p>
        <div style={{ "--i": 3 } as CSSProperties} className="anim-enter mt-4 flex flex-wrap justify-center gap-2">
          <Link href={localePath(locale, "/parts/")} className={buttonClass("primary", "sm", "rounded-full px-4")}>
            {t.home.browse}
          </Link>
          <a href={SITE.github} className={buttonClass("secondary", "sm", "rounded-full px-4")}>
            {t.home.github}
          </a>
        </div>
      </section>

      {featured && <Showcase part={featured} locale={locale} t={t} />}
    </div>
  );
}

/** 冒頭の部品1件のデータから組み立てるカードの一覧。手書きの数値は置かない */
function Showcase({ part, locale, t }: { part: Part; locale: Locale; t: Dictionary }) {
  const s = t.home.showcase;
  const v = part.verification;
  const be = breakEven(part.benchmarks);
  const summary = getIndex().parts.find((p) => p.name === part.name);
  const { title } = localizeText(part, locale);
  const href = localePath(locale, `/parts/${part.name}/`);
  const n = (x: number) => formatNumber(x, locale);

  return (
    <section className="grid gap-4 pb-24 md:grid-cols-2 lg:grid-cols-3">
      <div className="flex flex-col gap-4">
        <Card style={{ "--i": 4 } as CSSProperties} className="anim-enter">
          <CardHeader
            title={s.verification}
            description={fill(t.part.proof.matches, { name: part.reference.name })}
            action={part.sample && <SampleBadge t={t.sample} />}
          />
          <div className="space-y-4 p-5">
            <div className="flex items-baseline gap-2">
              <span className="text-4xl font-semibold tracking-tight tabular-nums">{n(v.passed)}</span>
              <span className="text-muted-foreground">/ {n(v.cases)} {t.part.proof.cases}</span>
            </div>
            <div className="h-2 overflow-hidden rounded-full bg-secondary" aria-hidden="true">
              <div className="anim-grow h-full rounded-full bg-primary" style={{ width: `${v.cases === 0 ? 0 : (v.passed / v.cases) * 100}%` }} />
            </div>
            <div className="flex flex-wrap gap-1.5">
              <Pill>{t.part.verification.methods[v.method]}</Pill>
              <Pill>
                {part.reference.language} · {part.reference.name} {part.reference.version}
              </Pill>
            </div>
          </div>
        </Card>

        <ShelvesCard locale={locale} t={t} className="anim-reveal" />
      </div>

      <div className="flex flex-col gap-4">
        <Card style={{ "--i": 5 } as CSSProperties} className="anim-enter">
          <CardHeader
            title={s.benchmarks}
            description={
              summary?.max_speedup !== undefined
                ? `${t.common.upTo} ${fill(t.common.fasterBy, { factor: formatFactor(summary.max_speedup) })}`
                : undefined
            }
            action={part.sample && <SampleBadge t={t.sample} />}
          />
          <div className="space-y-3 p-5">
            <Sparkline points={part.benchmarks} height={120} label={fill(t.part.sparkline, { name: part.reference.name })} />
            <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
              <span className="flex items-center gap-3">
                <LegendKey color="var(--series-rust)" label="kura-rs (Rust)" />
                <LegendKey color="var(--series-reference)" label={part.reference.name} dashed />
              </span>
              {be.kind === "crossover" && (
                <span>
                  {t.part.proof.breakEven} ≈ {formatSize(be.inputSize, locale)}
                </span>
              )}
            </div>
          </div>
        </Card>

        <Card className="anim-reveal">
          <CardHeader title={s.principles} />
          <ol className="space-y-4 p-5">
            {t.home.points.map((p, i) => (
              <li key={p.title} className="flex gap-3">
                <span className="flex size-6 shrink-0 items-center justify-center rounded-full bg-secondary font-mono text-xs font-medium">
                  {i + 1}
                </span>
                <div className="space-y-1">
                  <div className="text-sm font-medium">{p.title}</div>
                  <p className="text-sm text-muted-foreground">
                    {fillNode(p.body, { code: <code className="font-mono text-[13px] text-foreground">kura add &lt;name&gt;</code> })}
                  </p>
                </div>
              </li>
            ))}
          </ol>
        </Card>
      </div>

      <div className="flex flex-col gap-4 md:col-span-2 lg:col-span-1">
        <Card style={{ "--i": 6 } as CSSProperties} className="anim-enter">
          <CardHeader title={s.install} description={title} />
          <div className="p-5">
            <CodeBlock
              icon="terminal"
              labels={t.common}
              sources={[
                { id: "rust", label: "cargo", code: `cargo install ${CLI_PACKAGE}\nkura add ${part.name}`, lang: "bash" },
                ...(part.python ? [{ id: "python", label: "pip", code: `pip install ${part.python.package}`, lang: "bash" as const }] : []),
              ]}
            />
          </div>
        </Card>

        <Card className="anim-reveal">
          <CardHeader
            title={s.usage}
            action={
              <Link href={href} className={buttonClass("outline", "sm", "h-7 px-2.5 text-xs")}>
                {fill(s.open, { title })}
              </Link>
            }
          />
          <div className="p-5">
            <CodeBlock
              labels={t.common}
              sources={[
                { id: "rust", label: "Rust", code: part.usage.rust, lang: "rust" },
                ...(part.usage.python ? [{ id: "python", label: "Python", code: part.usage.python, lang: "python" as const }] : []),
              ]}
            />
          </div>
        </Card>
      </div>
    </section>
  );
}

function ShelvesCard({ locale, t, className = "" }: { locale: Locale; t: Dictionary; className?: string }) {
  const s = t.home.showcase;
  return (
    <Card className={className}>
      <CardHeader title={s.shelves} description={s.shelvesLead} />
      <ul className="space-y-1 p-3">
        {getPartsByShelf().map(({ shelf, parts }) => (
          <li key={shelf}>
            <Link
              href={localePath(locale, `/parts/#${shelf}`)}
              className="flex items-center justify-between gap-3 rounded-lg px-3 py-2.5 transition-colors hover:bg-accent"
            >
              <div className="min-w-0">
                <div className="text-sm font-medium">{t.shelves[shelf].label}</div>
                <div className="truncate text-xs text-muted-foreground">{t.shelves[shelf].description}</div>
              </div>
              <span className="shrink-0 text-xs text-muted-foreground tabular-nums">
                {parts.length === 1 ? s.partCount : fill(s.partsCount, { n: parts.length })}
              </span>
            </Link>
          </li>
        ))}
      </ul>
    </Card>
  );
}

function LegendKey({ color, label, dashed = false }: { color: string; label: string; dashed?: boolean }) {
  return (
    <span className="flex items-center gap-1.5">
      <svg width="14" height="6" aria-hidden="true">
        <line x1="0" y1="3" x2="14" y2="3" stroke={color} strokeWidth="2" strokeDasharray={dashed ? "4 3" : undefined} />
      </svg>
      <span className="text-foreground">{label}</span>
    </span>
  );
}
