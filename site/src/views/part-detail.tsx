import type { Metadata } from "next";
import Link from "next/link";
import type { CSSProperties } from "react";
import { notFound } from "next/navigation";
import { BenchmarkChart } from "@/components/benchmark-chart";
import { CodeBlock } from "@/components/code-block";
import { SampleBadge, SampleNotice } from "@/components/sample-badge";
import { Section, Stat } from "@/components/section";
import { Sparkline } from "@/components/sparkline";
import { buttonClass, Card } from "@/components/ui";
import type { Part } from "@/generated/Part";
import { fill, getDictionary, LOCALE_INFO, type Dictionary, type Locale } from "@/i18n/dictionaries";
import { localePath, localizeText } from "@/i18n/locale";
import { breakEven, type BreakEven } from "@/lib/benchmarks";
import { formatDate, formatFactor, formatMs, formatNumber, formatSize } from "@/lib/format";
import { getIndex, getPart } from "@/lib/parts";
import { JsonLd } from "@/components/json-ld";
import { absoluteUrl, pageMetadata } from "@/lib/seo";
import { CLI_PACKAGE, SITE } from "@/lib/site";
import { fillNode } from "@/i18n/fill-node";

export interface PartParams {
  params: Promise<{ name: string }>;
}

export function partStaticParams(): { name: string }[] {
  return getIndex().parts.map((p) => ({ name: p.name }));
}

export async function partMetadata({ params }: PartParams, locale: Locale): Promise<Metadata> {
  const part = getPart((await params).name);
  if (!part) return {};
  const { title, description } = localizeText(part, locale);
  return pageMetadata({ locale, path: `/parts/${part.name}/`, title, description, image: `/og/parts/${part.name}.png` });
}

const TOC_IDS = ["installation", "usage", "verification", "benchmarks", "source"] as const;

function breakEvenText(be: BreakEven, t: Dictionary, locale: Locale): string {
  switch (be.kind) {
    case "always-faster":
      return t.part.benchmarks.alwaysFaster;
    case "never-faster":
      return t.part.benchmarks.neverFaster;
    case "crossover":
      return fill(t.part.benchmarks.crossover, { size: formatSize(be.inputSize, locale) });
  }
}

