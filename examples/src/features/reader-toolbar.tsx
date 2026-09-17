import { CircleStopIcon, PlayIcon, RefreshCwIcon } from "lucide-react";
import { useRouterState } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Spinner } from "@/components/ui/spinner";
import { useReader } from "@/features/reader-context";

export function ReaderToolbar() {
  const {
    readers,
    selectedId,
    setSelectedId,
    isListing,
    isScanning,
    detail,
    setDetail,
    targets,
    refreshReaders,
    startScan,
    stopScan,
    heldBy,
  } = useReader();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const isScan = pathname === "/";
  const deviceHeld = heldBy !== null;
  const selectItems = Object.fromEntries(
    readers.map((reader) => [reader.id, `${reader.name} · ${reader.chipset}`]),
  );

  return (
    <header className="flex shrink-0 flex-wrap items-center gap-3 border-b px-5 py-3 pt-[max(0.75rem,env(safe-area-inset-top))] pr-[max(1.25rem,env(safe-area-inset-right))] pl-[max(1.25rem,env(safe-area-inset-left))]">
      <span className="shrink-0 text-sm font-medium text-muted-foreground">リーダー:</span>
      <div className="min-w-0 flex-1">
        <Select
          value={selectedId ?? undefined}
          onValueChange={(value) => {
            setSelectedId(value ?? null);
          }}
          items={selectItems}
          disabled={deviceHeld}
        >
          <SelectTrigger className="w-full min-w-0" aria-label="リーダーを選択">
            <SelectValue
              placeholder={readers.length === 0 ? "リーダーがありません" : "リーダーを選択"}
            />
          </SelectTrigger>
          <SelectContent alignItemWithTrigger={false} align="start" className="min-w-(--anchor-width)">
            {readers.length === 0 ? (
              <div className="p-3 text-center text-xs text-muted-foreground">
                利用可能なリーダーがありません
              </div>
            ) : (
              readers.map((reader) => (
                <SelectItem key={reader.id} value={reader.id}>
                  {reader.name}
                </SelectItem>
              ))
            )}
          </SelectContent>
        </Select>
      </div>

      {isScan ? (
        <Label className="flex shrink-0 items-center gap-1.5 text-xs text-muted-foreground">
          <input
            type="checkbox"
            className="size-3.5 accent-foreground"
            checked={detail}
            onChange={(event) => setDetail(event.target.checked)}
            disabled={isScanning}
          />
          詳細読取
        </Label>
      ) : null}

      <Button
        type="button"
        variant="outline"
        className="shrink-0"
        onClick={() => {
          void refreshReaders();
        }}
        disabled={deviceHeld || isListing}
        aria-label="リーダー一覧を再読み込み"
      >
        {isListing ? <Spinner data-icon="inline-start" /> : <RefreshCwIcon data-icon="inline-start" />}
        再読み込み
      </Button>

      {isScan ? (
        isScanning ? (
          <Button type="button" variant="destructive" className="shrink-0" onClick={() => void stopScan()}>
            <CircleStopIcon data-icon="inline-start" />
            停止
          </Button>
        ) : (
          <Button
            type="button"
            className="shrink-0"
            onClick={() => void startScan()}
            disabled={!selectedId || isListing || targets.length === 0 || heldBy === "lab"}
          >
            <PlayIcon data-icon="inline-start" />
            スキャン開始
          </Button>
        )
      ) : null}
    </header>
  );
}
