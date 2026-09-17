import {
  BUS_PROCESS_TYPES,
  BUS_TERMINAL_TYPES,
  CHARGE_PROCESS_TYPES,
  CYBERNETICS_LINE_OPERATOR_EXTRAS,
  CYBERNETICS_LINE_OPERATORS,
  CYBERNETICS_OPERATOR_RANGES,
  CYBERNETICS_REGION_LABELS,
  CYBERNETICS_STATIONS,
  ENTRY_EXIT_TYPE_LABELS,
  OPERATOR_DISPLAY_NAMES,
  PAYMENT_TYPE_LABELS,
  PROCESS_TYPE_CODE_MASK,
  PROCESS_TYPE_LABELS,
  RETAIL_PROCESS_TYPES,
  RETAIL_TERMINAL_TYPES,
  TERMINAL_TYPE_LABELS,
  regionForLine,
  type CyberneticsOperatorRange,
  type CyberneticsStationRecord,
} from "./maps";

export interface HistoryLookupEntry {
  rawBlockHex: string;
  terminalType: number;
  processType: number;
  regionCode: number;
  entryLineCode?: number;
  entryStationCode?: number;
  exitLineCode?: number;
  exitStationCode?: number;
}

export type CyberneticsRegionCode = number;

export interface CyberneticsStationCode {
  regionCode?: CyberneticsRegionCode;
  lineCode: number;
  stationCode: number;
}

export interface CyberneticsStationInfo {
  companyName: string;
  lineName: string;
  stationName: string;
}

export interface CyberneticsTripSegment {
  entry: CyberneticsStationInfo | null;
  exit: CyberneticsStationInfo | null;
  displayText: string;
}

export interface CyberneticsHistoryCodes {
  terminalType: number;
  processType: number;
  paymentType?: number;
  entryExitType?: number;
  packedRegion: number;
}

const stationByKey = new Map<string, CyberneticsStationRecord>();
for (const record of CYBERNETICS_STATIONS) {
  stationByKey.set(stationKey(record.region, record.line, record.station), record);
}

const operatorRangesByLine = new Map<string, CyberneticsOperatorRange[]>();
for (const range of CYBERNETICS_OPERATOR_RANGES) {
  const key = lineKey(range.region, range.line);
  const list = operatorRangesByLine.get(key);
  if (list) {
    list.push(range);
  } else {
    operatorRangesByLine.set(key, [range]);
  }
}

function lineKey(region: number, line: number): string {
  return `${region}-${line}`;
}

function stationKey(region: number, line: number, station: number): string {
  return `${region}-${line}-${station}`;
}

function displayCompanyName(companyName: string): string {
  return OPERATOR_DISPLAY_NAMES[companyName] ?? companyName;
}

function formatUnknownCode(value: number): string {
  return `0x${value.toString(16).toUpperCase().padStart(2, "0")}`;
}

function parseHexBytes(rawBlockHex: string): number[] | null {
  const hex = rawBlockHex.replaceAll(/\s+/gu, "");
  if (hex.length < 32 || hex.length % 2 !== 0) {
    return null;
  }
  const bytes: number[] = [];
  for (let index = 0; index < hex.length; index += 2) {
    const value = Number.parseInt(hex.slice(index, index + 2), 16);
    if (Number.isNaN(value)) {
      return null;
    }
    bytes.push(value);
  }
  return bytes;
}

export function parseHistoryCodes(entry: HistoryLookupEntry): CyberneticsHistoryCodes {
  const bytes = parseHexBytes(entry.rawBlockHex);
  return {
    terminalType: entry.terminalType,
    processType: entry.processType,
    paymentType: bytes?.[2],
    entryExitType: bytes?.[3],
    packedRegion: entry.regionCode,
  };
}

export function formatTerminalType(terminalType: number): string {
  return TERMINAL_TYPE_LABELS[terminalType] ?? formatUnknownCode(terminalType);
}

