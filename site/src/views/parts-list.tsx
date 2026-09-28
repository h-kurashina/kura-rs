import { PartCard } from "@/components/part-card";
import { getDictionary, type Locale } from "@/i18n/dictionaries";
import type { Metadata } from "next";
import { getPartsByShelf } from "@/lib/parts";
import { pageMetadata } from "@/lib/seo";

export function partsListMetadata(locale: Locale): Metadata {
  const t = getDictionary(locale);
  return pageMetadata({ locale, path: "/parts/", title: t.list.title, description: t.list.lead, image: "/og/site.png" });
}

export function PartsListView({ locale }: { locale: Locale }) {
  const t = getDictionary(locale);
  return (
    <div className="max-w-4xl space-y-12 py-8 lg:py-12">
      <header className="space-y-3 border-b pb-8">
        <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">{t.list.title}</h1>
        <p className="max-w-2xl text-sm leading-7 text-muted-foreground">{t.list.lead}</p>
      </header>
      {getPartsByShelf().map(({ shelf, parts }) => (
        <section key={shelf} id={shelf} className="scroll-mt-20 space-y-4">
          <div className="space-y-1">
            <h2 className="text-lg font-semibold tracking-tight">{t.shelves[shelf].label}</h2>
            <p className="text-sm text-muted-foreground">{t.shelves[shelf].description}</p>
          </div>
          {parts.length === 0 ? (
            <p className="rounded-lg border border-dashed px-5 py-6 text-sm text-muted-foreground">{t.list.empty}</p>
          ) : (
            <div className="grid gap-3 sm:grid-cols-2">
              {parts.map((p) => (
                <PartCard key={p.name} part={p} locale={locale} t={t} />
              ))}
            </div>
          )}
        </section>
      ))}
    </div>
  );
}
