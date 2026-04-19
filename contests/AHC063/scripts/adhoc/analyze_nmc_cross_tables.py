#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ANALYSIS_DIR = ROOT / "results" / "analysis"
BINS = [
    "v012_simple_beam",
    "v014_lane_split_beam",
    "v139_refactor_v137",
    "v149_no_logs",
]
BIN_ORDER = {bin_name: idx for idx, bin_name in enumerate(BINS)}
DENSITY_LOW_MAX = 0.40
DENSITY_MID_MAX = 0.55
DENSITY3_ORDER = {"L": 0, "M": 1, "H": 2}


@dataclass(frozen=True)
class CellWinner:
    case_count: int
    winner_bin: str
    winner_count: int
    runner_up_bin: str
    runner_up_count: int


@dataclass(frozen=True)
class CaseData:
    case_id: str
    N: int
    M: int
    C: int
    density3: str
    best_bin: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create cross winner tables for N/C/M/density3 from serial_eval JSONL."
    )
    parser.add_argument(
        "--jsonl",
        help="input JSONL path. Defaults to the latest serial_eval_*.jsonl under results/analysis",
    )
    parser.add_argument(
        "--out-dir",
        help="output directory. Defaults to results/analysis/cross_winner_<timestamp>",
    )
    return parser.parse_args()


def latest_jsonl_path() -> Path:
    candidates = list(ANALYSIS_DIR.glob("serial_eval_*.jsonl"))
    if not candidates:
        raise SystemExit("error: serial_eval_*.jsonl not found under results/analysis")
    return max(candidates, key=lambda path: path.stat().st_mtime)


def default_out_dir() -> Path:
    timestamp = datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
    return ANALYSIS_DIR / f"cross_winner_{timestamp}"


def density3_bucket(N: int, M: int) -> str:
    density = M / (N * N)
    if density < DENSITY_LOW_MAX:
        return "L"
    if density < DENSITY_MID_MAX:
        return "M"
    return "H"


