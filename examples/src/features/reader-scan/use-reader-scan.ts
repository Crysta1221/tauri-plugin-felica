import { useCallback, useEffect, useRef, useState } from "react";
import {
  FelicaError,
  FelicaReader,
  FelicaTypes,
  type FelicaType,
  type ScanResult,
  type ReaderInfo,
} from "tauri-plugin-felica-api";

export type ScanPhase = "idle" | "listing" | "ready" | "scanning" | "error";

export interface ReaderScanState {
  phase: ScanPhase;
  readers: ReaderInfo[];
  selectedId: string | null;
  scan: ScanResult | null;
  errorMessage: string | null;
  isListing: boolean;
  isScanning: boolean;
  detail: boolean;
  targets: FelicaType[];
}

export type ReaderHold = "scan" | "lab" | null;

const ALL_TARGETS: FelicaType[] = [
  FelicaTypes.TRANSIT,
  FelicaTypes.WAON,
  FelicaTypes.EDY,
  FelicaTypes.NANACO,
  FelicaTypes.QUICPAY,
  FelicaTypes.LITE,
];

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function formatSystemCodes(codes?: number[]): string {
  if (!codes || codes.length === 0) {
    return "";
  }
  return codes.map((code) => `0x${code.toString(16).toUpperCase().padStart(4, "0")}`).join(", ");
}

function productLabel(type: FelicaType): string {
  switch (type) {
    case "transit":
      return "交通系IC";
    case "waon":
      return "WAON";
    case "edy":
      return "楽天Edy";
    case "nanaco":
      return "nanaco";
    case "quicpay":
      return "QUICPay";
    case "lite":
      return "Lite";
    default:
      return type;
  }
}

function errorMessage(err: unknown): string {
  if (!isTauriRuntime()) {
    return "この画面は Tauri ホスト内でのみリーダーに接続できます。examples で pnpm tauri dev を実行してください。";
  }
  if (err instanceof FelicaError) {
    switch (err.code) {
      case "DEVICE_NOT_FOUND":
        return err.message
          ? `リーダーを開けませんでした。${err.message}`
          : "リーダーが見つかりません。USB 接続を確認して再読み込みしてください。";
      case "DEVICE_BUSY":
        return "リーダーが使用中です。他のアプリを閉じてから再試行してください。";
      case "DEVICE_ACCESS_DENIED":
        return "リーダーの WinUSB 面を開けませんでした。Sony 公式ドライバはそのままで構いません。診断ツールで「RC-S300/P WinUSB」が○か確認し、他の FeliCa アプリを閉じてから再試行してください。";
      case "DEVICE_DISCONNECTED":
        return "スキャン中にリーダーが切断されました。";
      case "SCAN_TIMEOUT":
        return "カード待ちがタイムアウトしました。";
      case "SCAN_CANCELLED":
        return "スキャンを停止しました。";
      case "UNSUPPORTED_CARD":
        return err.systemCodes?.length
          ? `対応していないカードです。システムコード: ${formatSystemCodes(err.systemCodes)}`
          : "このカードは既知の無鍵プロファイルではありません。";
      case "CARD_TYPE_MISMATCH":
        return `指定した製品ではありません。検出: ${(err.detected ?? []).map(productLabel).join(", ") || "なし"}`;
      case "SESSION_CLOSED":
        return "リーダー接続が閉じられました。もう一度開始してください。";
      case "INTERNAL_ERROR":
        return err.message ? `内部エラー: ${err.message}` : "内部エラーが発生しました。";
      default:
        return err.message;
    }
  }
  if (err instanceof Error) {
    return err.message;
  }
  return "不明なエラーが発生しました。";
}

