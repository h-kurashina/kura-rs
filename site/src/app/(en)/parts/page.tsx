import { PartsListView, partsListMetadata } from "@/views/parts-list";

export const metadata = partsListMetadata("en");

export default function Page() {
  return <PartsListView locale="en" />;
}
