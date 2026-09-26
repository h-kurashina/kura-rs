"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";

export interface SidebarSection {
  label: string;
  items: { name: string; title: string; href: string }[];
}

export interface SidebarLabels {
  gettingStarted: string;
  allParts: string;
  noParts: string;
}

function samePath(a: string, b: string): boolean {
  return a.replace(/\/$/, "") === b.replace(/\/$/, "");
}

export function Sidebar({ sections, allPartsHref, t }: { sections: SidebarSection[]; allPartsHref: string; t: SidebarLabels }) {
  const pathname = usePathname();
  return (
    <nav aria-label={t.allParts} className="space-y-6 text-[13px]">
      <div>
        <h4 className="mb-1 px-2.5 text-xs font-medium text-muted-foreground">{t.gettingStarted}</h4>
        <SidebarLink href={allPartsHref} active={samePath(pathname, allPartsHref)}>
          {t.allParts}
        </SidebarLink>
      </div>
      {sections.map((section) => (
        <div key={section.label}>
          <h4 className="mb-1 px-2.5 text-xs font-medium text-muted-foreground">{section.label}</h4>
          {section.items.length === 0 ? (
            <p className="px-2.5 py-1 text-muted-foreground/70">{t.noParts}</p>
          ) : (
            section.items.map((item) => (
              <SidebarLink key={item.name} href={item.href} active={samePath(pathname, item.href)}>
                {item.title}
              </SidebarLink>
            ))
          )}
        </div>
      ))}
    </nav>
  );
}

function SidebarLink({ href, active, children }: { href: string; active: boolean; children: ReactNode }) {
  return (
    <Link
      href={href}
      aria-current={active ? "page" : undefined}
      className={`flex h-8 items-center rounded-lg px-2.5 font-medium transition-colors ${
        active ? "bg-accent text-foreground" : "text-foreground/80 hover:bg-accent/60 hover:text-foreground"
      }`}
    >
      {children}
    </Link>
  );
}
