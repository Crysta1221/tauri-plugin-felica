import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const root = path.resolve(import.meta.dir, "..");
const assets = path.join(root, "assets");
const mapsDir = path.join(root, "src", "typescript", "labels", "maps");

function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  for (const line of text.split(/\r?\n/u)) {
    if (!line.trim()) {
      continue;
    }
    rows.push(splitCsvLine(line));
  }
  return rows;
}

function splitCsvLine(line: string): string[] {
  const cells: string[] = [];
  let current = "";
  let inQuotes = false;
  for (let i = 0; i < line.length; i += 1) {
    const ch = line[i];
    if (ch === '"') {
      inQuotes = !inQuotes;
      continue;
    }
    if (ch === "," && !inQuotes) {
      cells.push(current);
      current = "";
      continue;
    }
    current += ch;
  }
  cells.push(current);
  return cells;
}

function parseHex(value: string): number {
  return Number.parseInt(value.trim(), 16);
}

function escape(value: string): string {
  return value.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}

async function generateStations(): Promise<number> {
  const csv = await readFile(path.join(assets, "station_codes.csv"), "utf8");
  const rows = parseCsv(csv).slice(1);
  const records: string[] = [];
  for (const row of rows) {
    if (row.length < 6) {
      continue;
    }
    const region = parseHex(row[0]);
    const line = parseHex(row[1]);
    const station = parseHex(row[2]);
    if (Number.isNaN(region) || Number.isNaN(line) || Number.isNaN(station)) {
      continue;
    }
    const lineName = escape(row[4] ?? "");
    const stationName = escape(row[5] ?? "");
    records.push(
      `  { region: ${region}, line: ${line}, station: ${station}, lineName: "${lineName}", stationName: "${stationName}" },`,
    );
  }
  if (records.length === 0) {
    throw new Error("station_codes.csv produced an empty dictionary; refusing to overwrite");
  }
  const body = `/**
 * Generated from assets/station_codes.csv by scripts/generate-labels.ts.
 * Official JREA tables are unpublished; names may lag mergers or new stations.
 */

export interface CyberneticsStationRecord {
  region: number;
  line: number;
  station: number;
  lineName: string;
  stationName: string;
}

export const CYBERNETICS_STATIONS: readonly CyberneticsStationRecord[] = [
${records.join("\n")}
];
`;
  await writeFile(path.join(mapsDir, "stations.ts"), body, "utf8");
  return records.length;
}

async function generateBusCompanies(): Promise<number> {
  const csv = await readFile(path.join(assets, "bus_company_codes.csv"), "utf8");
  const rows = parseCsv(csv).slice(1);
  const entries: string[] = [];
  for (const row of rows) {
    if (row.length < 2) {
      continue;
    }
    const code = parseHex(row[0]);
    const name = escape(row[1] ?? "");
    if (Number.isNaN(code) || !name) {
      continue;
    }
    entries.push(`  ${code}: "${name}",`);
  }
  if (entries.length === 0) {
    throw new Error("bus_company_codes.csv produced an empty dictionary; refusing to overwrite");
  }
  const body = `/**
 * Generated from assets/bus_company_codes.csv by scripts/generate-labels.ts.
 */
export const BUS_COMPANY_LABELS: Readonly<Record<number, string>> = {
${entries.join("\n")}
};
`;
  await writeFile(path.join(mapsDir, "bus-companies.ts"), body, "utf8");
  return entries.length;
}

await mkdir(mapsDir, { recursive: true });
const stations = await generateStations();
const buses = await generateBusCompanies();
console.log(`generated ${stations} stations, ${buses} bus companies`);
