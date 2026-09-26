"use client";

import { useSyncExternalStore, useState } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ReferenceArea,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
  type TooltipContentProps,
  type TooltipValueType,
} from "recharts";
import type { BenchmarkPoint } from "@/generated/BenchmarkPoint";
import { fill, type Dictionary, type Locale } from "@/i18n/dictionaries";
import type { BreakEven } from "@/lib/benchmarks";
import { formatFactor, formatMs, formatSize } from "@/lib/format";

interface Props {
  points: BenchmarkPoint[];
  breakEven: BreakEven;
  referenceName: string;
  locale: Locale;
  t: Pick<Dictionary, "chart" | "common">;
}

type YScale = "log" | "linear";

const REDUCED_MOTION = "(prefers-reduced-motion: reduce)";

/** OS の「視差効果を減らす」設定。サーバー側では動かさない前提で true を返す */
function useReducedMotion(): boolean {
  return useSyncExternalStore(
    (onChange) => {
      const mq = window.matchMedia(REDUCED_MOTION);
      mq.addEventListener("change", onChange);
      return () => mq.removeEventListener("change", onChange);
    },
    () => window.matchMedia(REDUCED_MOTION).matches,
    () => true,
  );
}

function powersOfTen(min: number, max: number): number[] {
  const out: number[] = [];
  for (let e = Math.floor(Math.log10(min)); e <= Math.ceil(Math.log10(max)); e++) out.push(10 ** e);
  return out;
}

function ChartTooltip({
  active,
  payload,
  referenceName,
  locale,
  t,
}: TooltipContentProps<TooltipValueType, string | number> & Pick<Props, "referenceName" | "locale" | "t">) {
  const point = payload?.[0]?.payload as BenchmarkPoint | undefined;
  if (!active || !point) return null;
  const factor = point.reference_ms / point.rust_ms;
  return (
    <div className="min-w-44 rounded-lg border bg-card px-3 py-2 text-xs shadow-lg">
      <div className="mb-1.5 text-muted-foreground">
        {fill(t.chart.tooltipSize, { size: formatSize(point.input_size, locale) })}
      </div>
      <Row color="var(--series-rust)" label="kura-rs (Rust)" value={formatMs(point.rust_ms, locale)} />
      <Row color="var(--series-reference)" label={referenceName} value={formatMs(point.reference_ms, locale)} dashed />
      <div className="mt-1.5 border-t pt-1.5 font-medium">
        {fill(factor >= 1 ? t.common.fasterBy : t.common.slowerBy, { factor: formatFactor(factor >= 1 ? factor : 1 / factor) })}
      </div>
    </div>
  );
}

function Row({ color, label, value, dashed = false }: { color: string; label: string; value: string; dashed?: boolean }) {
  return (
    <div className="flex items-center gap-2 py-0.5">
      <LineKey color={color} dashed={dashed} />
      <span className="font-medium text-foreground tabular-nums">{value}</span>
      <span className="text-muted-foreground">{label}</span>
    </div>
  );
}

function LineKey({ color, dashed }: { color: string; dashed: boolean }) {
  return (
    <svg width="16" height="8" aria-hidden="true" className="shrink-0">
      <line x1="0" y1="4" x2="16" y2="4" stroke={color} strokeWidth="2" strokeDasharray={dashed ? "4 3" : undefined} />
    </svg>
  );
}

