import type { Metadata } from "next";
import { LogoMark } from "@/components/logo";
import { getDictionary, LOCALE_INFO, LOCALES } from "@/i18n/dictionaries";
import { localePath } from "@/i18n/locale";
import { fontVariables } from "@/lib/fonts";
import "./globals.css";

export const metadata: Metadata = { title: "404 · kura-rs", robots: { index: false } };

const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";

/** どのルートにも一致しない URL 用。言語が分からないので英語で出し、各言語の部品一覧へのリンクを並べる */
export default function GlobalNotFound() {
  const en = getDictionary("en").notFound;
  return (
    <html lang="en" className={fontVariables}>
      <body className="min-h-dvh font-sans">
        <main className="mx-auto flex min-h-dvh max-w-md flex-col justify-center gap-8 px-4">
          <LogoMark className="size-8" />
          <div className="space-y-2">
            <p className="font-mono text-sm text-muted-foreground">404</p>
            <h1 className="text-2xl font-semibold tracking-tight">{en.title}</h1>
            <p className="text-muted-foreground">{en.body}</p>
          </div>
          <ul className="grid grid-cols-2 gap-2 border-t pt-6 text-sm">
            {LOCALES.map((code) => (
              <li key={code} lang={LOCALE_INFO[code].tag}>
                <a href={`${base}${localePath(code, "/parts/")}`} className="block rounded-lg px-2 py-1.5 hover:bg-accent">
                  <span className="font-medium">{LOCALE_INFO[code].label}</span>
                  <span className="block text-xs text-muted-foreground">{getDictionary(code).notFound.back}</span>
                </a>
              </li>
            ))}
          </ul>
        </main>
      </body>
    </html>
  );
}
