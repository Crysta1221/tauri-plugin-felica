import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { ReaderProvider } from "@/features/reader-context";
import { ReaderToolbar } from "@/features/reader-toolbar";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  return (
    <ReaderProvider>
      <div className="flex h-dvh flex-col overflow-hidden">
        <nav className="flex shrink-0 gap-3 border-b px-5 py-2 text-sm">
          <Link to="/" className="text-muted-foreground data-[status=active]:text-foreground">
            スキャン
          </Link>
          <Link to="/lab" className="text-muted-foreground data-[status=active]:text-foreground">
            ラボ
          </Link>
        </nav>
        <ReaderToolbar />
        <div className="min-h-0 flex-1 overflow-hidden">
          <Outlet />
        </div>
      </div>
    </ReaderProvider>
  );
}
