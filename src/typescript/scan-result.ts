import { enrichScanResult } from "./enrich";
import type {
  BlockData,
  BlockReadError,
  EdyCard,
  FelicaType,
  LiteCard,
  NanacoCard,
  QuicpayCard,
  ScanResultDto,
  TransitCard,
  WaonCard,
} from "./types";

export class ScanResult {
  readonly idm: string;
  readonly pmm: string;
  readonly systemCodes: number[];
  readonly transit?: TransitCard;
  readonly waon?: WaonCard;
  readonly edy?: EdyCard;
  readonly nanaco?: NanacoCard;
  readonly quicpay?: QuicpayCard;
  readonly lite?: LiteCard;
  readonly blocks: BlockData[];
  readonly errors: BlockReadError[];

  constructor(dto: ScanResultDto) {
    const enriched = enrichScanResult(dto);
    this.idm = enriched.idm;
    this.pmm = enriched.pmm;
    this.systemCodes = enriched.systemCodes;
    this.transit = enriched.transit;
    this.waon = enriched.waon;
    this.edy = enriched.edy;
    this.nanaco = enriched.nanaco;
    this.quicpay = enriched.quicpay;
    this.lite = enriched.lite;
    this.blocks = enriched.blocks;
    this.errors = enriched.errors;
  }

  get products(): FelicaType[] {
    const out: FelicaType[] = [];
    if (this.transit) {
      out.push("transit");
    }
    if (this.waon) {
      out.push("waon");
    }
    if (this.edy) {
      out.push("edy");
    }
    if (this.nanaco) {
      out.push("nanaco");
    }
    if (this.quicpay) {
      out.push("quicpay");
    }
    if (this.lite) {
      out.push("lite");
    }
    return out;
  }
}
