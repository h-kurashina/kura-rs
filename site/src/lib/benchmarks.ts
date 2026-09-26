// ベンチマークのグラフ用の計算（損益分岐点）。最大倍率は Rust 側が index.json に入れる。
import type { BenchmarkPoint } from "@/generated/BenchmarkPoint";

export function speedup(p: BenchmarkPoint): number {
  return p.reference_ms / p.rust_ms;
}

export type BreakEven =
  | { kind: "always-faster" }
  | { kind: "never-faster" }
  /** inputSize: 対数軸上で補間した交点。これより大きい入力で Rust 版が速い */
  | { kind: "crossover"; inputSize: number; firstFasterMeasured: number };

/**
 * Rust 版が参照実装より速くなる入力サイズを求める。
 * 最後に「遅い→速い」に切り替わった区間を、log(サイズ) と log(倍率) で線形補間する。
 */
export function breakEven(points: readonly BenchmarkPoint[]): BreakEven {
  const faster = points.map((p) => p.rust_ms < p.reference_ms);
  if (faster.every(Boolean)) return { kind: "always-faster" };
  if (!faster.at(-1)) return { kind: "never-faster" };

  const lastSlower = faster.lastIndexOf(false);
  const a = points[lastSlower];
  const b = points[lastSlower + 1];
  if (!a || !b) return { kind: "always-faster" };

  const la = Math.log(speedup(a));
  const lb = Math.log(speedup(b));
  const t = la === lb ? 0.5 : -la / (lb - la);
  const x = Math.exp(Math.log(a.input_size) + t * (Math.log(b.input_size) - Math.log(a.input_size)));
  return { kind: "crossover", inputSize: x, firstFasterMeasured: b.input_size };
}
