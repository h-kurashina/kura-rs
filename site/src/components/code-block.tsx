import type { CopyLabels } from "@/components/copy-button";
import { CodeTabs } from "@/components/code-tabs";
import { highlight, type CodeLang } from "@/lib/highlight";

export interface CodeSource {
  id: string;
  label: string;
  code: string;
  lang: CodeLang;
}

/** サーバー側でハイライトしてから、タブ付きのコードブロックとして出す */
export async function CodeBlock({
  sources,
  labels,
  icon,
}: {
  sources: CodeSource[];
  labels: CopyLabels;
  icon?: "terminal" | "code";
}) {
  const tabs = await Promise.all(sources.map(async (s) => ({ ...s, html: await highlight(s.code, s.lang) })));
  return <CodeTabs tabs={tabs} labels={labels} icon={icon} />;
}
