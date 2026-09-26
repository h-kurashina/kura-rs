import { Geist_Mono, Noto_Sans_JP, Noto_Sans_KR, Noto_Sans_SC } from "next/font/google";

// ビルド時に Google Fonts から取得して自己ホストする（実行時の外部通信なし）。
// 韓国語・中国語の字形は、その言語のページでだけ使われる（globals.css の :lang 指定）ので先読みしない
const notoSansJp = Noto_Sans_JP({ subsets: ["latin"], variable: "--font-noto-sans-jp", display: "swap" });
const notoSansKr = Noto_Sans_KR({ subsets: ["latin"], variable: "--font-noto-sans-kr", display: "swap", preload: false });
const notoSansSc = Noto_Sans_SC({ subsets: ["latin"], variable: "--font-noto-sans-sc", display: "swap", preload: false });
const geistMono = Geist_Mono({ subsets: ["latin"], variable: "--font-geist-mono", display: "swap" });

/** <html> に付けるフォント変数のクラス */
export const fontVariables = [notoSansJp, notoSansKr, notoSansSc, geistMono].map((f) => f.variable).join(" ");
