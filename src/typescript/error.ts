import type { FelicaType } from "./types";

export type FelicaErrorCode =
  | "DEVICE_DISCONNECTED"
  | "DEVICE_NOT_FOUND"
  | "DEVICE_BUSY"
  | "DEVICE_ACCESS_DENIED"
  | "UNSUPPORTED_DEVICE"
  | "SCAN_TIMEOUT"
  | "SCAN_CANCELLED"
  | "COMMUNICATION_ERROR"
  | "PROTOCOL_ERROR"
  | "UNSUPPORTED_CARD"
  | "CARD_TYPE_MISMATCH"
  | "SESSION_CLOSED"
  | "INTERNAL_ERROR";

const KNOWN_CODES = new Set<FelicaErrorCode>([
  "DEVICE_DISCONNECTED",
  "DEVICE_NOT_FOUND",
  "DEVICE_BUSY",
  "DEVICE_ACCESS_DENIED",
  "UNSUPPORTED_DEVICE",
  "SCAN_TIMEOUT",
  "SCAN_CANCELLED",
  "COMMUNICATION_ERROR",
  "PROTOCOL_ERROR",
  "UNSUPPORTED_CARD",
  "CARD_TYPE_MISMATCH",
  "SESSION_CLOSED",
  "INTERNAL_ERROR",
]);

export class FelicaError extends Error {
  readonly code: FelicaErrorCode;
  readonly systemCodes?: number[];
  readonly detected?: FelicaType[];

  constructor(
    code: FelicaErrorCode,
    message: string,
    extras?: { systemCodes?: number[]; detected?: FelicaType[] },
  ) {
    super(message);
    this.name = "FelicaError";
    this.code = code;
    this.systemCodes = extras?.systemCodes;
    this.detected = extras?.detected;
    Object.setPrototypeOf(this, FelicaError.prototype);
  }
}

function asCode(value: string): FelicaErrorCode {
  return KNOWN_CODES.has(value as FelicaErrorCode) ? (value as FelicaErrorCode) : "INTERNAL_ERROR";
}

function extrasFrom(candidate: Record<string, unknown>): { systemCodes?: number[]; detected?: FelicaType[] } {
  const systemCodes = Array.isArray(candidate.systemCodes)
    ? candidate.systemCodes.filter((item): item is number => typeof item === "number")
    : undefined;
  const detected = Array.isArray(candidate.detected)
    ? candidate.detected.filter((item): item is FelicaType => typeof item === "string")
    : undefined;
  return { systemCodes, detected };
}

export function wrapError(err: unknown): FelicaError {
  if (err instanceof FelicaError) {
    return err;
  }
  if (typeof err === "object" && err !== null) {
    const candidate = err as Record<string, unknown>;
    if (typeof candidate.code === "string" && typeof candidate.message === "string") {
      return new FelicaError(asCode(candidate.code), candidate.message, extrasFrom(candidate));
    }
  }
  if (typeof err === "string") {
    try {
      const parsed = JSON.parse(err) as Record<string, unknown>;
      if (typeof parsed.code === "string" && typeof parsed.message === "string") {
        return new FelicaError(asCode(parsed.code), parsed.message, extrasFrom(parsed));
      }
    } catch {
      // Not JSON.
    }
    return new FelicaError("COMMUNICATION_ERROR", err);
  }
  if (err instanceof Error) {
    return new FelicaError("COMMUNICATION_ERROR", err.message);
  }
  return new FelicaError("INTERNAL_ERROR", String(err));
}
