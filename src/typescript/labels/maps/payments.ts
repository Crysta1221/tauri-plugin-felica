/**
 * History byte 2: settlement / payment type (支払種別).
 * Sparse public table from jennychan / WDIC-style dumps.
 */
export const PAYMENT_TYPE_LABELS: Readonly<Record<number, string>> = {
  0x00: "現金・なし",
  0x02: "VIEW",
  0x0b: "PiTaPa",
  0x0d: "オートチャージ対応PASMO",
  0x3f: "モバイルSuica",
};
