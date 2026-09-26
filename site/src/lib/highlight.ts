// Shiki によるビルド時のシンタックスハイライト（サーバー側専用）
import { createHighlighter, type Highlighter } from "shiki";

export type CodeLang = "rust" | "python" | "bash" | "json";

let highlighter: Promise<Highlighter> | null = null;

function getHighlighter(): Promise<Highlighter> {
  highlighter ??= createHighlighter({
    themes: ["github-light", "github-dark"],
    langs: ["rust", "python", "bash", "json"],
  });
  return highlighter;
}

export async function highlight(code: string, lang: CodeLang): Promise<string> {
  const h = await getHighlighter();
  return h.codeToHtml(code, {
    lang,
    themes: { light: "github-light", dark: "github-dark" },
  });
}