export async function PartDetailView({ params, locale }: PartParams & { locale: Locale }) {
  const { name } = await params;
  const part = getPart(name);
  if (!part) notFound();

  const t = getDictionary(locale);
  const n = (x: number) => formatNumber(x, locale);
  const copy = t.common;
  const summary = getIndex().parts.find((p) => p.name === name);
  const be = breakEven(part.benchmarks);
  const { verification: v, reference } = part;
  const { title, description } = localizeText(part, locale);
  const method = t.part.verification.methods[v.method];
  const all = getIndex().parts;
  const pos = all.findIndex((p) => p.name === name);
  const prev = pos > 0 ? all[pos - 1] : undefined;
  const next = pos >= 0 && pos < all.length - 1 ? all[pos + 1] : undefined;

  return (
    <div className="flex gap-8 py-8 lg:py-12">
      <JsonLd
        data={{
          "@type": "SoftwareSourceCode",
          name: title,
          description,
          url: absoluteUrl(localePath(locale, `/parts/${part.name}/`)),
          codeRepository: SITE.github,
          programmingLanguage: "Rust",
          version: part.version,
          inLanguage: locale,
          keywords: [...part.shelves, part.name, part.reference.name].join(", "),
        }}
      />
      <article className="min-w-0 max-w-3xl flex-1 space-y-10">
        <header className="space-y-5">
          <nav aria-label="Breadcrumb" className="flex items-center gap-1.5 text-sm text-muted-foreground">
            <Link href={localePath(locale, "/parts/")} className="hover:text-foreground">
              {t.part.breadcrumb}
            </Link>
            <span aria-hidden="true">/</span>
            <span>{part.shelves.map((s) => t.shelves[s].label).join(", ")}</span>
          </nav>
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div className="space-y-2">
              <div className="flex flex-wrap items-center gap-2.5">
                <h1 className="text-3xl font-semibold tracking-tight">{title}</h1>
                <span className="rounded-full bg-secondary px-2 py-0.5 font-mono text-xs text-muted-foreground">v{part.version}</span>
                <span className="text-xs text-muted-foreground">
                  {fill(t.part.added, { date: formatDate(`${part.added}T00:00:00Z`, locale) })}
                </span>
                {part.sample && <SampleBadge t={t.sample} />}
              </div>
              <p className="text-sm leading-7 text-muted-foreground">{description}</p>
            </div>
            <div className="flex shrink-0 gap-1.5">
              <PagerLink href={prev && localePath(locale, `/parts/${prev.name}/`)} label={t.part.previous} dir="prev" />
              <PagerLink href={next && localePath(locale, `/parts/${next.name}/`)} label={t.part.next} dir="next" />
            </div>
          </div>

          <ProofStrip part={part} maxSpeedup={summary?.max_speedup} breakEven={be} locale={locale} t={t} />

          {reference.url && (
            <div className="text-sm">
              <a href={reference.url} className="text-muted-foreground underline-offset-4 hover:text-foreground hover:underline">
                {fill(t.part.reference, { name: reference.name })} ↗
              </a>
            </div>
          )}
        </header>

        <Section id="installation" title={t.part.toc.installation}>
          <CodeBlock
            icon="terminal"
            labels={copy}
            sources={[
              { id: "rust", label: "cargo", code: `cargo install ${CLI_PACKAGE}\nkura add ${part.name}`, lang: "bash" },
              ...(part.python ? [{ id: "python", label: "pip", code: `pip install ${part.python.package}`, lang: "bash" as const }] : []),
            ]}
          />
          <div className="space-y-1 text-sm text-muted-foreground">
            <p>{part.rust.files.length === 1 ? t.part.install.copiesOne : fill(t.part.install.copiesMany, { n: part.rust.files.length })}</p>
            <p>
              {fillNode(t.part.install.conflict, {
                alt: <code className="font-mono text-[13px] text-foreground">cargo install {CLI_PACKAGE} --bin {CLI_PACKAGE}</code>,
                kura: <code className="font-mono text-[13px] text-foreground">kura</code>,
                addAlt: <code className="font-mono text-[13px] text-foreground">{CLI_PACKAGE} add</code>,
              })}
            </p>
          </div>
        </Section>

        <Section id="usage" title={t.part.toc.usage}>
          <CodeBlock
            labels={copy}
            sources={[
              { id: "rust", label: "Rust", code: part.usage.rust, lang: "rust" },
              ...(part.usage.python ? [{ id: "python", label: "Python", code: part.usage.python, lang: "python" as const }] : []),
            ]}
          />
        </Section>

        <Section id="verification" title={t.part.toc.verification} aside={part.sample && <SampleBadge t={t.sample} />}>
          <p className="text-muted-foreground">
            {fill(t.part.verification.lead, {
              name: reference.name,
              version: reference.version,
              language: reference.language,
              method: locale === "en" ? method.toLowerCase() : method,
            })}
          </p>
          <div className="grid gap-3 sm:grid-cols-2">
            <Stat label={t.part.verification.reference} value={reference.name} detail={`${reference.language} ${reference.version}`} />
            <Stat label={t.part.verification.cases} value={n(v.cases)} detail={method} />
            <Stat
              label={t.part.verification.passed}
              value={`${n(v.passed)} / ${n(v.cases)}`}
              detail={
                v.passed === v.cases
                  ? t.part.verification.allMatch
                  : fill(t.part.verification.mismatched, { n: n(v.cases - v.passed) })
              }
            />
            <Stat label={t.part.verification.lastRun} value={formatDate(v.last_run, locale)} detail="UTC" />
          </div>
          {part.sample && <SampleNotice t={t.sample} />}
        </Section>

        <Section id="benchmarks" title={t.part.toc.benchmarks} aside={part.sample && <SampleBadge t={t.sample} />}>
          <p className="text-muted-foreground">
            {fill(t.part.benchmarks.lead, { name: reference.name })} {breakEvenText(be, t, locale)}
            {summary?.max_speedup !== undefined && fill(t.part.benchmarks.upTo, { factor: formatFactor(summary.max_speedup) })}
            {LOCALE_INFO[locale].fullStop}
          </p>
          {part.sample && <SampleNotice t={t.sample} />}
          <BenchmarkChart
            points={part.benchmarks}
            breakEven={be}
            referenceName={reference.name}
            locale={locale}
            t={{ chart: t.chart, common: t.common }}
          />
          <BenchmarkTable part={part} locale={locale} t={t} />
          {(part.environment || part.input_unit) && (
            <div className="space-y-1 text-xs text-muted-foreground">
              {part.input_unit && <p>{fill(t.part.benchmarks.inputUnit, { unit: part.input_unit })}</p>}
              {part.environment && (
                <p>
                  {fill(t.part.benchmarks.environment, {
                    cpu: part.environment.cpu,
                    os: part.environment.os,
                    rust: part.environment.rust,
                    runtime: part.environment.reference_runtime,
                  })}
                </p>
              )}
            </div>
          )}
        </Section>

        <Section id="source" title={t.part.toc.source}>
          <div className="grid gap-4 sm:grid-cols-2">
            <Card className="overflow-hidden">
              <h3 className="border-b bg-code-header px-4 py-2.5 text-sm font-medium">{t.part.source.files}</h3>
              <ul className="divide-y">
                {part.rust.files.map((f) => (
                  <li key={f} className="px-4 py-2.5 font-mono text-[13px] text-muted-foreground">
                    {f}
                  </li>
                ))}
              </ul>
            </Card>
            <Card className="overflow-hidden">
              <h3 className="border-b bg-code-header px-4 py-2.5 text-sm font-medium">{t.part.source.dependencies}</h3>
              {part.rust.dependencies.length === 0 ? (
                <p className="px-4 py-2.5 text-sm text-muted-foreground">{t.part.source.none}</p>
              ) : (
                <ul className="divide-y">
                  {part.rust.dependencies.map((d) => (
                    <li key={d.name} className="flex flex-wrap items-baseline gap-x-2 px-4 py-2.5 font-mono text-[13px]">
                      <a href={`https://crates.io/crates/${d.name}`} className="font-medium hover:underline hover:underline-offset-4">
                        {d.name}
                      </a>
                      <span className="text-muted-foreground">{d.version}</span>
                      {d.features.length > 0 && (
                        <span className="text-muted-foreground">{fill(t.part.source.features, { list: d.features.join(", ") })}</span>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </Card>
          </div>
        </Section>
      </article>

      <aside className="sticky top-14 hidden h-[calc(100dvh-3.5rem)] w-40 shrink-0 flex-col justify-between py-2 pb-8 xl:flex">
        <div>
          <h4 className="mb-2 text-xs font-medium text-muted-foreground">{t.part.onThisPage}</h4>
          <ul className="space-y-2 text-[13px]">
            {TOC_IDS.map((id) => (
              <li key={id}>
                <a href={`#${id}`} className="text-muted-foreground transition-colors hover:text-foreground">
                  {t.part.toc[id]}
                </a>
              </li>
            ))}
          </ul>
        </div>
        <Card className="space-y-2 bg-secondary/60 p-4 shadow-none dark:bg-card">
          <div className="text-sm font-semibold">{t.part.promo.title}</div>
          <p className="text-[13px] text-muted-foreground">{t.part.promo.body}</p>
          <a href={SITE.github} className={buttonClass("outline", "sm", "mt-1 h-7 px-2.5 text-xs")}>
            {t.part.promo.action}
          </a>
        </Card>
      </aside>
    </div>
  );
}

/** 冒頭に置く「検証の証拠」の要約。詳細は各セクションへリンクする */
function ProofStrip({
  part,
  maxSpeedup,
  breakEven: be,
  locale,
  t,
}: {
  part: Part;
  maxSpeedup: number | undefined;
  breakEven: BreakEven;
  locale: Locale;
  t: Dictionary;
}) {
  const v = part.verification;
  const n = (x: number) => formatNumber(x, locale);
  return (
    <Card style={{ "--i": 1 } as CSSProperties} className="anim-enter overflow-hidden">
      <div className="grid sm:grid-cols-3 sm:divide-x">
        <ProofItem
          href="#verification"
          label={fill(t.part.proof.matches, { name: part.reference.name })}
          value={`${n(v.passed)} / ${n(v.cases)}`}
          unit={t.part.proof.cases}
        />
        <ProofItem
          href="#benchmarks"
          label={t.common.upTo}
          value={maxSpeedup === undefined ? "–" : formatFactor(maxSpeedup)}
          unit={t.common.faster}
        />
        <ProofItem
          href="#benchmarks"
          label={t.part.proof.breakEven}
          value={
            be.kind === "crossover"
              ? `≈ ${formatSize(be.inputSize, locale)}`
              : be.kind === "always-faster"
                ? t.part.proof.always
                : t.part.proof.never
          }
          unit={be.kind === "crossover" ? t.part.proof.inputs : t.common.faster}
        />
      </div>
      <a href="#benchmarks" className="block border-t bg-code px-5 pt-5 pb-3 transition-colors hover:bg-accent/40">
        <Sparkline points={part.benchmarks} height={96} label={fill(t.part.sparkline, { name: part.reference.name })} />
      </a>
      {part.sample && (
        <p className="flex items-center gap-2 border-t border-dashed px-5 py-2.5 text-xs text-muted-foreground">
          <span className="size-1.5 shrink-0 rounded-full bg-muted-foreground/70" aria-hidden="true" />
          {t.sample.strip}
        </p>
      )}
    </Card>
  );
}

function ProofItem({ href, label, value, unit }: { href: string; label: string; value: string; unit: string }) {
  return (
    <a href={href} className="block border-b px-5 py-4 transition-colors last:border-b-0 hover:bg-accent/40 sm:border-b-0">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 flex items-baseline gap-1.5">
        <span className="text-2xl font-semibold tracking-tight tabular-nums">{value}</span>
        <span className="text-sm text-muted-foreground">{unit}</span>
      </div>
    </a>
  );
}

function BenchmarkTable({ part, locale, t }: { part: Part; locale: Locale; t: Dictionary }) {
  return (
    <details className="group overflow-hidden rounded-lg border bg-card">
      <summary className="cursor-pointer px-4 py-2.5 text-sm text-muted-foreground select-none hover:text-foreground">
        {t.part.benchmarks.showTable}
      </summary>
      <div className="overflow-x-auto border-t">
        <table className="w-full text-sm tabular-nums">
          <thead className="text-left text-xs text-muted-foreground">
            <tr>
              <th className="px-4 py-2 font-medium">{t.part.benchmarks.inputSize}</th>
              <th className="px-4 py-2 text-right font-medium">kura-rs (Rust)</th>
              <th className="px-4 py-2 text-right font-medium">{part.reference.name}</th>
              <th className="px-4 py-2 text-right font-medium">{t.part.benchmarks.speedup}</th>
            </tr>
          </thead>
          <tbody>
            {part.benchmarks.map((p) => {
              const f = p.reference_ms / p.rust_ms;
              return (
                <tr key={p.input_size} className="border-t">
                  <td className="px-4 py-2">{formatNumber(p.input_size, locale)}</td>
                  <td className="px-4 py-2 text-right">{formatMs(p.rust_ms, locale)}</td>
                  <td className="px-4 py-2 text-right">{formatMs(p.reference_ms, locale)}</td>
                  <td className="px-4 py-2 text-right">
                    {f >= 1 ? (
                      formatFactor(f)
                    ) : (
                      <span className="text-muted-foreground">{fill(t.common.slowerBy, { factor: formatFactor(1 / f) })}</span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </details>
  );
}

function PagerLink({ href, label, dir }: { href: string | undefined; label: string; dir: "prev" | "next" }) {
  const icon = (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={dir === "prev" ? "m15 18-6-6 6-6" : "m9 18 6-6-6-6"} />
    </svg>
  );
  const cls = buttonClass("outline", "sm", "size-8 px-0");
  return href ? (
    <Link href={href} aria-label={label} title={label} className={cls}>
      {icon}
    </Link>
  ) : (
    <span aria-hidden="true" className={`${cls} opacity-40`}>
      {icon}
    </span>
  );
}
