import {
  formatEntryExitType,
  formatPaymentType,
  formatProcessType,
  formatTerminalType,
  stationRefFrom,
} from "./labels/cybernetics";
import { BUS_COMPANY_LABELS } from "./labels/maps/bus-companies";
import type {
  LabeledCode,
  ScanResultDto,
  StationRef,
  TransitCard,
  TransitGateRecord,
  TransitHistoryEntry,
} from "./types";

function labeled(code: number, label: string): LabeledCode {
  return { code, label };
}

function enrichStation(ref?: StationRef): StationRef | undefined {
  if (!ref) {
    return undefined;
  }
  return stationRefFrom(ref.line, ref.station, ref.region);
}

function enrichHistory(entry: TransitHistoryEntry): TransitHistoryEntry {
  const processCode = entry.processType.code & 0x7f;
  return {
    ...entry,
    terminalType: labeled(entry.terminalType.code, formatTerminalType(entry.terminalType.code)),
    processType: labeled(processCode, formatProcessType(processCode)),
    paymentType: labeled(entry.paymentType.code, formatPaymentType(entry.paymentType.code)),
    gateInstructionType: labeled(
      entry.gateInstructionType.code,
      formatEntryExitType(entry.gateInstructionType.code),
    ),
    entry: enrichStation(entry.entry),
    exit: enrichStation(entry.exit),
    busCompanyName:
      entry.busCompanyCode === undefined
        ? entry.busCompanyName
        : (BUS_COMPANY_LABELS[entry.busCompanyCode] ?? entry.busCompanyName),
  };
}

function enrichGateRecord(record: TransitGateRecord): TransitGateRecord {
  return {
    ...record,
    entryExitType: labeled(record.entryExitType.code, formatEntryExitType(record.entryExitType.code)),
    intermediateInstructionType: labeled(
      record.intermediateInstructionType.code,
      formatEntryExitType(record.intermediateInstructionType.code),
    ),
    station: stationRefFrom(record.station.line, record.station.station, record.station.region),
    commuterStation: stationRefFrom(
      record.commuterStation.line,
      record.commuterStation.station,
      record.commuterStation.region,
    ),
  };
}

function enrichTransit(card: TransitCard): TransitCard {
  return {
    ...card,
    gate: {
      ...card.gate,
      entry: enrichStation(card.gate.entry),
      intermediate: card.gate.intermediate
        ? {
            ...card.gate.intermediate,
            entry: enrichStation(card.gate.intermediate.entry),
            exit: enrichStation(card.gate.intermediate.exit),
          }
        : undefined,
    },
    histories: card.histories.map(enrichHistory),
    gateRecords: card.gateRecords?.map(enrichGateRecord),
    paidTickets: card.paidTickets?.map((ticket) => ({
      ...ticket,
      origin: stationRefFrom(ticket.origin.line, ticket.origin.station, ticket.origin.region),
      destination: stationRefFrom(
        ticket.destination.line,
        ticket.destination.station,
        ticket.destination.region,
      ),
      gateStation: stationRefFrom(
        ticket.gateStation.line,
        ticket.gateStation.station,
        ticket.gateStation.region,
      ),
    })),
  };
}

export function enrichScanResult(dto: ScanResultDto): ScanResultDto {
  return {
    ...dto,
    transit: dto.transit ? enrichTransit(dto.transit) : undefined,
  };
}
