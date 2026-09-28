import type { ReactNode } from "react";

export function Section({
  id,
  title,
  aside,
  children,
}: {
  id: string;
  title: string;
  aside?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section id={id} className="scroll-mt-20 space-y-4">
      <div className="flex flex-wrap items-center gap-3 border-b pb-3">
        <h2 className="text-lg font-semibold tracking-tight">
          <a href={`#${id}`} className="underline-offset-4 hover:underline">
            {title}
          </a>
        </h2>
        {aside}
      </div>
      {children}
    </section>
  );
}

export function Stat({ label, value, detail }: { label: string; value: ReactNode; detail?: ReactNode }) {
  return (
    <div className="min-w-0 rounded-lg border bg-card p-4">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1.5 text-lg leading-snug font-semibold tracking-tight tabular-nums [overflow-wrap:anywhere]">{value}</div>
      {detail && <div className="mt-0.5 text-xs text-muted-foreground">{detail}</div>}
    </div>
  );
}
