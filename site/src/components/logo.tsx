/**
 * kura-rs のロゴマーク。「蔵」を三角の屋根と棚に並んだ4つの部品で表し、
 * 右下の1つだけ薄くして「部品を取り出してコピーする」ことを示す。
 * 色は currentColor に従う。
 */
export function LogoMark({ className = "", title }: { className?: string; title?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={className} role={title ? "img" : undefined} aria-hidden={title ? undefined : true} aria-label={title}>
      <path d="M12 1.5 22.5 9h-21z" fill="currentColor" />
      <rect x="1.5" y="11" width="9.5" height="4.5" rx="1" fill="currentColor" />
      <rect x="13" y="11" width="9.5" height="4.5" rx="1" fill="currentColor" />
      <rect x="1.5" y="17.5" width="9.5" height="4.5" rx="1" fill="currentColor" />
      <rect
        x="13"
        y="17.5"
        width="9.5"
        height="4.5"
        rx="1"
        fill="currentColor"
        fillOpacity="0.4"
        className="transition-transform duration-300 ease-out group-hover/logo:translate-x-[1.5px] group-hover/logo:translate-y-[1px] motion-reduce:transition-none"
      />
    </svg>
  );
}
