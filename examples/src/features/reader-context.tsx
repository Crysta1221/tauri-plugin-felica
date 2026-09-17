import { createContext, useContext, useEffect, type ReactNode } from "react";
import { useRouterState } from "@tanstack/react-router";
import { useReaderScan } from "@/features/reader-scan/use-reader-scan";

type ReaderContextValue = ReturnType<typeof useReaderScan>;

const ReaderContext = createContext<ReaderContextValue | null>(null);

export function ReaderProvider({ children }: { children: ReactNode }) {
  const value = useReaderScan();
  const pathname = useRouterState({ select: (state) => state.location.pathname });

  useEffect(() => {
    if (pathname !== "/" && value.isScanning) {
      void value.stopScan();
    }
  }, [pathname, value.isScanning, value.stopScan]);

  return <ReaderContext.Provider value={value}>{children}</ReaderContext.Provider>;
}

export function useReader(): ReaderContextValue {
  const context = useContext(ReaderContext);
  if (!context) {
    throw new Error("useReader must be used within ReaderProvider");
  }
  return context;
}
