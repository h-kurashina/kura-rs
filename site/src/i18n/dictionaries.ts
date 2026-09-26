/**
 * 画面の文言の入口。言語を足すときは locales/<code>.ts を作り、ここの import と LOCALE_INFO に1行ずつ足す。
 * 部品の説明の訳は Rust 側のスキーマ（Translations）に同じコードの欄を足す。
 */
import { de } from "@/i18n/locales/de";
import { en } from "@/i18n/locales/en";
import { es } from "@/i18n/locales/es";
import { fr } from "@/i18n/locales/fr";
import { ja } from "@/i18n/locales/ja";
import { ko } from "@/i18n/locales/ko";
import { pt } from "@/i18n/locales/pt";
import { zh } from "@/i18n/locales/zh";

export type Dictionary = typeof en;

export interface LocaleInfo {
  dictionary: Dictionary;
  /** 言語の名前（その言語で書く。言語メニューに出す） */
  label: string;
  /** <html lang> と hreflang に使う言語タグ */
  tag: string;
  /** Intl（数値・日付の書式）に渡す言語タグ */
  intl: string;
  /** OGP の og:locale */
  og: string;
  /** 文末の句点 */
  fullStop: string;
}

/** 並び順がそのまま言語メニューの順になる。先頭が既定言語（URL に接頭辞なし） */
export const LOCALE_INFO = {
  en: { dictionary: en, label: "English", tag: "en", intl: "en-US", og: "en_US", fullStop: "." },
  ja: { dictionary: ja, label: "日本語", tag: "ja", intl: "ja-JP", og: "ja_JP", fullStop: "。" },
  zh: { dictionary: zh, label: "简体中文", tag: "zh-Hans", intl: "zh-CN", og: "zh_CN", fullStop: "。" },
  ko: { dictionary: ko, label: "한국어", tag: "ko", intl: "ko-KR", og: "ko_KR", fullStop: "." },
  es: { dictionary: es, label: "Español", tag: "es", intl: "es-ES", og: "es_ES", fullStop: "." },
  fr: { dictionary: fr, label: "Français", tag: "fr", intl: "fr-FR", og: "fr_FR", fullStop: "." },
  de: { dictionary: de, label: "Deutsch", tag: "de", intl: "de-DE", og: "de_DE", fullStop: "." },
  pt: { dictionary: pt, label: "Português (Brasil)", tag: "pt-BR", intl: "pt-BR", og: "pt_BR", fullStop: "." },
} as const satisfies Record<string, LocaleInfo>;

export type Locale = keyof typeof LOCALE_INFO;
export const LOCALES = Object.keys(LOCALE_INFO) as Locale[];
export const DEFAULT_LOCALE = "en" satisfies Locale;
/** URL に /<code>/ の接頭辞が付く言語 */
export const PREFIXED_LOCALES = LOCALES.filter((l) => l !== DEFAULT_LOCALE);

export function isLocale(value: string): value is Locale {
  return value in LOCALE_INFO;
}

export function getDictionary(locale: Locale): Dictionary {
  return LOCALE_INFO[locale].dictionary;
}

/** "{name}" のような置き換え部分を埋める */
export function fill(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (m, key: string) => (key in values ? String(values[key]) : m));
}
