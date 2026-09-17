export function formatYen(value: number): string {
  return new Intl.NumberFormat("ja-JP", {
    style: "currency",
    currency: "JPY",
    maximumFractionDigits: 0,
  }).format(value);
}

export function formatAmount(amount?: number | null): string {
  if (amount === undefined || amount === null) {
    return "ー";
  }
  if (amount > 0) {
    return `+${amount.toLocaleString()} 円`;
  }
  return `${amount.toLocaleString()} 円`;
}

export function formatIdm(idm: string): string {
  return idm.replaceAll(/(.{4})(?=.)/gu, "$1 ").trim();
}

export function formatSystemCode(value: number): string {
  return `0x${value.toString(16).toUpperCase().padStart(4, "0")}`;
}

export function formatTransitRoute(entry: {
  entry?: { label: string };
  exit?: { label: string };
  busCompanyCode?: number;
  busCompanyName?: string;
}): string {
  if (entry.busCompanyName) {
    return entry.busCompanyName;
  }
  if (entry.busCompanyCode !== undefined) {
    return `事業者 ${formatSystemCode(entry.busCompanyCode)}`;
  }
  if (entry.entry || entry.exit) {
    return `${entry.entry?.label ?? "ー"} → ${entry.exit?.label ?? "ー"}`;
  }
  return "ー";
}
