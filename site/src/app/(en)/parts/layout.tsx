import type { ReactNode } from "react";
import { PartsLayout } from "@/views/parts-layout";

export default function Layout({ children }: { children: ReactNode }) {
  return <PartsLayout locale="en">{children}</PartsLayout>;
}