export function BenchmarkChart({ points, breakEven, referenceName, locale, t }: Props) {
  const size = (n: number) => formatSize(n, locale);
  const ms = (n: number) => formatMs(n, locale);
  const [yScale, setYScale] = useState<YScale>("log");
  const reducedMotion = useReducedMotion();
  const first = points[0];
  const last = points.at(-1);
  if (!first || !last) return null;

  const xTicks = powersOfTen(first.input_size, last.input_size).filter((t) => t >= first.input_size && t <= last.input_size);
  const allMs = points.flatMap((p) => [p.rust_ms, p.reference_ms]);
  const yTicks = powersOfTen(Math.min(...allMs), Math.max(...allMs));

  return (
    <figure className="rounded-xl border bg-card p-4 shadow-xs sm:p-5">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-x-5 gap-y-1 text-xs text-muted-foreground" aria-label={t.chart.legend}>
          <span className="flex items-center gap-2">
            <LineKey color="var(--series-rust)" dashed={false} />
            <span className="text-foreground">kura-rs (Rust)</span>
          </span>
          <span className="flex items-center gap-2">
            <LineKey color="var(--series-reference)" dashed />
            <span className="text-foreground">{referenceName}</span>
          </span>
        </div>
        <div role="group" aria-label={t.chart.yScale} className="flex rounded-lg bg-secondary p-0.5 text-xs">
          {(["log", "linear"] as const).map((s) => (
            <button
              key={s}
              type="button"
              aria-pressed={yScale === s}
              onClick={() => setYScale(s)}
              className={`rounded-md px-2.5 py-1 font-medium transition-colors ${
                yScale === s ? "bg-background text-foreground shadow-xs dark:bg-accent" : "text-muted-foreground hover:text-foreground"
              }`}
            >
              {s === "log" ? t.chart.log : t.chart.linear}
            </button>
          ))}
        </div>
      </div>

      <div className="h-72 w-full sm:h-80">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={points} margin={{ top: 8, right: 16, bottom: 20, left: 8 }}>
            <CartesianGrid stroke="var(--chart-grid)" vertical={false} />
            {breakEven.kind === "crossover" && (
              <ReferenceArea
                x1={first.input_size}
                x2={breakEven.inputSize}
                fill="var(--chart-slower)"
                fillOpacity={1}
                ifOverflow="hidden"
              />
            )}
            <XAxis
              dataKey="input_size"
              type="number"
              scale="log"
              domain={[first.input_size, last.input_size]}
              ticks={xTicks}
              tickFormatter={size}
              stroke="var(--muted-foreground)"
              tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
              tickLine={false}
              axisLine={{ stroke: "var(--border)" }}
              label={{ value: t.chart.xLabel, position: "insideBottom", offset: -12, fontSize: 11, fill: "var(--muted-foreground)" }}
            />
            <YAxis
              type="number"
              scale={yScale}
              domain={yScale === "log" ? [yTicks[0] ?? "auto", yTicks.at(-1) ?? "auto"] : [0, "auto"]}
              ticks={yScale === "log" ? yTicks : undefined}
              allowDataOverflow={false}
              tickFormatter={ms}
              width={64}
              tick={{ fontSize: 11, fill: "var(--muted-foreground)" }}
              tickLine={false}
              axisLine={false}
            />
            {breakEven.kind === "crossover" && (
              <ReferenceLine
                x={breakEven.inputSize}
                stroke="var(--foreground)"
                strokeOpacity={0.5}
                strokeDasharray="2 3"
                label={{
                  value: fill(t.chart.breakEven, { size: size(breakEven.inputSize) }),
                  position: "insideTopLeft",
                  fontSize: 11,
                  fill: "var(--foreground)",
                }}
              />
            )}
            <Tooltip
              cursor={{ stroke: "var(--muted-foreground)", strokeWidth: 1 }}
              content={(props) => <ChartTooltip {...props} referenceName={referenceName} locale={locale} t={t} />}
            />
            <Line
              dataKey="reference_ms"
              name={referenceName}
              stroke="var(--series-reference)"
              strokeWidth={2}
              strokeDasharray="6 4"
              dot={{ r: 4, fill: "var(--series-reference)", stroke: "var(--background)", strokeWidth: 2 }}
              activeDot={{ r: 5, stroke: "var(--background)", strokeWidth: 2 }}
              isAnimationActive={!reducedMotion}
              animationDuration={900}
              animationEasing="ease-out"
            />
            <Line
              dataKey="rust_ms"
              name="kura-rs (Rust)"
              stroke="var(--series-rust)"
              strokeWidth={2}
              dot={{ r: 4, fill: "var(--series-rust)", stroke: "var(--background)", strokeWidth: 2 }}
              activeDot={{ r: 5, stroke: "var(--background)", strokeWidth: 2 }}
              isAnimationActive={!reducedMotion}
              animationDuration={900}
              animationEasing="ease-out"
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
      <figcaption className="mt-2 text-xs text-muted-foreground">
        {t.chart.caption}
        {breakEven.kind === "crossover" && ` ${t.chart.captionShaded}`}
      </figcaption>
    </figure>
  );
}
