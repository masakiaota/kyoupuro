#!/usr/bin/env python3
from __future__ import annotations
import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


@dataclass
class Stats:
    total: int = 0
    ok: int = 0
    fail: int = 0
    tle: int = 0
    elapsed_total_ms: int = 0
    max_elapsed_ms: int = 0
    score_total: int = 0
    score_count: int = 0

    def update(self, record: dict) -> None:
        elapsed_ms = int(record["elapsed_ms"])
        self.total += 1
        if record["status"] == "ok":
            self.ok += 1
        else:
            self.fail += 1
        if record["tle_2sec"]:
            self.tle += 1
        self.elapsed_total_ms += elapsed_ms
        self.max_elapsed_ms = max(self.max_elapsed_ms, elapsed_ms)
        if record["score"] is not None:
            self.score_total += int(record["score"])
            self.score_count += 1

    def row(self, key: str) -> list[str]:
        avg_elapsed = self.elapsed_total_ms / self.total if self.total else 0.0
        if self.score_count:
            avg_score = f"{self.score_total / self.score_count:.1f}"
        else:
            avg_score = "-"
        return [
            key,
            str(self.total),
            str(self.ok),
            str(self.fail),
            str(self.tle),
            f"{avg_elapsed:.1f}",
            str(self.max_elapsed_ms),
            avg_score,
            str(self.score_total),
        ]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Summarize JSONL output from scripts/eval_jsonl.py."
    )
    parser.add_argument(
        "jsonl",
        nargs="*",
        help="JSONL files to summarize. Defaults to the latest serial_eval_*.jsonl.",
    )
    return parser.parse_args()


def pick_default_jsonl() -> list[Path]:
    candidates = sorted((ROOT / "results" / "analysis").glob("serial_eval_*.jsonl"))
    if not candidates:
        raise SystemExit("error: no serial_eval_*.jsonl found under results/analysis")
    return [candidates[-1]]


def resolve_jsonl_paths(raw_paths: list[str]) -> list[Path]:
    if not raw_paths:
        return pick_default_jsonl()
    resolved = []
    for raw_path in raw_paths:
        path = Path(raw_path).expanduser().resolve()
        if not path.is_file():
            raise SystemExit(f"error: jsonl not found: {raw_path}")
        resolved.append(path)
    return resolved


def read_records(paths: list[Path]) -> list[dict]:
    records: list[dict] = []
    for path in paths:
        with path.open(encoding="utf-8") as fh:
            for line_no, line in enumerate(fh, start=1):
                if not line.strip():
                    continue
                try:
                    record = json.loads(line)
                except json.JSONDecodeError as exc:
                    raise SystemExit(
                        f"error: failed to parse JSON at {path}:{line_no}: {exc}"
                    ) from exc
                records.append(record)
    if not records:
        raise SystemExit("error: no records found in the given JSONL files")
    return records


def build_stats(records: list[dict], key_name: str) -> dict[str, Stats]:
    stats: dict[str, Stats] = {}
    for record in records:
        key = str(record[key_name])
        if key not in stats:
            stats[key] = Stats()
        stats[key].update(record)
    return stats


def format_table(headers: list[str], rows: list[list[str]]) -> str:
    widths = [len(header) for header in headers]
    for row in rows:
        for idx, cell in enumerate(row):
            widths[idx] = max(widths[idx], len(cell))

    def format_row(row: list[str]) -> str:
        return "  ".join(cell.ljust(widths[idx]) for idx, cell in enumerate(row))

    lines = [format_row(headers), format_row(["-" * width for width in widths])]
    lines.extend(format_row(row) for row in rows)
    return "\n".join(lines)


def print_section(title: str, stats: dict[str, Stats]) -> None:
    headers = [
        title,
        "total",
        "ok",
        "fail",
        "tle",
        "avg_ms",
        "max_ms",
        "avg_score",
        "sum_score",
    ]
    rows = [stat.row(key) for key, stat in stats.items()]
    print(f"\n{title}")
    print(format_table(headers, rows))


def main() -> int:
    args = parse_args()
    paths = resolve_jsonl_paths(args.jsonl)
    records = read_records(paths)

    print("sources")
    for path in paths:
        print(path)
    print(f"\nrecords: {len(records)}")

    overall = Stats()
    for record in records:
        overall.update(record)
    print_section("overall", {"all": overall})
    print_section("bin", build_stats(records, "bin"))
    print_section("dataset", build_stats(records, "dataset"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
