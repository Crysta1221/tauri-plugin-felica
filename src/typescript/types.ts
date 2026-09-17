export const FelicaTypes = {
  TRANSIT: "transit",
  WAON: "waon",
  EDY: "edy",
  NANACO: "nanaco",
  QUICPAY: "quicpay",
  LITE: "lite",
} as const;
export type FelicaType = (typeof FelicaTypes)[keyof typeof FelicaTypes];

export interface ReaderInfo {
  id: string;
  name: string;
  vendorId: number;
  productId: number;
  chipset: string;
}

export type ReaderPreference = "auto" | "rc_s380" | "rc_s300" | "rc_s320" | "rc_s330";

export interface ConnectOptions {
  id?: string;
  preference?: ReaderPreference;
}

export interface ScanOptions {
  targets?: FelicaType[];
  require?: boolean;
  detail?: boolean;
  timeoutMs?: number;
  signal?: AbortSignal;
}

export interface ServiceRead {
  systemCode: number;
  serviceCode: number;
  blocks: number[];
}

export interface BlockData {
  systemCode: number;
  serviceCode: number;
  blockIndex: number;
  dataHex: string;
}

export type BlockReadReason = "STATUS_FLAG" | "COMMUNICATION" | "PROTOCOL" | "WRONG_IDM";

export interface BlockReadError {
  systemCode: number;
  serviceCode: number;
  blockIndex: number;
  reason: BlockReadReason;
  statusFlag1?: number;
  statusFlag2?: number;
}

export interface ReadBlocksResult {
  blocks: BlockData[];
  errors: BlockReadError[];
}

export interface LabeledCode {
  code: number;
  label: string;
}

export interface StationCandidate {
  region: number;
  companyName: string;
  lineName: string;
  stationName: string;
}

export interface StationRef {
  line: number;
  station: number;
  region?: number;
  label: string;
  candidates?: StationCandidate[];
}

export interface ServiceStatus {
  serviceCode: number;
  keyVersion: number;
  exists: boolean;
}

export interface ServiceNode {
  index: number;
  code: number;
  kind: "area" | "service";
}

export interface TransitSettings {
  raw: number;
  touchDeGo: boolean;
  voiceGuidance: boolean;
  sfOutsideCommuter: boolean;
}

export interface TransitGate {
  hasRecord: boolean;
  entry?: StationRef;
  intermediate?: {
    entryDate?: string;
    entryTime?: string;
    entry?: StationRef;
    exitTime?: string;
    exit?: StationRef;
    unknown1Hex?: string;
    unknown2Hex?: string;
  };
}

export interface TransitHistoryEntry {
  rawBlockHex: string;
  blockIndex: number;
  terminalType: LabeledCode;
  processTypeRaw: number;
  processType: LabeledCode;
  paymentType: LabeledCode;
  gateInstructionType: LabeledCode;
  date: string;
  time?: string;
  entry?: StationRef;
  exit?: StationRef;
  busCompanyCode?: number;
  busStopCode?: number;
  busCompanyName?: string;
  balance: number;
  amount?: number | null;
  seqNumber: number;
  entryRegion: number;
  exitRegion: number;
}

export interface TransitGateRecord {
  rawBlockHex: string;
  blockIndex: number;
  entryExitType: LabeledCode;
  intermediateInstructionType: LabeledCode;
  station: StationRef;
  equipmentId: number;
  date: string;
  time: string;
  amount: number;
  commuterFare: number;
  commuterStation: StationRef;
}

export interface PaidTicket {
  rawBlockHex: string;
  blockIndex: number;
  origin: StationRef;
  destination: StationRef;
  expiresAt: string;
  issuedAt: string;
  issueType: number;
  amount: number;
  equipmentId: number;
  gateStation: StationRef;
}

export interface TransitCard {
  type: "transit";
  systemCode: number;
  idm: string;
  balance?: number;
  seqNumber?: number;
  settings?: TransitSettings;
  gate: TransitGate;
  histories: TransitHistoryEntry[];
  gateRecords?: TransitGateRecord[];
  paidTickets?: PaidTicket[];
}

export interface WaonHistoryEntry {
  raw: string;
  blockIndex: number;
  terminalId?: string;
  seqNumber?: number;
  typeCode?: number;
  dateTime?: { date: string; time: string };
  amount?: number;
  chargeAmount?: number;
  balance?: number;
}

export interface WaonCard {
  type: "waon";
  systemCode: number;
  idm: string;
  balance: number;
  points?: number;
  waonNumber?: string;
  histories: WaonHistoryEntry[];
}

export interface EmoneyHistoryEntry {
  rawBlockHex: string;
  typeCode?: number;
  amount?: number;
  balance?: number;
  date?: string;
  time?: string;
}

export interface EdyCard {
  type: "edy";
  systemCode: number;
  idm: string;
  balance: number;
  edyNumber?: string;
  histories: EmoneyHistoryEntry[];
}

export interface NanacoCard {
  type: "nanaco";
  systemCode: number;
  idm: string;
  balance: number;
  points?: number;
  nanacoNumber?: string;
  histories: EmoneyHistoryEntry[];
}

export interface QuicpayCard {
  type: "quicpay";
  systemCode: number;
  idm: string;
}

export interface LiteCard {
  type: "lite";
  systemCode: number;
  idm: string;
  sPad: Record<number, string>;
}

export interface ScanResultDto {
  idm: string;
  pmm: string;
  systemCodes: number[];
  transit?: TransitCard;
  waon?: WaonCard;
  edy?: EdyCard;
  nanaco?: NanacoCard;
  quicpay?: QuicpayCard;
  lite?: LiteCard;
  blocks: BlockData[];
  errors: BlockReadError[];
}

export interface PollCardResult {
  idm: string;
  pmm: string;
  systemCodes: number[];
}