export function formatProcessType(processType: number): string {
  const exact = PROCESS_TYPE_LABELS[processType];
  if (exact) {
    return exact;
  }
  const code = processType & PROCESS_TYPE_CODE_MASK;
  const label = PROCESS_TYPE_LABELS[code];
  if (label) {
    if (processType === code) {
      return label;
    }
    return `${label}（併用）`;
  }
  return formatUnknownCode(processType);
}

export function formatPaymentType(paymentType: number): string {
  return PAYMENT_TYPE_LABELS[paymentType] ?? formatUnknownCode(paymentType);
}

export function formatEntryExitType(entryExitType: number): string {
  return ENTRY_EXIT_TYPE_LABELS[entryExitType] ?? formatUnknownCode(entryExitType);
}

export function formatRegion(region: number): string {
  return CYBERNETICS_REGION_LABELS[region] ?? formatUnknownCode(region);
}

function lookupStationRecord(
  lineCode: number,
  stationCode: number,
  region: number,
): CyberneticsStationRecord | null {
  return stationByKey.get(stationKey(region, lineCode, stationCode)) ?? null;
}

function lookupCompanyName(lineCode: number, stationCode: number, region: number): string | null {
  const ranges = operatorRangesByLine.get(lineKey(region, lineCode));
  if (ranges) {
    const match = ranges.find((range) => stationCode >= range.stationFrom && stationCode <= range.stationTo);
    if (match) {
      return match.companyName;
    }
  }

  const lineCompany =
    CYBERNETICS_LINE_OPERATORS[lineKey(region, lineCode)] ??
    CYBERNETICS_LINE_OPERATOR_EXTRAS[lineKey(region, lineCode)];
  if (lineCompany) {
    return lineCompany;
  }

  for (const candidateRegion of [0, 1, 2, 3]) {
    if (candidateRegion === region) {
      continue;
    }
    const extra = CYBERNETICS_LINE_OPERATOR_EXTRAS[lineKey(candidateRegion, lineCode)];
    if (extra) {
      return extra;
    }
    const generated = CYBERNETICS_LINE_OPERATORS[lineKey(candidateRegion, lineCode)];
    if (generated) {
      return generated;
    }
  }

  return null;
}

export interface StationLookup {
  region: number;
  companyName: string;
  lineName: string;
  stationName: string;
}

export function lookupStations(
  lineCode: number,
  stationCode: number,
  region?: number,
): StationLookup[] {
  if (lineCode === 0 && stationCode === 0) {
    return [];
  }
  const regions = region === undefined ? [0, 1, 2, 3] : [region];
  const out: StationLookup[] = [];
  for (const candidate of regions) {
    const record = lookupStationRecord(lineCode, stationCode, candidate);
    const company = lookupCompanyName(lineCode, stationCode, candidate);
    if (!record && !company) {
      continue;
    }
    out.push({
      region: candidate,
      companyName: displayCompanyName(company ?? record?.lineName ?? "不明"),
      lineName: record?.lineName ?? "",
      stationName: record?.stationName ?? "",
    });
  }
  return out;
}

export function stationRefFrom(
  lineCode: number,
  stationCode: number,
  region?: number,
): {
  line: number;
  station: number;
  region?: number;
  label: string;
  candidates?: StationLookup[];
} {
  const candidates = lookupStations(lineCode, stationCode, region);
  let label: string;
  if (candidates.length === 1 && candidates[0].stationName) {
    label = candidates[0].stationName;
  } else if (candidates.length > 1) {
    label = candidates
      .map((item) => item.stationName || `${item.companyName}`)
      .join(" or ");
  } else {
    label = `0x${lineCode.toString(16).toUpperCase().padStart(2, "0")}/0x${stationCode
      .toString(16)
      .toUpperCase()
      .padStart(2, "0")}`;
  }
  return {
    line: lineCode,
    station: stationCode,
    region,
    label,
    candidates: candidates.length > 0 ? candidates : undefined,
  };
}

export function resolveOperatorName(
  lineCode?: number | null,
  stationCode?: number | null,
  packedRegion?: number | null,
  role: "entry" | "exit" = "entry",
): string | null {
  if (lineCode === undefined || lineCode === null || stationCode === undefined || stationCode === null) {
    return null;
  }
  if (lineCode === 0 && stationCode === 0) {
    return null;
  }
  const region = regionForLine(lineCode, packedRegion ?? 0, role);
  const company = lookupCompanyName(lineCode, stationCode, region);
  return company ? displayCompanyName(company) : null;
}

