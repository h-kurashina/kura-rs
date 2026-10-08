import type { Metadata } from "next";
import { GoogleAnalytics } from "@next/third-parties/google";
import type { ReactNode } from "react";
import { fontVariables } from "@/lib/fonts";
import { CommandMenu, type CommandGroupData } from "@/components/command-menu";
import { SiteHeader } from "@/components/site-header";
import { getDictionary, LOCALE_INFO, type Locale } from "@/i18n/dictionaries";
import { localePath, localizeText } from "@/i18n/locale";
import { getPartsByShelf } from "@/lib/parts";
import { SITE_URL } from "@/lib/seo";
import { SITE } from "@/lib/site";

export function rootMetadata(locale: Locale): Metadata {
  const t = getDictionary(locale);
  return {
    metadataBase: new URL(`${SITE_URL}/`),
    title: { default: `${SITE.name} · ${t.meta.tagline}`, template: `%s · ${SITE.name}` },
    description: t.meta.description,
    applicationName: SITE.name,
    robots: { index: true, follow: true },
  };
}

/** 言語ごとのルートレイアウトの中身。<html lang> を言語に合わせる */
export function RootShell({ locale, children }: { locale: Locale; children: ReactNode }) {
  const t = getDictionary(locale);
  const groups: CommandGroupData[] = getPartsByShelf().map(({ shelf, parts }) => ({
    label: t.shelves[shelf].label,
    items: parts.map((p) => ({
      name: p.name,
      ...localizeText(p, locale),
      reference: p.reference,
      href: localePath(locale, `/parts/${p.name}/`),
    })),
  }));

  return (
    <html lang={LOCALE_INFO[locale].tag} className={fontVariables}>
      <body className="min-h-dvh font-sans">
        <SiteHeader
          locale={locale}
          t={t}
          search={
            <CommandMenu
              groups={groups}
              allPartsHref={localePath(locale, "/parts/")}
              t={{ ...t.search, allParts: t.sidebar.allParts }}
            />
          }
        />
        <main>{children}</main>
      </body>
      <GoogleAnalytics gaId={SITE.gaId} />
    </html>
  );
}
