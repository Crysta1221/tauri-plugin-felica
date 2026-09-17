import { useEffect, useRef, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { FelicaCard, FelicaError, FelicaReader } from "tauri-plugin-felica-api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useReader } from "@/features/reader-context";

export const Route = createFileRoute("/lab")({
  component: LabPage,
});

function LabPage() {
  const { selectedId, heldBy, setHeldBy } = useReader();
  const [log, setLog] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [reader, setReader] = useState<FelicaReader | null>(null);
  const [card, setCard] = useState<FelicaCard | null>(null);
  const [systemCode, setSystemCode] = useState("0003");
  const [serviceCode, setServiceCode] = useState("008B");
  const [blocks, setBlocks] = useState("0");

  const readerRef = useRef(reader);
  const cardRef = useRef(card);
  readerRef.current = reader;
  cardRef.current = card;

  function append(message: string) {
    setLog((current) => `${current}${current ? "\n" : ""}${message}`);
  }

  async function withBusy(action: () => Promise<void>) {
    setBusy(true);
    try {
      await action();
    } catch (err) {
      const message = err instanceof FelicaError ? `${err.code}: ${err.message}` : String(err);
      append(message);
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    return () => {
      const held = cardRef.current;
      const session = readerRef.current;
      if (held) {
        void held.release().catch(() => undefined);
      }
      if (session) {
        void session.disconnect().catch(() => undefined);
      }
      setHeldBy((current) => (current === "lab" ? null : current));
    };
  }, [setHeldBy]);

  return (
    <div className="mx-auto flex h-full min-h-0 max-w-3xl flex-col gap-4 overflow-hidden p-5">
      <h1 className="shrink-0 font-heading text-lg font-semibold">低レベル確認</h1>
      <Card className="shrink-0">
        <CardHeader>
          <CardTitle className="text-sm">操作</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          <Button
            type="button"
            disabled={busy || !selectedId || heldBy === "scan" || reader !== null}
            onClick={() =>
              void withBusy(async () => {
                if (!selectedId) {
                  append("リーダーを選択してください");
                  return;
                }
                setHeldBy("lab");
                try {
                  const session = await FelicaReader.connect({ id: selectedId });
                  setReader(session);
                  append(`connected ${session.info.name}`);
                } catch (err) {
                  setHeldBy((current) => (current === "lab" ? null : current));
                  throw err;
                }
              })
            }
          >
            接続
          </Button>
          <Button
            type="button"
            disabled={busy || !reader}
            onClick={() =>
              void withBusy(async () => {
                if (!reader) {
                  return;
                }
                const next = await reader.poll();
                setCard(next);
                append(`poll IDm=${next.idm}`);
              })
            }
          >
            poll
          </Button>
          <Button
            type="button"
            disabled={busy || !card}
            onClick={() =>
              void withBusy(async () => {
                if (!card) {
                  return;
                }
                const sc = Number.parseInt(systemCode, 16);
                const res = await card.selectSystem(sc);
                append(`selectSystem ${systemCode} IDm=${res.idm}`);
              })
            }
          >
            selectSystem
          </Button>
          <Button
            type="button"
            disabled={busy || !card}
            onClick={() =>
              void withBusy(async () => {
                if (!card) {
                  return;
                }
                const sc = Number.parseInt(serviceCode, 16);
                const res = await card.requestService([sc]);
                append(JSON.stringify(res));
              })
            }
          >
            requestService
          </Button>
          <Button
            type="button"
            disabled={busy || !card}
            onClick={() =>
              void withBusy(async () => {
                if (!card) {
                  return;
                }
                const sc = Number.parseInt(serviceCode, 16);
                const blockList = blocks
                  .split(/[,\s]+/u)
                  .filter(Boolean)
                  .map((item) => Number.parseInt(item, 10));
                const res = await card.read(sc, blockList);
                append(`blocks=${res.blocks.length} errors=${res.errors.length}`);
              })
            }
          >
            read
          </Button>
          <Button
            type="button"
            disabled={busy || !card}
            onClick={() =>
              void withBusy(async () => {
                if (!card) {
                  return;
                }
                const nodes = await card.searchServices({ max: 32 });
                append(nodes.map((node) => `${node.index}:${node.kind}:0x${node.code.toString(16)}`).join(", "));
              })
            }
          >
            searchServices
          </Button>
          <Button
            type="button"
            disabled={busy || !card}
            onClick={() =>
              void withBusy(async () => {
                if (!card) {
                  return;
                }
                await card.release();
                setCard(null);
                append("released");
              })
            }
          >
            release
          </Button>
          <Button
            type="button"
            variant="outline"
            disabled={busy || !reader}
            onClick={() =>
              void withBusy(async () => {
                if (!reader) {
                  return;
                }
                if (card) {
                  await card.release().catch(() => undefined);
                  setCard(null);
                }
                await reader.disconnect();
                setReader(null);
                setHeldBy(null);
                append("disconnected");
              })
            }
          >
            切断
          </Button>
        </CardContent>
      </Card>
      <div className="grid shrink-0 gap-3 sm:grid-cols-3">
        <Label className="grid gap-1 text-xs">
          システムコード
          <input
            className="border bg-background px-2 py-1 font-mono"
            value={systemCode}
            onChange={(event) => setSystemCode(event.target.value)}
          />
        </Label>
        <Label className="grid gap-1 text-xs">
          サービスコード
          <input
            className="border bg-background px-2 py-1 font-mono"
            value={serviceCode}
            onChange={(event) => setServiceCode(event.target.value)}
          />
        </Label>
        <Label className="grid gap-1 text-xs">
          ブロック
          <input
            className="border bg-background px-2 py-1 font-mono"
            value={blocks}
            onChange={(event) => setBlocks(event.target.value)}
          />
        </Label>
      </div>
      <ScrollArea className="min-h-0 flex-1 border">
        <pre className="p-3 font-mono text-xs whitespace-pre-wrap">{log || "ログはありません"}</pre>
      </ScrollArea>
    </div>
  );
}
