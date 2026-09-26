import type { ReactNode } from "react";
import { Sidebar, type SidebarSection } from "@/components/sidebar";
import { getDictionary, type Locale } from "@/i18n/dictionaries";
import { localePath, localizeText } from "@/i18n/locale";
import { getPartsByShelf } from "@/lib/parts";

export function PartsLayout({ locale, children }: { locale: Locale; children: ReactNode }) {
  const t = getDictionary(locale);
  const sections: SidebarSection[] = getPartsByShelf().map(({ shelf, parts }) => ({
    label: t.shelves[shelf].label,
    items: parts.map((p) => ({
      name: p.name,
      title: localizeText(p, locale).title,
      href: localePath(locale, `/parts/${p.name}/`),
    })),
  }));

  return (
    <div className="mx-auto flex max-w-screen-2xl gap-10 px-4 sm:px-6">
      <aside className="sticky top-14 hidden h-[calc(100dvh-3.5rem)] w-56 shrink-0 overflow-y-auto py-8 pr-2 md:block">
        <Sidebar sections={sections} allPartsHref={localePath(locale, "/parts/")} t={t.sidebar} />
      </aside>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
