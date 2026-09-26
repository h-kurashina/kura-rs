// /og/site.png : トップと一覧ページの OGP 画像
import { ImageResponse } from "next/og";
import { OG_SIZE, OgCard } from "@/components/og-card";
import { getDictionary } from "@/i18n/dictionaries";
import { getIndex } from "@/lib/parts";

export const dynamic = "force-static";

export function GET(): ImageResponse {
  const t = getDictionary("en");
  const n = getIndex().parts.length;
  return new ImageResponse(
    <OgCard
      eyebrow="Rust parts registry"
      title={t.meta.tagline}
      description={t.meta.description}
      facts={[`${n} ${n === 1 ? "part" : "parts"}`, "Rust", "Python (PyO3)"]}
    />,
    OG_SIZE,
  );
}
