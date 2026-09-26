import type { Dictionary } from "@/i18n/dictionaries";
import { fillNode } from "@/i18n/fill-node";

export function SampleBadge({ t, className = "" }: { t: Dictionary["sample"]; className?: string }) {
  return (
    <span
      title={t.badgeTitle}
      className={`inline-flex items-center gap-1.5 rounded-full border border-dashed border-foreground/30 px-2 py-0.5 text-xs font-medium whitespace-nowrap text-muted-foreground ${className}`}
    >
      <span className="size-1.5 rounded-full bg-muted-foreground/70" aria-hidden="true" />
      {t.badge}
    </span>
  );
}

export function SampleNotice({ t }: { t: Dictionary["sample"] }) {
  return (
    <div className="flex gap-3 rounded-xl border border-dashed bg-card px-4 py-3 text-sm text-muted-foreground">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="mt-0.5 shrink-0" aria-hidden="true">
        <circle cx="12" cy="12" r="9" />
        <path d="M12 8v5M12 16h.01" strokeLinecap="round" />
      </svg>
      <p>
        <span className="font-medium text-foreground">{t.noticeLead}</span>{" "}
        {fillNode(t.noticeBody, { code: <code className="font-mono text-[13px]">&quot;sample&quot;: true</code> })}
      </p>
    </div>
  );
}
