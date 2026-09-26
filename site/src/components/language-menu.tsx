"use client";

import { usePathname } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import type { Locale } from "@/i18n/dictionaries";
import { switchLocalePath } from "@/i18n/locale";

export interface LanguageOption {
  code: Locale;
  label: string;
  tag: string;
}

/**
 * 言語メニュー。今のページの別の言語版へ移る。
 * 言語ごとにルートレイアウトが別なので、Link ではなく通常の a でページ全体を読み直す。
 */
export function LanguageMenu({ current, options, label }: { current: Locale; options: LanguageOption[]; label: string }) {
  const [open, setOpen] = useState(false);
  const pathname = usePathname();
  const root = useRef<HTMLDivElement>(null);
  const base = process.env.NEXT_PUBLIC_BASE_PATH ?? "";
  const currentLabel = options.find((o) => o.code === current)?.label ?? current;

  useEffect(() => {
    if (!open) return;
    function onPointer(e: PointerEvent) {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={root} className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`${label}: ${currentLabel}`}
        onClick={() => setOpen((o) => !o)}
        className="inline-flex h-8 items-center gap-1.5 rounded-lg px-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
          <circle cx="12" cy="12" r="9" />
          <path d="M3 12h18M12 3c2.5 2.7 3.8 5.7 3.8 9s-1.3 6.3-3.8 9c-2.5-2.7-3.8-5.7-3.8-9s1.3-6.3 3.8-9Z" />
        </svg>
        <span className="hidden sm:inline">{currentLabel}</span>
      </button>
      {open && (
        <ul
          role="menu"
          aria-label={label}
          className="anim-pop absolute right-0 z-50 mt-1.5 min-w-44 origin-top-right overflow-hidden rounded-xl border bg-card p-1 shadow-lg"
        >
          {options.map((o) => (
            <li key={o.code} role="none">
              <a
                role="menuitem"
                href={`${base}${switchLocalePath(pathname, o.code)}`}
                hrefLang={o.tag}
                lang={o.tag}
                aria-current={o.code === current ? "true" : undefined}
                className="flex items-center justify-between gap-3 rounded-lg px-2.5 py-1.5 text-sm transition-colors hover:bg-accent"
              >
                {o.label}
                {o.code === current && (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                    <path d="M20 6 9 17l-5-5" />
                  </svg>
                )}
              </a>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
