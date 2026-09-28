/**
 * 見た目の基本部品（ボタン・カード・ラベル）。色は globals.css のトークンだけを使う。
 */
import type { ComponentProps, ReactNode } from "react";

type ButtonVariant = "primary" | "secondary" | "outline" | "ghost";
type ButtonSize = "sm" | "md";

const BUTTON_BASE =
  "inline-flex shrink-0 items-center justify-center gap-1.5 rounded-md text-sm font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50";
const BUTTON_VARIANT: Record<ButtonVariant, string> = {
  primary: "bg-primary text-primary-foreground shadow-xs hover:bg-primary/90",
  secondary: "bg-secondary text-foreground hover:bg-secondary/80",
  outline: "border bg-background shadow-xs hover:bg-accent dark:border-input dark:bg-input/30 dark:hover:bg-input/50",
  ghost: "hover:bg-accent",
};
const BUTTON_SIZE: Record<ButtonSize, string> = {
  sm: "h-8 px-3",
  md: "h-9 px-4",
};

export function buttonClass(variant: ButtonVariant = "primary", size: ButtonSize = "md", extra = ""): string {
  return `${BUTTON_BASE} ${BUTTON_VARIANT[variant]} ${BUTTON_SIZE[size]} ${extra}`;
}

export function Card({ className = "", ...props }: ComponentProps<"div">) {
  return <div className={`rounded-lg border bg-card text-foreground ${className}`} {...props} />;
}

export function CardHeader({ title, description, action }: { title: ReactNode; description?: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-3 px-5 pt-5">
      <div className="min-w-0 space-y-1">
        <div className="leading-none font-semibold">{title}</div>
        {description && <div className="text-sm text-muted-foreground">{description}</div>}
      </div>
      {action}
    </div>
  );
}

export function Pill({ className = "", ...props }: ComponentProps<"span">) {
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full bg-secondary px-2.5 py-0.5 text-xs font-medium text-foreground ${className}`}
      {...props}
    />
  );
}
