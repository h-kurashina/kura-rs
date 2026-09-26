import { HomeView, homeMetadata } from "@/views/home";

export const metadata = homeMetadata("en");

export default function Page() {
  return <HomeView locale="en" />;
}
