/**
 * History byte 0: equipment / terminal type (機器種別).
 * Compiled from felicalib / nfc-felica / Metrodroid reverse-engineering of
 * Cybernetics IC history. Unlisted codes stay unknown.
 */
export const TERMINAL_TYPE_LABELS: Readonly<Record<number, string>> = {
  0x03: "精算機",
  0x04: "携帯型端末",
  0x05: "車載端末",
  0x07: "券売機",
  0x08: "券売機",
  0x09: "入金機",
  0x12: "券売機",
  0x13: "券売機",
  0x14: "券売機",
  0x15: "券売機",
  0x16: "改札機",
  0x17: "簡易改札機",
  0x18: "窓口端末",
  0x19: "窓口端末",
  0x1a: "改札端末",
  0x1b: "携帯電話",
  0x1c: "乗継精算機",
  0x1d: "連絡改札機",
  0x1f: "簡易入金機",
  0x20: "窓口端末",
  0x21: "精算機",
  0x22: "窓口端末",
  0x23: "改札機",
  0x24: "車内補充券発行機",
  0x46: "VIEW ALTTE",
  0x48: "VIEW ALTTE",
  0xc7: "物販端末",
  0xc8: "自販機",
  0xc9: "物販端末",
  0xca: "物販端末",
};

/** Terminals that encode retail time + store id instead of rail stations. */
export const RETAIL_TERMINAL_TYPES: ReadonlySet<number> = new Set([0xc7, 0xc8, 0xc9, 0xca]);

/** Bus / onboard terminals that encode 16-bit company and stop codes. */
export const BUS_TERMINAL_TYPES: ReadonlySet<number> = new Set([0x05]);
