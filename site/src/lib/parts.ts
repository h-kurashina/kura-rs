/**
 * kura-registry が site/public/r/ に書き出した JSON を読む（ビルド時専用）。
 * 検証は Rust 側で済んでいるので、ここでは生成済みの型を信頼して読むだけにする。
 */
import fs from "node:fs";
import path from "node:path";
import type { Part } from "@/generated/Part";
import type { PartIndex } from "@/generated/PartIndex";
import type { PartSummary } from "@/generated/PartSummary";
import type { Shelf } from "@/generated/Shelf";

const R_DIR = path.join(process.cwd(), "public", "r");

function readJson<T>(file: string): T {
  const full = path.join(R_DIR, file);
  if (!fs.existsSync(full)) {
    throw new Error(`${full} not found. Run \`npm run registry\` (cargo run -p kura-registry -- build) first.`);
  }
  const data: unknown = JSON.parse(fs.readFileSync(full, "utf8"));
  return data as T;
}

let indexCache: PartIndex | null = null;

export function getIndex(): PartIndex {
  // 開発中は registry を書き換えたらすぐ反映させたいので、本番ビルドのときだけ覚えておく
  if (process.env.NODE_ENV !== "production") return readJson<PartIndex>("index.json");
  indexCache ??= readJson<PartIndex>("index.json");
  return indexCache;
}

export function getPart(name: string): Part | undefined {
  const summary = getIndex().parts.find((p) => p.name === name);
  return summary ? readJson<Part>(summary.path) : undefined;
}

/** いちばん新しく追加された部品（同じ日なら名前順で先のもの）。トップの「新着」に出す */
export function getLatestPart(): PartSummary | undefined {
  return [...getIndex().parts].sort((a, b) => b.added.localeCompare(a.added) || a.name.localeCompare(b.name))[0];
}

export const SHELF_ORDER: readonly Shelf[] = ["ai", "security"];

export function getPartsByShelf(): { shelf: Shelf; parts: PartSummary[] }[] {
  const parts = getIndex().parts;
  return SHELF_ORDER.map((shelf) => ({
    shelf,
    parts: parts.filter((p) => p.shelves.includes(shelf)).sort((a, b) => a.title.localeCompare(b.title)),
  }));
}
