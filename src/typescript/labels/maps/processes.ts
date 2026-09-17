/**
 * History byte 1: processing / usage type (処理種別 / 利用種別).
 * Bit 7 set means SF was combined with cash or another card.
 * Labels follow felicalib / nfc-felica / Metrodroid.
 */
export const PROCESS_TYPE_LABELS: Readonly<Record<number, string>> = {
  0x01: "運賃支払",
  0x02: "チャージ",
  0x03: "券購入",
  0x04: "精算",
  0x05: "入場精算",
  0x06: "窓口出場",
  0x07: "新規発行",
  0x08: "窓口控除",
  0x0d: "バス等",
  0x0f: "バス等",
  0x11: "再発行",
  0x13: "新幹線利用",
  0x14: "入場時オートチャージ",
  0x15: "出場時オートチャージ",
  0x1f: "バスチャージ",
  0x23: "バス企画券購入",
  0x46: "物販",
  0x48: "特典チャージ",
  0x49: "レジ入金",
  0x4a: "物販取消",
  0x4b: "入場物販",
  0x84: "他社精算",
  0x85: "他社入場精算",
  0xc6: "現金併用物販",
  0xcb: "入場現金併用物販",
};

export const BUS_PROCESS_TYPES: ReadonlySet<number> = new Set([0x0d, 0x0f, 0x1f, 0x23]);

export const RETAIL_PROCESS_TYPES: ReadonlySet<number> = new Set([
  0x46, 0x49, 0x4a, 0x4b, 0xc6, 0xcb,
]);

export const CHARGE_PROCESS_TYPES: ReadonlySet<number> = new Set([0x02, 0x14, 0x15, 0x48, 0x49]);

/** Mask used to ignore the combined-payment flag when looking up labels. */
export const PROCESS_TYPE_CODE_MASK = 0x7f;
