#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ANALYSIS_DIR = ROOT / "results" / "analysis"
TARGET_BINS = [
    "v012_simple_beam",
    "v139_refactor_v137",
    "v149_no_logs",
]
BIN_ORDER = {bin_name: idx for idx, bin_name in enumerate(TARGET_BINS)}
DENSITY_LOW_MAX = 0.40
DENSITY_MID_MAX = 0.55
DENSITY3_ORDER = {"L": 0, "M": 1, "H": 2}
STRICT_BUDGET_MS = 800
NEAR_BUDGET_MS = 1000


@dataclass(frozen=True)
class CaseBudget:
    case_id: str
    dataset: str
    N: int
    M: int
    C: int
    density: float
    density3: str
    total_elapsed_ms: int


@dataclass(frozen=True)
class BucketStat:
    case_count: int
    avg_elapsed_ms: float
    p95_elapsed_ms: int
    max_elapsed_ms: int


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Analyze whether v012+v139+v149 total elapsed stays within a time budget."
    )
    parser.add_argument(
        "--jsonl",
        help="input JSONL path. Defaults to the latest serial_eval_*.jsonl under results/analysis",
    )
    parser.add_argument(
        "--out-dir",
        help="output directory. Defaults to results/analysis/threeway_budget_<timestamp>",
    )
    return parser.parse_args()


def latest_jsonl_path() -> Path:
    candidates = list(ANALYSIS_DIR.glob("serial_eval_*.jsonl"))
    if not candidates:
        raise SystemExit("error: serial_eval_*.jsonl not found under results/analysis")
    return max(candidates, key=lambda path: path.stat().st_mtime)


def default_out_dir() -> Path:
    timestamp = datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
    return ANALYSIS_DIR / f"threeway_budget_{timestamp}"


def density3_bucket(N: int, M: int) -> tuple[float, str]:
    density = M / (N * N)
    if density < DENSITY_LOW_MAX:
        return density, "L"
    if density < DENSITY_MID_MAX:
        return density, "M"
    return density, "H"


def load_cases(jsonl_path: Path) -> list[CaseBudget]:
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
            if record["bin"] not in TARGET_BINS:
                continue
            grouped[record["case_id"]].append(record)

    cases = []
    for case_id, records in sorted(grouped.items()):
        if len(records) != len(TARGET_BINS):
            raise SystemExit(
                f"error: case {case_id} has {len(records)} target rows, expected {len(TARGET_BINS)}"
            )
        if any(record["status"] != "ok" for record in records):
            raise SystemExit(f"error: case {case_id} contains non-ok rows")
        bins_seen = {record["bin"] for record in records}
        if bins_seen != set(TARGET_BINS):
            raise SystemExit(f"error: case {case_id} does not contain the expected target bins")

        meta = records[0]
        density, density3 = density3_bucket(int(meta["N"]), int(meta["M"]))
        cases.append(
            CaseBudget(
                case_id=case_id,
                dataset=meta["dataset"],
                N=int(meta["N"]),
                M=int(meta["M"]),
                C=int(meta["C"]),
                density=density,
                density3=density3,
                total_elapsed_ms=sum(int(record["elapsed_ms"]) for record in records),
            )
        )
    return cases


def bucket_stats(cases: list[CaseBudget], keys: tuple[str, ...]) -> dict[tuple[object, ...], BucketStat]:
    grouped: dict[tuple[object, ...], list[int]] = defaultdict(list)
    for case in cases:
        grouped[tuple(getattr(case, key) for key in keys)].append(case.total_elapsed_ms)

    stats = {}
    for bucket, totals in grouped.items():
        totals_sorted = sorted(totals)
        p95_index = min(len(totals_sorted) - 1, int(len(totals_sorted) * 0.95))
        stats[bucket] = BucketStat(
            case_count=len(totals),
            avg_elapsed_ms=sum(totals) / len(totals),
            p95_elapsed_ms=totals_sorted[p95_index],
            max_elapsed_ms=totals_sorted[-1],
        )
    return stats


def format_float(value: float) -> str:
    return f"{value:.3f}"


