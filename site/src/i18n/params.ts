import { notFound } from "next/navigation";
import { isLocale, type Locale } from "@/i18n/dictionaries";

/** URL の [locale] を言語コードとして受け取る。知らない言語なら 404 */
export async function localeFromParams(params: Promise<{ locale: string }>): Promise<Locale> {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  return locale;
}