export function useReaderScan() {
  const [readers, setReaders] = useState<ReaderInfo[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [errorMessageState, setErrorMessageState] = useState<string | null>(null);
  const [isListing, setIsListing] = useState(false);
  const [isScanning, setIsScanning] = useState(false);
  const [detail, setDetail] = useState(false);
  const [targets, setTargets] = useState<FelicaType[]>(ALL_TARGETS);

  const [heldBy, setHeldBy] = useState<ReaderHold>(null);

  const sessionRef = useRef<FelicaReader | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const loopIdRef = useRef(0);

  const refreshReaders = useCallback(async () => {
    if (!isTauriRuntime()) {
      setErrorMessageState(errorMessage(undefined));
      setReaders([]);
      return;
    }
    setIsListing(true);
    try {
      const next = await FelicaReader.list();
      setReaders(next);
      setSelectedId((current) => {
        if (current && next.some((reader) => reader.id === current)) {
          return current;
        }
        return next[0]?.id ?? null;
      });
      setErrorMessageState(null);
    } catch (err) {
      setErrorMessageState(errorMessage(err));
    } finally {
      setIsListing(false);
    }
  }, []);

  const stopScan = useCallback(async () => {
    abortRef.current?.abort();
    abortRef.current = null;
    const session = sessionRef.current;
    sessionRef.current = null;
    if (!session) {
      setHeldBy((current) => (current === "scan" ? null : current));
      setIsScanning(false);
      return;
    }
    try {
      await session.cancelScan();
    } catch {
      // Cancel is best-effort while tearing down.
    }
    try {
      await session.disconnect();
    } catch {
      // Disconnect is best-effort while tearing down.
    }
    setHeldBy((current) => (current === "scan" ? null : current));
    setIsScanning(false);
  }, []);

  const startScan = useCallback(async () => {
    if (isScanning || !selectedId || heldBy === "lab") {
      return;
    }

    setScan(null);
    setErrorMessageState(null);
    setHeldBy("scan");
    setIsScanning(true);
    const loopId = loopIdRef.current + 1;
    loopIdRef.current = loopId;
    const abort = new AbortController();
    abortRef.current = abort;

    try {
      const session = await FelicaReader.connect({ id: selectedId });
      if (abort.signal.aborted || loopId !== loopIdRef.current) {
        await session.disconnect();
        return;
      }
      sessionRef.current = session;

      try {
        const allSelected = ALL_TARGETS.every((item) => targets.includes(item));
        const next = await session.scan({
          signal: abort.signal,
          detail,
          targets: allSelected ? undefined : targets,
        });
        if (!abort.signal.aborted && loopId === loopIdRef.current) {
          setScan(next);
          setErrorMessageState(null);
        }
      } catch (err) {
        if (!(err instanceof FelicaError && err.code === "SCAN_CANCELLED")) {
          setErrorMessageState(errorMessage(err));
        }
      }
    } catch (err) {
      setErrorMessageState(errorMessage(err));
    } finally {
      if (loopId === loopIdRef.current) {
        const session = sessionRef.current;
        sessionRef.current = null;
        abortRef.current = null;
        if (session) {
          try {
            await session.disconnect();
          } catch {
            // Ignore disconnect errors after the loop ends.
          }
        }
        setHeldBy((current) => (current === "scan" ? null : current));
        setIsScanning(false);
      }
    }
  }, [detail, heldBy, isScanning, selectedId, targets]);

  useEffect(() => {
    void refreshReaders();
    return () => {
      loopIdRef.current += 1;
      abortRef.current?.abort();
      const session = sessionRef.current;
      sessionRef.current = null;
      if (session) {
        void session.disconnect();
      }
    };
  }, [refreshReaders]);

  let phase: ScanPhase = "ready";
  if (isScanning) {
    phase = "scanning";
  } else if (isListing && readers.length === 0) {
    phase = "listing";
  } else if (errorMessageState && readers.length === 0) {
    phase = "error";
  } else if (readers.length === 0) {
    phase = "idle";
  }

  return {
    phase,
    readers,
    selectedId,
    scan,
    errorMessage: errorMessageState,
    isListing,
    isScanning,
    detail,
    setDetail,
    targets,
    setTargets,
    allTargets: ALL_TARGETS,
    setSelectedId,
    refreshReaders,
    startScan,
    stopScan,
    heldBy,
    setHeldBy,
  };
}

export { productLabel };
