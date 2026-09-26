import type { Metadata } from "next";
import type { ReactNode } from "react";
import { PREFIXED_LOCALES } from "@/i18n/dictionaries";
import { localeFromParams } from "@/i18n/params";
import { RootShell, rootMetadata } from "@/views/root-shell";
import "../../globals.css";

/**
 * 英語以外の言語（/ja/, /zh/ ...）のルートレイアウト。英語は app/(en) にある。
 * 言語は i18n/dictionaries.ts の LOCALE_INFO から自動で増える。
 */
export const dynamicParams = false;

export function generateStaticParams(): { locale: string }[] {
  return PREFIXED_LOCALES.map((locale) => ({ locale }));
}

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }): Promise<Metadata> {
  return rootMetadata(await localeFromParams(params));
}

export default async function Layout({ children, params }: { children: ReactNode; params: Promise<{ locale: string }> }) {
  return <RootShell locale={await localeFromParams(params)}>{children}</RootShell>;
}
