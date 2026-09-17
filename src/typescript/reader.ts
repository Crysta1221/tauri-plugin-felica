import { invoke } from "@tauri-apps/api/core";
import { FelicaCard } from "./card";
import { FelicaError, wrapError } from "./error";
import { ScanResult } from "./scan-result";
import type {
  ConnectOptions,
  PollCardResult,
  ReadBlocksResult,
  ReaderInfo,
  ScanOptions,
  ScanResultDto,
  ServiceRead,
} from "./types";

export class FelicaReader {
  readonly sessionId: string;
  readonly info: ReaderInfo;

  constructor(sessionId: string, info: ReaderInfo) {
    this.sessionId = sessionId;
    this.info = info;
  }

  static async list(): Promise<ReaderInfo[]> {
    try {
      return await invoke<ReaderInfo[]>("plugin:felica|list_readers");
    } catch (err) {
      throw wrapError(err);
    }
  }

  static async connect(options?: ConnectOptions): Promise<FelicaReader> {
    try {
      const res = await invoke<{ sessionId: string; reader: ReaderInfo }>(
        "plugin:felica|connect_reader",
        {
          payload: {
            id: options?.id,
            preference: options?.preference,
          },
        },
      );
      return new FelicaReader(res.sessionId, res.reader);
    } catch (err) {
      throw wrapError(err);
    }
  }

  async scan(options?: ScanOptions): Promise<ScanResult> {
    if (options?.signal?.aborted) {
      throw new FelicaError("SCAN_CANCELLED", "Scan operation was cancelled by AbortSignal");
    }

    let onAbort: (() => void) | undefined;
    if (options?.signal) {
      onAbort = () => {
        invoke("plugin:felica|cancel_scan", {
          payload: { sessionId: this.sessionId },
        }).catch(() => {
          // Ignore error during abort signal delivery
        });
      };
      options.signal.addEventListener("abort", onAbort, { once: true });
    }

    try {
      const response = await invoke<ScanResultDto>("plugin:felica|scan_card", {
        payload: {
          sessionId: this.sessionId,
          timeoutMs: options?.timeoutMs,
          targets: options?.targets,
          require: options?.require,
          detail: options?.detail,
        },
      });
      return new ScanResult(response);
    } catch (err) {
      throw wrapError(err);
    } finally {
      if (options?.signal && onAbort) {
        options.signal.removeEventListener("abort", onAbort);
      }
    }
  }

  async read(services: ServiceRead[], timeoutMs?: number): Promise<ReadBlocksResult> {
    try {
      return await invoke<ReadBlocksResult>("plugin:felica|read_blocks", {
        payload: {
          sessionId: this.sessionId,
          timeoutMs,
          services,
        },
      });
    } catch (err) {
      throw wrapError(err);
    }
  }

  async poll(systemCode?: number, timeoutMs?: number): Promise<FelicaCard> {
    try {
      const response = await invoke<PollCardResult>("plugin:felica|poll_card", {
        payload: {
          sessionId: this.sessionId,
          timeoutMs,
          systemCode,
        },
      });
      return new FelicaCard(this.sessionId, response, systemCode);
    } catch (err) {
      throw wrapError(err);
    }
  }

  async cancelScan(): Promise<void> {
    try {
      await invoke("plugin:felica|cancel_scan", {
        payload: { sessionId: this.sessionId },
      });
    } catch (err) {
      throw wrapError(err);
    }
  }

  async disconnect(): Promise<void> {
    try {
      await invoke("plugin:felica|disconnect_reader", {
        payload: { sessionId: this.sessionId },
      });
    } catch (err) {
      throw wrapError(err);
    }
  }
}
