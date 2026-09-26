// /og/parts/<name>.png : 部品ページの OGP 画像
import { ImageResponse } from "next/og";
import { OG_SIZE, OgCard } from "@/components/og-card";
import { formatFactor } from "@/lib/format";
import { getIndex, getPart } from "@/lib/parts";

export const dynamic = "force-static";
export const dynamicParams = false;

export function generateStaticParams(): { file: string }[] {
  return getIndex().parts.map((p) => ({ file: `${p.name}.png` }));
}

export async function GET(_req: Request, ctx: RouteContext<"/og/parts/[file]">): Promise<Response> {
  const name = (await ctx.params).file.replace(/\.png$/, "");
  const part = getPart(name);
  const summary = getIndex().parts.find((p) => p.name === name);
  if (!part) return new Response("Not found", { status: 404 });

  const v = part.verification;
  const facts = [`${v.passed.toLocaleString("en-US")} / ${v.cases.toLocaleString("en-US")} cases match ${part.reference.name}`];
  if (summary?.max_speedup !== undefined) facts.push(`Up to ${formatFactor(summary.max_speedup)} faster`);

  return new ImageResponse(
    <OgCard
      eyebrow={part.shelves.map((s) => (s === "ai" ? "AI" : "Security")).join(" · ")}
      title={part.title}
      description={part.description}
      facts={facts}
      sample={part.sample}
      points={part.benchmarks}
    />,
    OG_SIZE,
  );
}
