import { useState } from "react";
import { CreditCardIcon, NfcIcon, UsbIcon } from "lucide-react";
import type {
  BlockData,
  BlockReadError,
  FelicaType,
  LiteCard,
  ScanResult,
  TransitHistoryEntry,
  WaonHistoryEntry,
} from "tauri-plugin-felica-api";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useReader } from "@/features/reader-context";
import { formatAmount, formatIdm, formatSystemCode, formatTransitRoute, formatYen } from "./format";
import { productLabel } from "./use-reader-scan";

export function ReaderScanPage() {
  const { readers, scan, errorMessage, isListing, isScanning, targets, setTargets, allTargets } =
    useReader();
  const [hideCardIdentifiers, setHideCardIdentifiers] = useState(false);

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden bg-background text-foreground">
      <div className="flex shrink-0 flex-wrap gap-1.5 border-b px-5 py-2">
        {allTargets.map((type) => {
          const selected = targets.includes(type);
          return (
            <Button
              key={type}
              type="button"
              size="sm"
              variant={selected ? "secondary" : "outline"}
              className="h-7 px-2 text-xs"
              disabled={isScanning}
              onClick={() => {
                setTargets((current) =>
                  current.includes(type) ? current.filter((item) => item !== type) : [...current, type],
                );
              }}
            >
              {productLabel(type)}
            </Button>
          );
        })}
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 divide-y overflow-hidden md:grid-cols-2 md:grid-rows-1 md:divide-x md:divide-y-0">
        <section className="flex min-h-0 flex-col gap-3 overflow-hidden p-5 pb-[max(1.25rem,env(safe-area-inset-bottom))]">
          <h2 className="shrink-0 font-heading text-sm font-semibold">カード情報</h2>
          {isScanning ? (
            <Alert>
              <NfcIcon />
              <AlertTitle>カード待ち</AlertTitle>
              <AlertDescription>リーダーに FeliCa カードをかざしてください。</AlertDescription>
            </Alert>
          ) : null}

          {errorMessage ? (
            <Alert variant="destructive">
              <AlertTitle>読み取りエラー</AlertTitle>
              <AlertDescription>{errorMessage}</AlertDescription>
            </Alert>
          ) : null}

          {scan ? (
            <ScrollArea className="min-h-0 flex-1">
              <ScanSummary scan={scan} hideCardIdentifiers={hideCardIdentifiers} onHideChange={setHideCardIdentifiers} />
            </ScrollArea>
          ) : isListing && readers.length === 0 ? (
            <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto">
              <Skeleton className="h-36 w-full" />
              <Skeleton className="h-48 w-full" />
            </div>
          ) : (
            <Empty className="min-h-0 flex-1 border">
              <EmptyHeader>
                <EmptyMedia variant="icon">
                  {isScanning ? (
                    <NfcIcon className="animate-pulse" />
                  ) : readers.length > 0 ? (
                    <CreditCardIcon />
                  ) : (
                    <UsbIcon />
                  )}
                </EmptyMedia>
                <EmptyTitle className="text-balance">
                  {isScanning ? "カードをかざしてください" : readers.length > 0 ? "スキャンを開始" : "リーダーが見つかりません"}
                </EmptyTitle>
                <EmptyDescription className="text-pretty">
                  {isScanning
                    ? "FeliCa カードをリーダーにかざしてください。"
                    : readers.length > 0
                      ? "上の「スキャン開始」ボタンを押してカードをかざしてください。"
                      : "PaSoRi を接続して「再読み込み」を押してください。"}
                </EmptyDescription>
              </EmptyHeader>
            </Empty>
          )}
        </section>

        <section className="flex min-h-0 flex-col gap-3 overflow-hidden p-5 pb-[max(1.25rem,env(safe-area-inset-bottom))]">
          <h2 className="shrink-0 font-heading text-sm font-semibold">履歴 / 生データ</h2>
          <RightPane scan={scan} />
        </section>
      </div>
    </div>
  );
}

