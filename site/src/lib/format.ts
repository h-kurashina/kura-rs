import type { Locale } from "@/i18n/dictionaries";
import { intlLocale } from "@/i18n/locale";

export function formatSize(n: number, locale: Locale): string {
  const tag = intlLocale(locale);
  return n >= 10_000
    ? new Intl.NumberFormat(tag, { notation: "compact", maximumFractionDigits: 1 }).format(n)
    : new Intl.NumberFormat(tag).format(Math.round(n));
}

export function formatNumber(n: number, locale: Locale): string {
  return n.toLocaleString(intlLocale(locale));
}

export function formatMs(ms: number, locale: Locale): string {
  const tag = intlLocale(locale);
  if (ms >= 1000) return `${(ms / 1000).toLocaleString(tag, { maximumFractionDigits: 2 })} s`;
  if (ms >= 10) return `${ms.toLocaleString(tag, { maximumFractionDigits: 0 })} ms`;
  return `${ms.toLocaleString(tag, { maximumFractionDigits: 2 })} ms`;
}

export function formatFactor(f: number): string {
  return `${f >= 10 ? f.toFixed(0) : f.toFixed(1)}×`;
}

export function formatDate(iso: string, locale: Locale): string {
  return new Date(iso).toLocaleDateString(intlLocale(locale), {
    year: "numeric",
    month: "short",
    day: "numeric",
    timeZone: "UTC",
  });
}
