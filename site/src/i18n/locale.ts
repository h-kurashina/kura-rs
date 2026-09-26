import type { PartTranslation } from "@/generated/PartTranslation";
import type { Translations } from "@/generated/Translations";
import { DEFAULT_LOCALE, LOCALE_INFO, PREFIXED_LOCALES, type Locale } from "@/i18n/dictionaries";

// 既定以外の言語コードが、Rust 側の Translations の欄とそろっていることを型で確かめる
type MissingInSchema = Exclude<Exclude<Locale, typeof DEFAULT_LOCALE>, keyof Translations>;
const schemaCoversLocales: MissingInSchema extends never ? true : never = true;
void schemaCoversLocales;

/** 既定言語（英語）は接頭辞なし、それ以外は /ja/... のように前に付ける */
export function localePath(locale: Locale, path: string): string {
  return locale === DEFAULT_LOCALE ? path : `/${locale}${path}`;
}

const PREFIX = new RegExp(`^/(${PREFIXED_LOCALES.join("|")})(?=/|$)`);

/** 表示中のパスを、別の言語の同じページのパスに変える */
export function switchLocalePath(pathname: string, to: Locale): string {
  const bare = pathname.replace(PREFIX, "") || "/";
  return localePath(to, bare);
}

/** Intl に渡す言語タグ */
export function intlLocale(locale: Locale): string {
  return LOCALE_INFO[locale].intl;
}

/** 部品のタイトルと説明を、訳があればその言語で返す（なければ英語） */
export function localizeText(
  part: { title: string; description: string; translations?: Translations },
  locale: Locale,
): { title: string; description: string } {
  const t: PartTranslation | undefined = locale === DEFAULT_LOCALE ? undefined : part.translations?.[locale];
  return { title: t?.title ?? part.title, description: t?.description ?? part.description };
}
