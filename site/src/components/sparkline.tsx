import { useId } from "react";
import type { BenchmarkPoint } from "@/generated/BenchmarkPoint";

/**
 * カード用の小さなベンチマーク曲線（両軸とも対数）。数値の読み取りは詳細ページのグラフと表で行う。
 */
export function Sparkline({
  points,
  height = 64,
  className = "",
  label,
}: {
  points: BenchmarkPoint[];
  height?: number;
  className?: string;
  label: string;
}) {
  const clipId = useId();
  const width = 240;
  const pad = 4;
  if (points.length < 2) return null;

  const xs = points.map((p) => Math.log10(p.input_size));
  const ys = points.flatMap((p) => [Math.log10(p.rust_ms), Math.log10(p.reference_ms)]);
  const [x0, x1] = [Math.min(...xs), Math.max(...xs)];
  const [y0, y1] = [Math.min(...ys), Math.max(...ys)];
  const sx = (v: number) => pad + ((Math.log10(v) - x0) / (x1 - x0 || 1)) * (width - pad * 2);
  const sy = (v: number) => height - pad - ((Math.log10(v) - y0) / (y1 - y0 || 1)) * (height - pad * 2);
  const path = (key: "rust_ms" | "reference_ms") =>
    points.map((p, i) => `${i === 0 ? "M" : "L"}${sx(p.input_size).toFixed(1)},${sy(p[key]).toFixed(1)}`).join(" ");

  return (
    <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" role="img" aria-label={label} className={`w-full ${className}`} style={{ height }}>
      {/* 線が左から描かれるように、クリップ用の矩形を伸ばす（globals.css の anim-draw） */}
      <clipPath id={clipId}>
        <rect className="anim-draw" x="0" y="0" width={width} height={height} />
      </clipPath>
      <g clipPath={`url(#${clipId})`}>
        <path d={path("reference_ms")} fill="none" stroke="var(--series-reference)" strokeWidth="2" strokeDasharray="5 4" vectorEffect="non-scaling-stroke" />
        <path d={path("rust_ms")} fill="none" stroke="var(--series-rust)" strokeWidth="2" vectorEffect="non-scaling-stroke" />
      </g>
    </svg>
  );
}
