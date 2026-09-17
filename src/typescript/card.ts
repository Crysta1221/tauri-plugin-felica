import { invoke } from "@tauri-apps/api/core";
import { wrapError } from "./error";
import type {
  PollCardResult,
  ReadBlocksResult,
  ServiceNode,
  ServiceStatus,
} from "./types";

export class FelicaCard {
  readonly sessionId: string;
  idm: string;
  pmm: string;
  systemCodes: number[];
  currentSystem?: number;
  private released = false;

  constructor(sessionId: string, poll: PollCardResult, currentSystem?: number) {
    this.sessionId = sessionId;
    this.idm = poll.idm;
    this.pmm = poll.pmm;
    this.systemCodes = poll.systemCodes;
    this.currentSystem = currentSystem ?? poll.systemCodes[0];
  }

  private ensureOpen(): void {
    if (this.released) {
      throw wrapError({ code: "SESSION_CLOSED", message: "Card handle has been released" });
    }
  }

  async requestService(codes: number[]): Promise<ServiceStatus[]> {
    this.ensureOpen();
    try {
      const res = await invoke<{ results: ServiceStatus[] }>("plugin:felica|request_service", {
        payload: { sessionId: this.sessionId, serviceCodes: codes },
      });
      return res.results;
    } catch (err) {
      throw wrapError(err);
    }
  }

  async searchServices(options?: { from?: number; max?: number }): Promise<ServiceNode[]> {
    this.ensureOpen();
    try {
      const res = await invoke<{ nodes: ServiceNode[] }>("plugin:felica|search_services", {
        payload: {
          sessionId: this.sessionId,
          startIndex: options?.from,
          maxNodes: options?.max,
        },
      });
      return res.nodes;
    } catch (err) {
      throw wrapError(err);
    }
  }

  async selectSystem(systemCode: number): Promise<{ idm: string; pmm: string }> {
    this.ensureOpen();
    try {
      const res = await invoke<{ idm: string; pmm: string }>("plugin:felica|select_system", {
        payload: { sessionId: this.sessionId, systemCode },
      });
      this.idm = res.idm;
      this.pmm = res.pmm;
      this.currentSystem = systemCode;
      return res;
    } catch (err) {
      throw wrapError(err);
    }
  }

  async read(serviceCode: number, blocks: number[]): Promise<ReadBlocksResult> {
    this.ensureOpen();
    try {
      return await invoke<ReadBlocksResult>("plugin:felica|read_blocks", {
        payload: {
          sessionId: this.sessionId,
          services: [
            {
              systemCode: this.currentSystem ?? 0,
              serviceCode,
              blocks,
            },
          ],
        },
      });
    } catch (err) {
      throw wrapError(err);
    }
  }

  async release(): Promise<void> {
    if (this.released) {
      return;
    }
    try {
      await invoke("plugin:felica|release_card", {
        payload: { sessionId: this.sessionId },
      });
      this.released = true;
    } catch (err) {
      throw wrapError(err);
    }
  }
}
