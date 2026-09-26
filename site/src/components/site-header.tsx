import Link from "next/link";
import type { ReactNode } from "react";
import { LanguageMenu } from "@/components/language-menu";
import { LogoMark } from "@/components/logo";
import { LOCALE_INFO, LOCALES, type Dictionary, type Locale } from "@/i18n/dictionaries";
import { localePath } from "@/i18n/locale";
import { SITE } from "@/lib/site";

export function SiteHeader({ locale, t, search }: { locale: Locale; t: Dictionary; search?: ReactNode }) {
  return (
    <header className="sticky top-0 z-40 w-full bg-background/80 backdrop-blur-lg supports-[backdrop-filter]:bg-background/60">
      <div className="mx-auto flex h-14 max-w-screen-2xl items-center gap-2 px-4 sm:px-6">
        <Link
          href={localePath(locale, "/")}
          className="group/logo mr-3 inline-flex h-8 items-center gap-2 rounded-lg px-1.5 font-mono text-[15px] font-semibold tracking-tight"
        >
          <LogoMark className="size-6" />
          kura-rs
        </Link>
        <nav className="flex items-center gap-0.5 text-sm font-medium">
          <HeaderLink href={localePath(locale, "/parts/")}>{t.nav.parts}</HeaderLink>
          <HeaderLink href={localePath(locale, "/parts/#ai")}>{t.shelves.ai.label}</HeaderLink>
          <HeaderLink href={localePath(locale, "/parts/#security")} className="hidden sm:inline-flex">
            {t.shelves.security.label}
          </HeaderLink>
        </nav>
        <div className="ml-auto flex items-center gap-1.5">
          {search}
          <a
            href={SITE.github}
            aria-label={t.nav.github}
            className="inline-flex size-8 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
              <path d="M12 .5a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2c-3.3.7-4-1.6-4-1.6-.6-1.4-1.4-1.8-1.4-1.8-1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.7-1.6-2.7-.3-5.5-1.3-5.5-6 0-1.2.5-2.3 1.2-3.1-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0C17.3 4.7 18.3 5 18.3 5c.7 1.7.2 2.9.1 3.2.8.8 1.2 1.9 1.2 3.1 0 4.7-2.8 5.7-5.5 6 .4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .5Z" />
            </svg>
          </a>
          <span className="mx-1 h-4 w-px bg-border" aria-hidden="true" />
          <LanguageMenu
            current={locale}
            label={t.nav.language}
            options={LOCALES.map((code) => ({ code, label: LOCALE_INFO[code].label, tag: LOCALE_INFO[code].tag }))}
          />
        </div>
      </div>
    </header>
  );
}

function HeaderLink({ href, className = "", children }: { href: string; className?: string; children: ReactNode }) {
  return (
    <Link
      href={href}
      className={`inline-flex h-8 items-center rounded-lg px-2.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground ${className}`}
    >
      {children}
    </Link>
  );
}
