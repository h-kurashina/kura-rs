/**
 * OGP 画像（1200×630）の中身。next/og の ImageResponse で描くので、使える CSS は一部だけ。
 * 既定のフォントに日本語の字形がないため、文字は英語だけにする。
 */
import type { BenchmarkPoint } from "@/generated/BenchmarkPoint";

export const OG_SIZE = { width: 1200, height: 630 };

function sparklinePath(points: BenchmarkPoint[], key: "rust_ms" | "reference_ms", w: number, h: number): string {
  const xs = points.map((p) => Math.log10(p.input_size));
  const ys = points.flatMap((p) => [Math.log10(p.rust_ms), Math.log10(p.reference_ms)]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  return points
    .map((p, i) => {
      const x = ((Math.log10(p.input_size) - x0) / (x1 - x0 || 1)) * w;
      const y = h - ((Math.log10(p[key]) - y0) / (y1 - y0 || 1)) * h;
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

export function OgCard({
  eyebrow,
  title,
  description,
  facts = [],
  sample = false,
  points,
}: {
  eyebrow: string;
  title: string;
  description: string;
  facts?: string[];
  sample?: boolean;
  points?: BenchmarkPoint[];
}) {
  const chart = { w: 1040, h: 150 };
  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        padding: "64px 80px",
        background: "#0a0a0a",
        color: "#fafafa",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
        <svg width="44" height="44" viewBox="0 0 24 24">
          <path d="M12 1.5 22.5 9h-21z" fill="#fafafa" />
          <rect x="1.5" y="11" width="9.5" height="4.5" rx="1" fill="#fafafa" />
          <rect x="13" y="11" width="9.5" height="4.5" rx="1" fill="#fafafa" />
          <rect x="1.5" y="17.5" width="9.5" height="4.5" rx="1" fill="#fafafa" />
          <rect x="13" y="17.5" width="9.5" height="4.5" rx="1" fill="#fafafa" fillOpacity="0.4" />
        </svg>
        <div style={{ fontSize: 30, fontWeight: 600 }}>kura-rs</div>
        <div style={{ fontSize: 26, color: "#a1a1a1", marginLeft: 8 }}>{eyebrow}</div>
        {sample && (
          <div
            style={{
              marginLeft: "auto",
              fontSize: 22,
              color: "#a1a1a1",
              border: "2px dashed #525252",
              borderRadius: 999,
              padding: "6px 18px",
            }}
          >
            Sample data
          </div>
        )}
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 18 }}>
        <div style={{ fontSize: 76, fontWeight: 700, letterSpacing: -2, lineHeight: 1.05 }}>{title}</div>
        <div style={{ fontSize: 30, color: "#a1a1a1", lineHeight: 1.4, maxWidth: 1000 }}>{description}</div>
      </div>

      {points && points.length > 1 ? (
        <svg width={chart.w} height={chart.h} viewBox={`-4 -4 ${chart.w + 8} ${chart.h + 8}`}>
          <path d={sparklinePath(points, "reference_ms", chart.w, chart.h)} fill="none" stroke="#d95926" strokeWidth="5" strokeDasharray="14 10" />
          <path d={sparklinePath(points, "rust_ms", chart.w, chart.h)} fill="none" stroke="#3987e5" strokeWidth="5" />
        </svg>
      ) : null}

      <div style={{ display: "flex", gap: 16 }}>
        {facts.map((f) => (
          <div key={f} style={{ fontSize: 24, background: "#262626", borderRadius: 999, padding: "8px 22px" }}>
            {f}
          </div>
        ))}
      </div>
    </div>
  );
}
