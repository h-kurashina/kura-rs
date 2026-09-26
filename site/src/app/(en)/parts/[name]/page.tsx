import type { Metadata } from "next";
import { PartDetailView, partMetadata, partStaticParams, type PartParams } from "@/views/part-detail";

export const dynamicParams = false;
export const generateStaticParams = partStaticParams;

export function generateMetadata(props: PartParams): Promise<Metadata> {
  return partMetadata(props, "en");
}

export default function Page(props: PartParams) {
  return <PartDetailView {...props} locale="en" />;
}
