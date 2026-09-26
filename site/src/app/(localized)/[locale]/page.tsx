import type { Metadata } from "next";
import { localeFromParams } from "@/i18n/params";
import { HomeView, homeMetadata } from "@/views/home";

interface Props {
  params: Promise<{ locale: string }>;
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  return homeMetadata(await localeFromParams(params));
}

export default async function Page({ params }: Props) {
  return <HomeView locale={await localeFromParams(params)} />;
}
