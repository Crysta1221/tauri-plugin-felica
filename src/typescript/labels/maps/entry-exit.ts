/**
 * History byte 3: entry/exit classification (入出場種別).
 * Public table from jennychan's Suica history notes.
 */
export const ENTRY_EXIT_TYPE_LABELS: Readonly<Record<number, string>> = {
  0x00: "なし",
  0x01: "入場",
  0x02: "入場出場",
  0x03: "定期入場出場",
  0x04: "入場定期出場",
  0x0e: "窓口出場",
  0x0f: "入場出場（バス等）",
  0x12: "料金定期入場料金出場",
  0x17: "入場出場（乗継割引）",
  0x21: "入場出場（バス等乗継割引）",
};