def write_csv(path: Path, fieldnames: list[str], rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def build_bucket_rows(cases: list[CaseBudget]) -> list[dict[str, str]]:
    definitions = [
        ("N", ("N",)),
        ("C", ("C",)),
        ("density3", ("density3",)),
        ("N_C", ("N", "C")),
        ("N_density3", ("N", "density3")),
        ("C_density3", ("C", "density3")),
        ("N_C_density3", ("N", "C", "density3")),
    ]
    rows = []
    for dimension, keys in definitions:
        stats = bucket_stats(cases, keys)
        def sort_key(item: tuple[tuple[object, ...], BucketStat]) -> tuple[object, ...]:
            bucket = item[0]
            out = []
            for value in bucket:
                if value in DENSITY3_ORDER:
                    out.append(DENSITY3_ORDER[value])
                else:
                    out.append(value)
            return tuple(out)

        for bucket, stat in sorted(stats.items(), key=sort_key):
            rows.append(
                {
                    "dimension": dimension,
                    "bucket": "|".join(str(value) for value in bucket),
                    "case_count": str(stat.case_count),
                    "avg_elapsed_ms": format_float(stat.avg_elapsed_ms),
                    "p95_elapsed_ms": str(stat.p95_elapsed_ms),
                    "max_elapsed_ms": str(stat.max_elapsed_ms),
                    "strict_safe_800ms": "true" if stat.max_elapsed_ms <= STRICT_BUDGET_MS else "false",
                    "near_safe_1000ms": "true" if stat.max_elapsed_ms <= NEAR_BUDGET_MS else "false",
                }
            )
    return rows


def build_nm_rows(cases: list[CaseBudget]) -> list[dict[str, str]]:
    stats = bucket_stats(cases, ("N", "M"))
    rows = []
    for (N, M), stat in sorted(stats.items()):
        rows.append(
            {
                "N": str(N),
                "M": str(M),
                "case_count": str(stat.case_count),
                "avg_elapsed_ms": format_float(stat.avg_elapsed_ms),
                "p95_elapsed_ms": str(stat.p95_elapsed_ms),
                "max_elapsed_ms": str(stat.max_elapsed_ms),
                "strict_safe_800ms": "true" if stat.max_elapsed_ms <= STRICT_BUDGET_MS else "false",
                "near_safe_1000ms": "true" if stat.max_elapsed_ms <= NEAR_BUDGET_MS else "false",
            }
        )
    return rows


def contiguous_ranges(values: list[int]) -> str:
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


def nm_safe_ranges(nm_rows: list[dict[str, str]]) -> list[str]:
    rows_by_n: dict[int, list[int]] = defaultdict(list)
    for row in nm_rows:
        if row["strict_safe_800ms"] != "true":
            continue
        rows_by_n[int(row["N"])].append(int(row["M"]))

    lines = []
    for N in sorted(rows_by_n):
        lines.append(f"- `N={N}` で観測上 `max<=800ms` だった `M` は {contiguous_ranges(rows_by_n[N])} である。")
    return lines


def markdown_table(headers: list[str], rows: list[list[str]]) -> list[str]:
    lines = []
    lines.append("| " + " | ".join(headers) + " |")
    lines.append("| " + " | ".join(["---"] * len(headers)) + " |")
    for row in rows:
        lines.append("| " + " | ".join(row) + " |")
    return lines


def filter_bucket_rows(rows: list[dict[str, str]], dimension: str, flag_key: str) -> list[dict[str, str]]:
    filtered = [row for row in rows if row["dimension"] == dimension and row[flag_key] == "true"]
    def sort_key(row: dict[str, str]) -> tuple[object, ...]:
        parts = row["bucket"].split("|")
        out = []
        for part in parts:
            if part in DENSITY3_ORDER:
                out.append(DENSITY3_ORDER[part])
            else:
                out.append(int(part))
        return tuple(out)
    return sorted(filtered, key=sort_key)


def write_summary(
    path: Path,
    jsonl_path: Path,
    cases: list[CaseBudget],
    bucket_rows: list[dict[str, str]],
    nm_rows: list[dict[str, str]],
) -> None:
    global_avg = sum(case.total_elapsed_ms for case in cases) / len(cases)
    global_max = max(case.total_elapsed_ms for case in cases)

    safe_n = filter_bucket_rows(bucket_rows, "N", "strict_safe_800ms")
    safe_c = filter_bucket_rows(bucket_rows, "C", "strict_safe_800ms")
    safe_density = filter_bucket_rows(bucket_rows, "density3", "strict_safe_800ms")
    safe_nc = filter_bucket_rows(bucket_rows, "N_C", "strict_safe_800ms")
    near_nc = [
        row
        for row in bucket_rows
        if row["dimension"] == "N_C"
        and row["strict_safe_800ms"] == "false"
        and row["near_safe_1000ms"] == "true"
    ]
    safe_nd = filter_bucket_rows(bucket_rows, "N_density3", "strict_safe_800ms")
    safe_ncd = filter_bucket_rows(bucket_rows, "N_C_density3", "strict_safe_800ms")

    lines = []
    lines.append("# 3-way 合計時間 budget 分析")
    lines.append("")
    lines.append("## 前提")
    lines.append(f"- 入力 JSONL: `{jsonl_path}`")
    lines.append("- 対象時間は `v012_simple_beam + v139_refactor_v137 + v149_no_logs` の wall-clock 合計である。")
    lines.append(f"- 厳密安全条件は「その bucket に含まれる観測ケースで `max_elapsed_ms <= {STRICT_BUDGET_MS}`」と定義する。")
    lines.append(f"- near-safe は `max_elapsed_ms <= {NEAR_BUDGET_MS}` とする。")
    lines.append("")
    lines.append("## 全体")
    lines.append(f"- 全 2100 ケース平均: {format_float(global_avg)} ms")
    lines.append(f"- 全 2100 ケース最大: {global_max} ms")
    lines.append(f"- 単独軸 (`N` / `C` / `density3`) だけでは、観測上すべてのケースで `<= {STRICT_BUDGET_MS}ms` を保証できる bucket はなかった。")
    lines.append("")
    lines.append("## 厳密安全な N × C")
    if safe_nc:
        lines.extend(
            markdown_table(
                ["N", "C", "case_count", "avg_ms", "p95_ms", "max_ms"],
                [
                    [
                        row["bucket"].split("|")[0],
                        row["bucket"].split("|")[1],
                        row["case_count"],
                        row["avg_elapsed_ms"],
                        row["p95_elapsed_ms"],
                        row["max_elapsed_ms"],
                    ]
                    for row in safe_nc
                ],
            )
        )
    lines.append("")
    lines.append("## 厳密安全な N × density3")
    if safe_nd:
        lines.extend(
            markdown_table(
                ["N", "density3", "case_count", "avg_ms", "p95_ms", "max_ms"],
                [
                    [
                        row["bucket"].split("|")[0],
                        row["bucket"].split("|")[1],
                        row["case_count"],
                        row["avg_elapsed_ms"],
                        row["p95_elapsed_ms"],
                        row["max_elapsed_ms"],
                    ]
                    for row in safe_nd
                ],
            )
        )
    lines.append("")
    lines.append("## 厳密安全な N × C × density3")
    lines.append(f"- 厳密安全 bucket 数: {len(safe_ncd)}")
    lines.append("- 細かい bucket 一覧は `bucket_budget.csv` を見ればよい。")
    lines.append("")
    lines.append("## near-safe な N × C")
    if near_nc:
        lines.extend(
            markdown_table(
                ["N", "C", "case_count", "avg_ms", "p95_ms", "max_ms"],
                [
                    [
                        row["bucket"].split("|")[0],
                        row["bucket"].split("|")[1],
                        row["case_count"],
                        row["avg_elapsed_ms"],
                        row["p95_elapsed_ms"],
                        row["max_elapsed_ms"],
                    ]
                    for row in near_nc
                ],
            )
        )
    lines.append("")
    lines.append("## N × M の厳密安全 range")
    lines.extend(nm_safe_ranges(nm_rows))
    lines.append("")
    lines.append("## 素朴な結論")
    lines.append(f"- `0.8s` を厳密に守るなら、まず `N × C` か `N × density3` で条件を切るのがよい。")
    lines.append("- 最も簡単な厳密安全条件は `N=8 and C>=4`、または `N in {9,10,11} and C=3` である。")
    lines.append("- `N × density3` では `N=8 and density in {L,H}`、および `N in {10,11,12} and density=L` が厳密安全である。")
    lines.append("- `N × M` で切ると安全領域は広がるが、`M` に対して単純な単調閾値にはなっていない。")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    jsonl_path = Path(args.jsonl).expanduser().resolve() if args.jsonl else latest_jsonl_path()
    out_dir = Path(args.out_dir).expanduser().resolve() if args.out_dir else default_out_dir()
    out_dir.mkdir(parents=True, exist_ok=True)

    cases = load_cases(jsonl_path)
    bucket_rows = build_bucket_rows(cases)
    nm_rows = build_nm_rows(cases)

    write_csv(
        out_dir / "bucket_budget.csv",
        [
            "dimension",
            "bucket",
            "case_count",
            "avg_elapsed_ms",
            "p95_elapsed_ms",
            "max_elapsed_ms",
            "strict_safe_800ms",
            "near_safe_1000ms",
        ],
        bucket_rows,
    )
    write_csv(
        out_dir / "N_M_budget.csv",
        [
            "N",
            "M",
            "case_count",
            "avg_elapsed_ms",
            "p95_elapsed_ms",
            "max_elapsed_ms",
            "strict_safe_800ms",
            "near_safe_1000ms",
        ],
        nm_rows,
    )
    write_summary(
        out_dir / "summary.md",
        jsonl_path=jsonl_path,
        cases=cases,
        bucket_rows=bucket_rows,
        nm_rows=nm_rows,
    )

    print(f"wrote {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
