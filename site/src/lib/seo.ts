/**
 * 検索エンジンと SNS 向けのメタデータ。URL はすべて絶対 URL で出す。
 *
 * GitHub Pages のサブパスに置く場合は NEXT_PUBLIC_BASE_PATH も合わせて設定する。
 */
import type { Metadata } from "next";
import { DEFAULT_LOCALE, getDictionary, LOCALE_INFO, LOCALES, type Locale } from "@/i18n/dictionaries";
import { localePath } from "@/i18n/locale";
import { SITE } from "@/lib/site";

/**
 * 公開先の URL。優先順は
 * 1. NEXT_PUBLIC_SITE_URL（独自ドメインなど、明示したいとき）
 * 2. Vercel がビルド時に渡す本番ドメイン（VERCEL_PROJECT_PRODUCTION_URL）
 * 3. GitHub Pages
 */
export const SITE_URL = (
  process.env.NEXT_PUBLIC_SITE_URL ??
  (process.env.VERCEL_PROJECT_PRODUCTION_URL ? `https://${process.env.VERCEL_PROJECT_PRODUCTION_URL}` : "https://h-kurashina.github.io/kura")
).replace(/\/$/, "");

export function absoluteUrl(path: string): string {
  return `${SITE_URL}${path}`;
}

/** 言語ごとの同じページへの URL（hreflang 用） */
export function languageAlternates(path: string): Record<string, string> {
  const entries: [string, string][] = LOCALES.map((l) => [LOCALE_INFO[l].tag, absoluteUrl(localePath(l, path))]);
  entries.push(["x-default", absoluteUrl(localePath(DEFAULT_LOCALE, path))]);
  return Object.fromEntries(entries);
}

/**
 * ページごとのメタデータ。
 * path は言語の接頭辞を含まない形（例: "/parts/minhash/"）、image は OGP 画像の絶対パス。
 */
export function pageMetadata({
  locale,
  path,
  title,
  description,
  image,
}: {
  locale: Locale;
  path: string;
  title?: string;
  description: string;
  image: string;
}): Metadata {
  const t = getDictionary(locale);
  const url = absoluteUrl(localePath(locale, path));
  const fullTitle = title ? `${title} · ${SITE.name}` : `${SITE.name} · ${t.meta.tagline}`;
  const images = [{ url: absoluteUrl(image), width: 1200, height: 630, alt: fullTitle }];
  return {
    ...(title ? { title } : {}),
    description,
    alternates: { canonical: url, languages: languageAlternates(path) },
    openGraph: {
      type: "website",
      siteName: SITE.name,
      title: fullTitle,
      description,
      url,
      locale: LOCALE_INFO[locale].og,
      alternateLocale: LOCALES.filter((l) => l !== locale).map((l) => LOCALE_INFO[l].og),
      images,
    },
    twitter: { card: "summary_large_image", title: fullTitle, description, images: images.map((i) => i.url) },
  };
}
