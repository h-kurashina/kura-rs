import type { ReactNode } from "react";
import { RootShell, rootMetadata } from "@/views/root-shell";
import "../globals.css";

export const metadata = rootMetadata("en");

export default function Layout({ children }: { children: ReactNode }) {
  return <RootShell locale="en">{children}</RootShell>;
}
