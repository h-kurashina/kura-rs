import type { Metadata } from "next";
import { localeFromParams } from "@/i18n/params";
import { PartsListView, partsListMetadata } from "@/views/parts-list";

interface Props {
  params: Promise<{ locale: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  return partsListMetadata(await localeFromParams(params));
}

export default async function Page({ params }: Props) {
  return <PartsListView locale={await localeFromParams(params)} />;
}
