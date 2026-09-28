"use client";

import { useState } from "react";
import { CopyButton, type CopyLabels } from "@/components/copy-button";

export interface CodeTab {
  id: string;
  label: string;
  code: string;
  /** Shiki でハイライト済みの HTML（サーバー側で作る） */
  html: string;
}

/** 見出し帯にタブとコピーボタンを持つコードブロック。タブが1つなら見出しとして表示する */
export function CodeTabs({ tabs, labels, icon = "code" }: { tabs: CodeTab[]; labels: CopyLabels; icon?: "terminal" | "code" }) {
  const [active, setActive] = useState(tabs[0]?.id ?? "");
  const current = tabs.find((t) => t.id === active) ?? tabs[0];
  if (!current) return null;

  return (
    <div className="overflow-hidden rounded-lg border bg-code">
      <div className="flex h-11 items-center gap-2 border-b bg-code-header px-3">
        <span className="flex size-5 items-center justify-center rounded-sm bg-foreground/80 text-background" aria-hidden="true">
          {icon === "terminal" ? (
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
              <path d="m5 7 5 5-5 5M13 17h6" />
            </svg>
          ) : (
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
              <path d="m8 7-5 5 5 5M16 7l5 5-5 5" />
            </svg>
          )}
        </span>
        <div role="tablist" className="flex items-center gap-1">
          {tabs.map((tab) => {
            const selected = tab.id === current.id;
            return (
              <button
                key={tab.id}
                type="button"
                role="tab"
                aria-selected={selected}
                onClick={() => setActive(tab.id)}
                disabled={tabs.length === 1}
                className={`rounded-md px-2 py-1 font-mono text-[13px] transition-colors disabled:cursor-default ${
                  selected ? "bg-background text-foreground shadow-xs dark:bg-accent" : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {tab.label}
              </button>
            );
          })}
        </div>
        <CopyButton value={current.code} labels={labels} className="ml-auto" />
      </div>
      <div
        key={current.id}
        role="tabpanel"
        className="anim-fade overflow-x-auto px-4 py-3.5 font-mono text-[13px] leading-6 [&_pre]:outline-none"
        dangerouslySetInnerHTML={{ __html: current.html }}
      />
    </div>
  );
}