function ScanSummary({
  scan,
  hideCardIdentifiers,
  onHideChange,
}: {
  scan: ScanResult;
  hideCardIdentifiers: boolean;
  onHideChange: (value: boolean) => void;
}) {
  const balances = [
    scan.transit?.balance !== undefined ? { key: "transit" as FelicaType, value: scan.transit.balance } : null,
    scan.waon ? { key: "waon" as FelicaType, value: scan.waon.balance } : null,
    scan.edy ? { key: "edy" as FelicaType, value: scan.edy.balance } : null,
    scan.nanaco ? { key: "nanaco" as FelicaType, value: scan.nanaco.balance } : null,
  ].filter((item): item is { key: FelicaType; value: number } => item !== null);

  return (
    <div className="flex flex-col gap-5">
      {balances.length > 0 ? (
        <div className="grid gap-3">
          {balances.map((item) => (
            <Card key={item.key}>
              <CardHeader className="border-b">
                <CardTitle className="text-balance">{productLabel(item.key)} 残高</CardTitle>
              </CardHeader>
              <CardContent className="pt-6">
                <p className="font-heading text-4xl font-bold tracking-tight tabular-nums">{formatYen(item.value)}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : null}

      <Card>
        <CardHeader className="border-b pb-3">
          <CardTitle className="flex items-center gap-3 text-sm">
            カード情報
            <Label className="font-normal text-muted-foreground">
              <input
                type="checkbox"
                className="size-3.5 accent-foreground"
                checked={hideCardIdentifiers}
                onChange={(event) => onHideChange(event.target.checked)}
              />
              情報を隠す
            </Label>
          </CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3 pt-4 text-xs">
          <InfoRow label="IDm" value={hideCardIdentifiers ? "•••• •••• •••• ••••" : formatIdm(scan.idm)} mono />
          <InfoRow label="PMm" value={hideCardIdentifiers ? "•••• •••• •••• ••••" : formatIdm(scan.pmm)} mono />
          <div className="flex items-start justify-between border-b pb-2">
            <span className="text-muted-foreground">システムコード</span>
            <div className="flex max-w-[70%] flex-wrap justify-end gap-1">
              {scan.systemCodes.map((code) => (
                <Badge key={code} variant="outline" className="font-mono font-normal">
                  {formatSystemCode(code)}
                </Badge>
              ))}
            </div>
          </div>
          <div className="flex items-start justify-between border-b pb-2">
            <span className="text-muted-foreground">識別</span>
            <div className="flex max-w-[70%] flex-wrap justify-end gap-1">
              {scan.products.map((type) => (
                <Badge key={type} variant="secondary">
                  {productLabel(type)}
                </Badge>
              ))}
            </div>
          </div>
          {scan.products.map((type) => {
            const product =
              type === "transit"
                ? scan.transit
                : type === "waon"
                  ? scan.waon
                  : type === "edy"
                    ? scan.edy
                    : type === "nanaco"
                      ? scan.nanaco
                      : type === "quicpay"
                        ? scan.quicpay
                        : scan.lite;
            if (!product) {
              return null;
            }
            return (
              <InfoRow
                key={`${type}-idm`}
                label={`${productLabel(type)} IDm`}
                value={
                  hideCardIdentifiers
                    ? "••••"
                    : `${formatSystemCode(product.systemCode)} / ${formatIdm(product.idm)}`
                }
                mono
              />
            );
          })}
          {scan.transit?.settings ? (
            <div className="flex flex-wrap justify-end gap-1">
              {scan.transit.settings.touchDeGo ? <Badge variant="outline">タッチでGo</Badge> : null}
              {scan.transit.settings.voiceGuidance ? <Badge variant="outline">音声案内</Badge> : null}
              {scan.transit.settings.sfOutsideCommuter ? <Badge variant="outline">SF外定期</Badge> : null}
            </div>
          ) : null}
        </CardContent>
      </Card>
    </div>
  );
}

function InfoRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-center justify-between border-b pb-2">
      <span className="text-muted-foreground">{label}</span>
      <span className={mono ? "font-mono" : undefined}>{value}</span>
    </div>
  );
}

function RightPane({ scan }: { scan: ScanResult | null }) {
  if (!scan) {
    return (
      <Empty className="min-h-0 flex-1 border">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <CreditCardIcon />
          </EmptyMedia>
          <EmptyTitle>データはありません</EmptyTitle>
          <EmptyDescription>カードを読み取ると、履歴または生ブロックが表示されます。</EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  }

  return (
    <ScrollArea className="min-h-0 flex-1 rounded-md border">
      {scan.transit && scan.transit.histories.length > 0 ? (
        <TransitHistoryTable rows={scan.transit.histories} />
      ) : null}
      {scan.waon && scan.waon.histories.length > 0 ? <WaonHistoryTable rows={scan.waon.histories} /> : null}
      {scan.edy && scan.edy.histories.length > 0 ? (
        <SimpleHistory title="楽天Edy" rows={scan.edy.histories} />
      ) : null}
      {scan.nanaco && scan.nanaco.histories.length > 0 ? (
        <SimpleHistory title="nanaco" rows={scan.nanaco.histories} />
      ) : null}
      {scan.quicpay ? (
        <Empty className="border-0">
          <EmptyHeader>
            <EmptyTitle>QUICPay</EmptyTitle>
            <EmptyDescription>公開ブロックはありません。</EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : null}
      {scan.lite ? <LitePads card={scan.lite} /> : null}
      <RawBlocks blocks={scan.blocks} errors={scan.errors} />
    </ScrollArea>
  );
}

function TransitHistoryTable({ rows }: { rows: TransitHistoryEntry[] }) {
  return (
    <div>
      <div className="border-b px-3 py-2 text-xs font-medium text-muted-foreground">交通系IC</div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>日時</TableHead>
            <TableHead>処理</TableHead>
            <TableHead>端末</TableHead>
            <TableHead>区間</TableHead>
            <TableHead className="text-right">金額</TableHead>
            <TableHead className="text-right">残高</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((entry) => (
            <TableRow key={`${entry.blockIndex}-${entry.seqNumber}`}>
              <TableCell className="whitespace-nowrap tabular-nums">
                {entry.time ? `${entry.date} ${entry.time}` : entry.date}
              </TableCell>
              <TableCell>
                <Badge variant="outline" className="font-normal">
                  {entry.processType.label}
                </Badge>
              </TableCell>
              <TableCell className="whitespace-nowrap">{entry.terminalType.label}</TableCell>
              <TableCell className="whitespace-nowrap">{formatTransitRoute(entry)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatAmount(entry.amount)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatYen(entry.balance)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

function WaonHistoryTable({ rows }: { rows: WaonHistoryEntry[] }) {
  return (
    <div>
      <div className="border-b px-3 py-2 text-xs font-medium text-muted-foreground">WAON</div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>日時</TableHead>
            <TableHead>利用</TableHead>
            <TableHead>チャージ</TableHead>
            <TableHead className="text-right">残高</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((entry) => (
            <TableRow key={`${entry.blockIndex}-${entry.raw}`}>
              <TableCell className="whitespace-nowrap tabular-nums">
                {entry.dateTime ? `${entry.dateTime.date} ${entry.dateTime.time}` : "ー"}
              </TableCell>
              <TableCell>{entry.amount !== undefined ? formatYen(entry.amount) : "ー"}</TableCell>
              <TableCell>{entry.chargeAmount !== undefined ? formatYen(entry.chargeAmount) : "ー"}</TableCell>
              <TableCell className="text-right tabular-nums">
                {entry.balance !== undefined ? formatYen(entry.balance) : "ー"}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

function SimpleHistory({
  title,
  rows,
}: {
  title: string;
  rows: { rawBlockHex: string; date?: string; time?: string; amount?: number; balance?: number }[];
}) {
  return (
    <div>
      <div className="border-b px-3 py-2 text-xs font-medium text-muted-foreground">{title}</div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>日時</TableHead>
            <TableHead className="text-right">金額</TableHead>
            <TableHead className="text-right">残高</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((entry, index) => (
            <TableRow key={`${title}-${index}-${entry.rawBlockHex}`}>
              <TableCell className="whitespace-nowrap tabular-nums">
                {entry.date ? `${entry.date}${entry.time ? ` ${entry.time}` : ""}` : "ー"}
              </TableCell>
              <TableCell className="text-right">{entry.amount !== undefined ? formatYen(entry.amount) : "ー"}</TableCell>
              <TableCell className="text-right">{entry.balance !== undefined ? formatYen(entry.balance) : "ー"}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

function LitePads({ card }: { card: LiteCard }) {
  const entries = Object.entries(card.sPad);
  if (entries.length === 0) {
    return null;
  }
  return (
    <div>
      <div className="border-b px-3 py-2 text-xs font-medium text-muted-foreground">Lite S_PAD</div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>パッド</TableHead>
            <TableHead>データ</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {entries.map(([index, hex]) => (
            <TableRow key={index}>
              <TableCell className="tabular-nums">{index}</TableCell>
              <TableCell className="font-mono text-xs break-all">{hex}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

function RawBlocks({ blocks, errors }: { blocks: BlockData[]; errors: BlockReadError[] }) {
  return (
    <div>
      <div className="border-b px-3 py-2 text-xs font-medium text-muted-foreground">生データ</div>
      {errors.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>エラー</TableHead>
              <TableHead>ブロック</TableHead>
              <TableHead>理由</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {errors.map((error) => (
              <TableRow key={`${error.systemCode}-${error.serviceCode}-${error.blockIndex}-${error.reason}`}>
                <TableCell className="font-mono">
                  {formatSystemCode(error.systemCode)}/{formatSystemCode(error.serviceCode)}
                </TableCell>
                <TableCell>{error.blockIndex}</TableCell>
                <TableCell>{error.reason}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>システム</TableHead>
            <TableHead>サービス</TableHead>
            <TableHead>ブロック</TableHead>
            <TableHead>データ</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {blocks.map((block) => (
            <TableRow key={`${block.systemCode}-${block.serviceCode}-${block.blockIndex}-${block.dataHex}`}>
              <TableCell className="font-mono">{formatSystemCode(block.systemCode)}</TableCell>
              <TableCell className="font-mono">{formatSystemCode(block.serviceCode)}</TableCell>
              <TableCell className="tabular-nums">{block.blockIndex}</TableCell>
              <TableCell className="font-mono text-xs break-all">{block.dataHex}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
