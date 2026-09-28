import Link from "next/link";
import { SampleBadge } from "@/components/sample-badge";
import type { PartSummary } from "@/generated/PartSummary";
import { fill, type Dictionary, type Locale } from "@/i18n/dictionaries";
import { localePath, localizeText } from "@/i18n/locale";
import { formatFactor } from "@/lib/format";

export function PartCard({ part, locale, t }: { part: PartSummary; locale: Locale; t: Dictionary }) {
  const { title, description } = localizeText(part, locale);
  return (
    <Link
      href={localePath(locale, `/parts/${part.name}/`)}
      className="group flex min-w-0 flex-col rounded-lg border bg-card transition-colors hover:border-foreground/30 hover:bg-muted/40"
    >
      <div className="flex flex-1 flex-col p-5">
        <div className="flex items-start justify-between gap-3">
          <h3 className="text-sm font-semibold">{title}</h3>
          <span aria-hidden="true" className="text-muted-foreground transition-colors group-hover:text-foreground">↗</span>
        </div>
        {part.sample && <SampleBadge t={t.sample} className="mt-2 self-start" />}
        <p className="mt-1.5 flex-1 text-sm leading-6 text-muted-foreground">{description}</p>
        <dl className="mt-5 flex flex-wrap items-end justify-between gap-4 text-sm">
          <div>
            <dt className="text-xs text-muted-foreground">{t.card.verifiedAgainst}</dt>
            <dd className="font-medium">{part.reference}</dd>
          </div>
          <div>
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
