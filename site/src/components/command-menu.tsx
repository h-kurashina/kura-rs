"use client";

import { Command } from "cmdk";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";

export interface CommandGroupData {
  label: string;
  items: { name: string; title: string; description: string; reference: string; href: string }[];
}

export interface CommandLabels {
  button: string;
  placeholder: string;
  empty: string;
  pages: string;
  allParts: string;
}

export function CommandMenu({ groups, allPartsHref, t }: { groups: CommandGroupData[]; allPartsHref: string; t: CommandLabels }) {
  const [open, setOpen] = useState(false);
  const router = useRouter();

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key.toLowerCase() === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((o) => !o);
      }
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);

  function go(href: string) {
    setOpen(false);
    router.push(href);
  }

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="inline-flex h-8 items-center gap-2 rounded-lg bg-secondary px-2.5 text-sm text-muted-foreground transition-colors hover:bg-secondary/80 hover:text-foreground sm:w-60 sm:pr-1.5"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
          <circle cx="11" cy="11" r="7" />
          <path d="m20 20-3.5-3.5" />
        </svg>
        <span className="hidden sm:inline">{t.button}</span>
        <span className="sr-only sm:hidden">{t.button}</span>
        <kbd className="ml-auto hidden h-5 items-center gap-0.5 rounded border bg-background px-1.5 font-mono text-[11px] font-medium text-muted-foreground sm:inline-flex">
          ⌘K
        </kbd>
      </button>

      <Command.Dialog
        open={open}
        onOpenChange={setOpen}
        label={t.button}
        overlayClassName="anim-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-[2px]"
        contentClassName="fixed top-[18vh] left-1/2 z-50 w-[calc(100vw-2rem)] max-w-lg -translate-x-1/2"
      >
        <div className="anim-pop overflow-hidden rounded-xl border bg-card p-1 shadow-2xl">
        <Command.Input
          placeholder={t.placeholder}
          className="h-11 w-full rounded-lg border-b bg-transparent px-3 text-sm outline-none placeholder:text-muted-foreground"
        />
        <Command.List className="max-h-80 overflow-y-auto p-2">
          <Command.Empty className="py-6 text-center text-sm text-muted-foreground">{t.empty}</Command.Empty>
          <Command.Group heading={t.pages} className={GROUP}>
            <Command.Item value={t.allParts} onSelect={() => go(allPartsHref)} className={ITEM}>
              {t.allParts}
            </Command.Item>
          </Command.Group>
          {groups
            .filter((g) => g.items.length > 0)
            .map((g) => (
              <Command.Group key={g.label} heading={g.label} className={GROUP}>
                {g.items.map((item) => (
                  <Command.Item
                    key={item.name}
                    value={`${g.label} ${item.name}`}
                    keywords={[item.name, item.title, item.description, item.reference]}
                    onSelect={() => go(item.href)}
                    className={ITEM}
                  >
                    <span className="font-medium">{item.title}</span>
                    <span className="truncate text-muted-foreground">{item.description}</span>
                  </Command.Item>
                ))}
              </Command.Group>
            ))}
        </Command.List>
        </div>
      </Command.Dialog>
    </>
  );
}

const GROUP =
  "[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:text-muted-foreground";
const ITEM =
  "flex cursor-pointer items-center gap-3 rounded-lg px-2.5 py-2 text-sm data-[selected=true]:bg-accent";
