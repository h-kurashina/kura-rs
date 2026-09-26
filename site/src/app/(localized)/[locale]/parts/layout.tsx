import type { ReactNode } from "react";
import { localeFromParams } from "@/i18n/params";
import { PartsLayout } from "@/views/parts-layout";

export default async function Layout({ children, params }: { children: ReactNode; params: Promise<{ locale: string }> }) {
  return <PartsLayout locale={await localeFromParams(params)}>{children}</PartsLayout>;
}
