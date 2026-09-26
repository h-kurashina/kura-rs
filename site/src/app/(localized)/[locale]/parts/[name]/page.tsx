import type { Metadata } from "next";
import { localeFromParams } from "@/i18n/params";
import { PartDetailView, partMetadata, partStaticParams } from "@/views/part-detail";

interface Props {
  params: Promise<{ locale: string; name: string }>;
}

export const dynamicParams = false;
export const generateStaticParams = partStaticParams;

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  return partMetadata({ params }, await localeFromParams(params));
}

export default async function Page({ params }: Props) {
  return <PartDetailView params={params} locale={await localeFromParams(params)} />;
}
