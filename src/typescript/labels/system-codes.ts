export const SYSTEM_CODE_LABELS: Readonly<Record<number, string>> = {
  0x0003: "交通系IC",
  0xfe00: "共通領域",
  0x811d: "楽天Edy",
  0x04c7: "nanaco",
  0x8b61: "WAON",
  0x852b: "WAON",
  0x04c1: "QUICPay",
  0x88b4: "FeliCa Lite/Lite-S",
  0x802b: "せたまる",
  0x80de: "IruCa",
  0x8592: "PASPY",
  0x865e: "SAPICA",
};

export function labelSystemCode(code: number): string {
  return SYSTEM_CODE_LABELS[code] ?? "";
}