def load_cases(jsonl_path: Path) -> list[CaseData]:
    grouped: dict[str, list[dict]] = defaultdict(list)
    with jsonl_path.open(encoding="utf-8") as fh:
        for line_no, line in enumerate(fh, start=1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as exc:
                raise SystemExit(
                    f"error: failed to parse JSON at {jsonl_path}:{line_no}: {exc}"
                ) from exc
            grouped[record["case_id"]].append(record)

    cases = []
    for case_id, records in sorted(grouped.items()):
        if len(records) != len(BINS):
            raise SystemExit(
                f"error: case {case_id} has {len(records)} records, expected {len(BINS)}"
            )
        if any(record["status"] != "ok" for record in records):
            raise SystemExit(f"error: case {case_id} contains non-ok rows")

        best = min(
            records,
            key=lambda record: (
                int(record["score"]),
                int(record["elapsed_ms"]),
                BIN_ORDER[record["bin"]],
            ),
        )
        meta = records[0]
        cases.append(
            CaseData(
                case_id=case_id,
                N=int(meta["N"]),
                M=int(meta["M"]),
                C=int(meta["C"]),
                density3=density3_bucket(int(meta["N"]), int(meta["M"])),
                best_bin=best["bin"],
            )
        )
    return cases


def make_cell_winner(cases: list[CaseData], row_key: str, col_key: str) -> dict[tuple[object, object], CellWinner]:
    counts: dict[tuple[object, object], Counter[str]] = defaultdict(Counter)
    totals: Counter[tuple[object, object]] = Counter()
    for case in cases:
        pair = (getattr(case, row_key), getattr(case, col_key))
        counts[pair][case.best_bin] += 1
        totals[pair] += 1

    result = {}
    for pair, counter in counts.items():
        ranking = sorted(
            BINS,
            key=lambda bin_name: (-counter[bin_name], BIN_ORDER[bin_name]),
        )
        winner_bin = ranking[0]
        runner_up_bin = ranking[1]
        result[pair] = CellWinner(
            case_count=totals[pair],
            winner_bin=winner_bin,
            winner_count=counter[winner_bin],
            runner_up_bin=runner_up_bin,
            runner_up_count=counter[runner_up_bin],
        )
    return result


def write_matrix_csv(
    path: Path,
    row_values: list[object],
    col_values: list[object],
    cell_map: dict[tuple[object, object], CellWinner],
    row_label: str,
) -> None:
    with path.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.writer(fh)
        writer.writerow([row_label, *col_values])
        for row_value in row_values:
            row = [row_value]
            for col_value in col_values:
                cell = cell_map.get((row_value, col_value))
                row.append(cell.winner_bin if cell else "")
            writer.writerow(row)


def write_detail_csv(path: Path, dimensions: list[tuple[str, str, str]], cases: list[CaseData]) -> None:
    rows = []
    for dimension_name, row_key, col_key in dimensions:
        cell_map = make_cell_winner(cases, row_key=row_key, col_key=col_key)
        for (row_value, col_value), cell in sorted(cell_map.items(), key=lambda item: (str(item[0][0]), str(item[0][1]))):
            rows.append(
                {
                    "dimension": dimension_name,
                    "row_value": str(row_value),
                    "col_value": str(col_value),
                    "case_count": str(cell.case_count),
                    "winner_bin": cell.winner_bin,
                    "winner_count": str(cell.winner_count),
                    "runner_up_bin": cell.runner_up_bin,
                    "runner_up_count": str(cell.runner_up_count),
                    "win_gap_count": str(cell.winner_count - cell.runner_up_count),
                }
            )

    fieldnames = [
        "dimension",
        "row_value",
        "col_value",
        "case_count",
        "winner_bin",
        "winner_count",
        "runner_up_bin",
        "runner_up_count",
        "win_gap_count",
    ]
    with path.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def compress_ranges(values: list[int]) -> str:
    if not values:
        return "-"
    values = sorted(values)
    ranges = []
    start = values[0]
    end = values[0]
    for value in values[1:]:
        if value == end + 1:
            end = value
            continue
        ranges.append(f"{start}" if start == end else f"{start}..{end}")
        start = value
        end = value
    ranges.append(f"{start}" if start == end else f"{start}..{end}")
    return ", ".join(ranges)


def markdown_table(headers: list[str], rows: list[list[str]]) -> list[str]:
    lines = []
    lines.append("| " + " | ".join(headers) + " |")
    lines.append("| " + " | ".join(["---"] * len(headers)) + " |")
    for row in rows:
        lines.append("| " + " | ".join(row) + " |")
    return lines


def cell_markdown_matrix(
    row_values: list[object],
    col_values: list[object],
    cell_map: dict[tuple[object, object], CellWinner],
    row_label: str,
) -> list[str]:
    headers = [row_label, *[str(col) for col in col_values]]
    rows = []
    for row_value in row_values:
        row = [str(row_value)]
        for col_value in col_values:
            cell = cell_map.get((row_value, col_value))
            row.append(cell.winner_bin if cell else "")
        rows.append(row)
    return markdown_table(headers, rows)


def nm_range_summary(
    row_values: list[int],
    col_values: list[int],
    cell_map: dict[tuple[object, object], CellWinner],
) -> list[str]:
    lines = []
    for n in row_values:
        winners = []
        current_bin = None
        current_start = None
        current_end = None
        for m in col_values:
            cell = cell_map.get((n, m))
            if cell is None:
                if current_bin is not None:
                    winners.append((current_start, current_end, current_bin))
                    current_bin = None
                continue
            winner = cell.winner_bin
            if current_bin == winner and current_end is not None and m == current_end + 1:
                current_end = m
                continue
            if current_bin is not None:
                winners.append((current_start, current_end, current_bin))
            current_bin = winner
            current_start = m
            current_end = m
        if current_bin is not None:
            winners.append((current_start, current_end, current_bin))
        formatted = ", ".join(
            f"{start}" if start == end else f"{start}..{end}"
            for start, end, _ in winners
        )
        labeled = ", ".join(
            f"`{bin_name}`: {', '.join(f'{start}' if start == end else f'{start}..{end}' for start, end, winner in winners if winner == bin_name)}"
            for bin_name in BINS
            if any(winner == bin_name for _, _, winner in winners)
        )
        lines.append(f"- `N={n}` の `M` winner range は {labeled} である。")
    return lines


def write_summary(
    path: Path,
    jsonl_path: Path,
    nc_row_values: list[int],
    nc_col_values: list[int],
    nc_cells: dict[tuple[object, object], CellWinner],
    nm_row_values: list[int],
    nm_col_values: list[int],
    nm_cells: dict[tuple[object, object], CellWinner],
    cd_row_values: list[int],
    cd_col_values: list[str],
    cd_cells: dict[tuple[object, object], CellWinner],
) -> None:
    lines = []
    lines.append("# クロス winner 表")
    lines.append("")
    lines.append("## 前提")
    lines.append(f"- 入力 JSONL: `{jsonl_path}`")
    lines.append("- 各セルには、その bucket 内で最も勝数の多かった bin 名を入れている。")
    lines.append("- tie は bin の固定順 `v012 -> v014 -> v139 -> v149` で破っている。")
    lines.append("")
    lines.append("## N × C")
    lines.extend(
        cell_markdown_matrix(
            row_values=nc_row_values,
            col_values=nc_col_values,
            cell_map=nc_cells,
            row_label="N\\C",
        )
    )
    lines.append("")
    lines.append("## C × density3")
    lines.extend(
        cell_markdown_matrix(
            row_values=cd_row_values,
            col_values=cd_col_values,
            cell_map=cd_cells,
            row_label="C\\density3",
        )
    )
    lines.append("")
    lines.append("## N × M の読み方")
    lines.append("- `N × M` は列数が多いため、行列本体は CSV に保存している。ここでは行ごとの大まかな range だけ抜き出す。")
    lines.extend(nm_range_summary(nm_row_values, nm_col_values, nm_cells))
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    jsonl_path = Path(args.jsonl).expanduser().resolve() if args.jsonl else latest_jsonl_path()
    out_dir = Path(args.out_dir).expanduser().resolve() if args.out_dir else default_out_dir()
    out_dir.mkdir(parents=True, exist_ok=True)

    cases = load_cases(jsonl_path)

    nc_row_values = sorted({case.N for case in cases})
    nc_col_values = sorted({case.C for case in cases})
    nm_row_values = sorted({case.N for case in cases})
    nm_col_values = sorted({case.M for case in cases})
    cd_row_values = sorted({case.C for case in cases})
    cd_col_values = sorted({case.density3 for case in cases}, key=lambda value: DENSITY3_ORDER[value])

    nc_cells = make_cell_winner(cases, row_key="N", col_key="C")
    nm_cells = make_cell_winner(cases, row_key="N", col_key="M")
    cd_cells = make_cell_winner(cases, row_key="C", col_key="density3")

    write_matrix_csv(
        out_dir / "N_C_winner_matrix.csv",
        row_values=nc_row_values,
        col_values=nc_col_values,
        cell_map=nc_cells,
        row_label="N\\C",
    )
    write_matrix_csv(
        out_dir / "N_M_winner_matrix.csv",
        row_values=nm_row_values,
        col_values=nm_col_values,
        cell_map=nm_cells,
        row_label="N\\M",
    )
    write_matrix_csv(
        out_dir / "C_density3_winner_matrix.csv",
        row_values=cd_row_values,
        col_values=cd_col_values,
        cell_map=cd_cells,
        row_label="C\\density3",
    )
    write_detail_csv(
        out_dir / "winner_cell_details.csv",
        dimensions=[
            ("N_C", "N", "C"),
            ("N_M", "N", "M"),
            ("C_density3", "C", "density3"),
        ],
        cases=cases,
    )
    write_summary(
        out_dir / "summary.md",
        jsonl_path=jsonl_path,
        nc_row_values=nc_row_values,
        nc_col_values=nc_col_values,
        nc_cells=nc_cells,
        nm_row_values=nm_row_values,
        nm_col_values=nm_col_values,
        nm_cells=nm_cells,
        cd_row_values=cd_row_values,
        cd_col_values=cd_col_values,
        cd_cells=cd_cells,
    )

    print(f"wrote {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
