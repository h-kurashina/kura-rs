import type { NextConfig } from "next";

// GitHub Pages のサブパスに置く場合は NEXT_PUBLIC_BASE_PATH=/kura のように指定する
const basePath = process.env.NEXT_PUBLIC_BASE_PATH ?? "";

const nextConfig: NextConfig = {
  output: "export",
  basePath,
  trailingSlash: true,
  images: { unoptimized: true },
  // 言語ごとにルートレイアウトが分かれているので、404 は global-not-found で出す
  experimental: { globalNotFound: true },
};

export default nextConfig;
