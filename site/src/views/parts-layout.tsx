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
    <div className="mx-auto flex max-w-7xl gap-8 px-5 sm:px-8 lg:gap-10">
      <aside className="sticky top-14 hidden h-[calc(100dvh-3.5rem)] w-48 shrink-0 overflow-y-auto border-r py-8 pr-5 md:block">
        <Sidebar sections={sections} allPartsHref={localePath(locale, "/parts/")} t={t.sidebar} />
      </aside>
      <div className="min-w-0 flex-1">
        <details className="mt-5 rounded-lg border p-3 md:hidden">
          <summary className="cursor-pointer text-sm font-medium">{t.sidebar.allParts}</summary>
          <div className="max-h-72 overflow-y-auto pt-4">
            <Sidebar sections={sections} allPartsHref={localePath(locale, "/parts/")} t={t.sidebar} />
          </div>
        </details>
        {children}
      </div>
    </div>
  );
}