export function resolveStation(
  lineCode?: number | null,
  stationCode?: number | null,
  packedRegion?: number | null,
  role: "entry" | "exit" = "entry",
): CyberneticsStationInfo | null {
  if (lineCode === undefined || lineCode === null || stationCode === undefined || stationCode === null) {
    return null;
  }
  if (lineCode === 0 && stationCode === 0) {
    return null;
  }

  const region = regionForLine(lineCode, packedRegion ?? 0, role);
  const record = lookupStationRecord(lineCode, stationCode, region);
  const company = lookupCompanyName(lineCode, stationCode, region);

  if (!record && !company) {
    return null;
  }

  return {
    companyName: displayCompanyName(company ?? "不明"),
    lineName: record?.lineName ?? "",
    stationName: record?.stationName ?? "",
  };
}

export function formatStationName(
  lineCode?: number | null,
  stationCode?: number | null,
  packedRegion?: number | null,
  role: "entry" | "exit" = "entry",
): string {
  if (lineCode === undefined || lineCode === null || stationCode === undefined || stationCode === null) {
    return "ー";
  }
  if (lineCode === 0 && stationCode === 0) {
    return "ー";
  }

  const station = resolveStation(lineCode, stationCode, packedRegion, role);
  if (station?.stationName) {
    return station.stationName;
  }

  const company = station?.companyName ?? resolveOperatorName(lineCode, stationCode, packedRegion, role);
  const codes = `${lineCode}-${stationCode}`;
  return company ? `${company} ${codes}` : codes;
}

export function isRetailHistory(entry: HistoryLookupEntry): boolean {
  return (
    RETAIL_TERMINAL_TYPES.has(entry.terminalType) || RETAIL_PROCESS_TYPES.has(entry.processType & PROCESS_TYPE_CODE_MASK)
  );
}

export function isBusHistory(entry: HistoryLookupEntry): boolean {
  return BUS_TERMINAL_TYPES.has(entry.terminalType) || BUS_PROCESS_TYPES.has(entry.processType & PROCESS_TYPE_CODE_MASK);
}

export function isBusUsage(terminalType: number, processType: number): boolean {
  return BUS_TERMINAL_TYPES.has(terminalType) || BUS_PROCESS_TYPES.has(processType & PROCESS_TYPE_CODE_MASK);
}

function hasStationPair(lineCode?: number | null, stationCode?: number | null): boolean {
  return (
    lineCode !== undefined &&
    lineCode !== null &&
    stationCode !== undefined &&
    stationCode !== null &&
    !(lineCode === 0 && stationCode === 0)
  );
}

export function formatTripSection(entry: HistoryLookupEntry): string {
  if (isRetailHistory(entry) || isBusHistory(entry) || entry.terminalType === 0x1b) {
    return "ー";
  }

  const hasEntry = hasStationPair(entry.entryLineCode, entry.entryStationCode);
  const hasExit = hasStationPair(entry.exitLineCode, entry.exitStationCode);
  if (!hasEntry && !hasExit) {
    return "ー";
  }

  const processCode = entry.processType & PROCESS_TYPE_CODE_MASK;
  if (CHARGE_PROCESS_TYPES.has(processCode) || processCode === 0x03 || processCode === 0x07) {
    if (!hasEntry) {
      return "ー";
    }
    const place = resolveStation(
      entry.entryLineCode,
      entry.entryStationCode,
      entry.regionCode,
      "entry",
    );
    return place?.stationName || "ー";
  }

  const entryText = hasEntry
    ? formatStationName(entry.entryLineCode, entry.entryStationCode, entry.regionCode, "entry")
    : "ー";
  const exitText = hasExit
    ? formatStationName(entry.exitLineCode, entry.exitStationCode, entry.regionCode, "exit")
    : "ー";

  if (entryText === "ー" && exitText === "ー") {
    return "ー";
  }

  return `${entryText} → ${exitText}`;
}
