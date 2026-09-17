/**
 * Packed region byte (history byte 15):
 * bits 7-6 = entry region, bits 5-4 = exit region, bits 3-0 unused.
 *
 * Line codes 0x00-0x7F are JR (nationwide unique). Line codes 0x80-0xFF
 * are private/public railways and repeat per region.
 */
export const CYBERNETICS_REGION_LABELS: Readonly<Record<number, string>> = {
  0: "JR・関東私鉄",
  1: "中部私鉄",
  2: "関西私鉄",
  3: "その他私鉄",
};

export const JR_LINE_CODE_MAX = 0x7f;

export function unpackRegionByte(regionByte: number): { entryRegion: number; exitRegion: number } {
  return {
    entryRegion: (regionByte >> 6) & 0x03,
    exitRegion: (regionByte >> 4) & 0x03,
  };
}

export function regionForLine(lineCode: number, packedRegion: number, role: "entry" | "exit"): number {
  if (lineCode <= JR_LINE_CODE_MAX) {
    return 0;
  }
  const unpacked = unpackRegionByte(packedRegion);
  return role === "entry" ? unpacked.entryRegion : unpacked.exitRegion;
}
