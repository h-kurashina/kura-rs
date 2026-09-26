import Link from "next/link";
import { SampleBadge } from "@/components/sample-badge";
import { Sparkline } from "@/components/sparkline";
import type { PartSummary } from "@/generated/PartSummary";
import { fill, type Dictionary, type Locale } from "@/i18n/dictionaries";
import { localePath, localizeText } from "@/i18n/locale";
import { formatFactor } from "@/lib/format";
import { getPart } from "@/lib/parts";

export function PartCard({ part, locale, t }: { part: PartSummary; locale: Locale; t: Dictionary }) {
  const { title, description } = localizeText(part, locale);
  const benchmarks = getPart(part.name)?.benchmarks ?? [];
  return (
    <Link
      href={localePath(locale, `/parts/${part.name}/`)}
      className="anim-reveal group flex flex-col overflow-hidden rounded-xl border bg-card shadow-xs transition-[translate,box-shadow,border-color] duration-200 hover:-translate-y-0.5 hover:border-foreground/20 hover:shadow-md motion-reduce:hover:translate-y-0"
    >
      <div className="relative border-b bg-code px-5 pt-6 pb-4">
        <Sparkline points={benchmarks} height={72} label={fill(t.part.sparkline, { name: part.reference })} />
        {part.sample && <SampleBadge t={t.sample} className="absolute top-3 right-3 bg-card" />}
      </div>
      <div className="flex flex-1 flex-col p-5">
        <h3 className="font-semibold tracking-tight">{title}</h3>
        <p className="mt-1.5 line-clamp-3 flex-1 text-sm text-muted-foreground">{description}</p>
        <dl className="mt-5 flex items-end justify-between gap-4 text-sm">
          <div>
            <dt className="text-xs text-muted-foreground">{t.card.verifiedAgainst}</dt>
            <dd className="font-medium">{part.reference}</dd>
          </div>
          <div className="text-right">
            <dt className="text-xs text-muted-foreground">{t.common.upTo}</dt>
            <dd className="font-medium tabular-nums">
              {part.max_speedup === undefined ? "–" : fill(t.common.fasterBy, { factor: formatFactor(part.max_speedup) })}
            </dd>
          </div>
        </dl>
      </div>
    </Link>
  );
}
