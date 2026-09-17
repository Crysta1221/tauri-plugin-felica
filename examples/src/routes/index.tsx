import { createFileRoute } from "@tanstack/react-router";
import { ReaderScanPage } from "@/features/reader-scan/reader-scan-page";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return <ReaderScanPage />;
}
