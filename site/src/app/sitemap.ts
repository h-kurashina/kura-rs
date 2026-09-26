import type { MetadataRoute } from "next";
import { LOCALES } from "@/i18n/dictionaries";
import { localePath } from "@/i18n/locale";
import { getIndex } from "@/lib/parts";
import { absoluteUrl, languageAlternates } from "@/lib/seo";

export const dynamic = "force-static";

/** 全ページを言語ごとに並べ、互いを hreflang で結ぶ */
export default function sitemap(): MetadataRoute.Sitemap {
  const paths = ["/", "/parts/", ...getIndex().parts.map((p) => `/parts/${p.name}/`)];
  return paths.flatMap((path) =>
    LOCALES.map((locale) => ({
      url: absoluteUrl(localePath(locale, path)),
      changeFrequency: "weekly" as const,
      priority: path === "/" ? 1 : path === "/parts/" ? 0.8 : 0.7,
      alternates: { languages: languageAlternates(path) },
    })),
  );
}
