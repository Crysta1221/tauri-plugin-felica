"""Generate Cybernetics operator/station TypeScript maps from ekicode.csv."""

from __future__ import annotations

import collections
import csv
import json
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
CSV_PATH = SCRIPT_DIR / "data" / "ekicode.csv"
OUT_DIR = SCRIPT_DIR.parent / "src" / "features" / "reader-scan" / "maps"

HEADER = """\
/**
 * Generated from publicly collected Cybernetics station codes
 * (furugori/ekicode, originally based on SFCardFan-style surveys).
 * Official JREA tables are unpublished; names may lag mergers or new stations.
 */
"""


def js(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def main() -> None:
    rows: list[tuple[int, int, int, str, str, str]] = []
    with CSV_PATH.open(encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            rows.append(
                (
                    int(row["地域コード"]),
                    int(row["路線コード"], 16),
                    int(row["駅順コード"], 16),
                    row["会社名"].strip(),
                    row["線区名"].strip(),
                    row["駅名"].strip(),
                )
            )
    rows.sort()

    by_rl: dict[tuple[int, int], list[tuple[int, str, str, str]]] = collections.defaultdict(list)
    for region, line, station, company, line_name, station_name in rows:
        by_rl[(region, line)].append((station, company, line_name, station_name))

    operator_ranges: list[tuple[int, int, int, int, str]] = []
    station_entries: list[tuple[int, int, int, str, str]] = []
    line_operators: list[tuple[int, int, str]] = []

    for (region, line), items in sorted(by_rl.items()):
        items.sort()
        counts = collections.Counter(company for _, company, _, _ in items)
        line_operators.append((region, line, counts.most_common(1)[0][0]))
        start_station, start_company = items[0][0], items[0][1]
        prev_station = start_station
        for station, company, line_name, station_name in items:
            station_entries.append((region, line, station, line_name, station_name))
            if company != start_company:
                operator_ranges.append((region, line, start_station, prev_station, start_company))
                start_station, start_company = station, company
            prev_station = station
        operator_ranges.append((region, line, start_station, prev_station, start_company))

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    op_lines = [
        HEADER,
        "export interface CyberneticsOperatorRange {",
        "  region: number;",
        "  line: number;",
        "  stationFrom: number;",
        "  stationTo: number;",
        "  companyName: string;",
        "}",
        "",
        "/** Line-level operator (most common company on that line code). */",
        "export const CYBERNETICS_LINE_OPERATORS: Readonly<Record<string, string>> = {",
    ]
    for region, line, company in line_operators:
        op_lines.append(f"  \"{region}-{line}\": {js(company)},")
    op_lines.extend(
        [
            "};",
            "",
            "/** Station-level operator ranges. Prefer this when a station code is present. */",
            "export const CYBERNETICS_OPERATOR_RANGES: readonly CyberneticsOperatorRange[] = [",
        ]
    )
    for region, line, start, end, company in operator_ranges:
        op_lines.append(
            "  { "
            f"region: {region}, line: {line}, stationFrom: {start}, stationTo: {end}, "
            f"companyName: {js(company)} "
            "},"
        )
    op_lines.append("];")
    op_lines.append("")
    (OUT_DIR / "operators.ts").write_text("\n".join(op_lines), encoding="utf-8")

    st_lines = [
        HEADER,
        "export interface CyberneticsStationRecord {",
        "  region: number;",
        "  line: number;",
        "  station: number;",
        "  lineName: string;",
        "  stationName: string;",
        "}",
        "",
        "export const CYBERNETICS_STATIONS: readonly CyberneticsStationRecord[] = [",
    ]
    for region, line, station, line_name, station_name in station_entries:
        st_lines.append(
            "  { "
            f"region: {region}, line: {line}, station: {station}, "
            f"lineName: {js(line_name)}, stationName: {js(station_name)} "
            "},"
        )
    st_lines.append("];")
    st_lines.append("")
    (OUT_DIR / "stations.ts").write_text("\n".join(st_lines), encoding="utf-8")

    print(f"ranges={len(operator_ranges)} stations={len(station_entries)} lines={len(line_operators)}")
    print(f"operators.ts={(OUT_DIR / 'operators.ts').stat().st_size}")
    print(f"stations.ts={(OUT_DIR / 'stations.ts').stat().st_size}")


if __name__ == "__main__":
    main()
