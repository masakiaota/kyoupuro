#!/usr/bin/env python3
"""Summarize v004_trace stderr logs against v002/v004 scores."""

from __future__ import annotations

import argparse
import csv
import re
import sys
from pathlib import Path


DEFAULT_V002_LABEL = "greedy near-lexico a1e7 b1e3 g300"
DEFAULT_V004_LABEL = "beam W=128 expand=32 v002-score hash-dedup"

CASE_RE = re.compile(r"(\d{4})(?:\.txt)?")
TRACE_RE = re.compile(
    r"^\[trace\] depth=(?P<depth>\d+) target=(?P<target>\d+) "
    r"parent=(?P<parent>\S+) generated=(?P<generated>\S+) "
    r"kept_hash=(?P<kept_hash>\S+) kept_exact=(?P<kept_exact>\S+) .*"
    r"child_eval=Evaluator \{ score_key: (?P<child_score>\d+), "
    r"total_cost: (?P<child_cost>\d+), tie_break: (?P<child_tie>\d+) \} "
    r"kept_eval=(?P<kept_eval>.*?) "
    r"worst_kept=(?P<worst>.*)$"
)
DROP_RE = re.compile(
    r"^\[trace\.drop\] depth=(?P<depth>\d+) target=(?P<target>\d+) "
    r"parent=(?P<parent>\S+) generated=(?P<generated>\S+) "
    r"action=(?P<action>.*?) child_hash=(?P<child_hash>[0-9a-f]+) "
    r"child_eval=Evaluator \{ score_key: (?P<child_score>\d+), "
    r"total_cost: (?P<child_cost>\d+), tie_break: (?P<child_tie>\d+) \} "
    r"worst_kept=(?P<worst>.*)$"
)
EVAL_RE = re.compile(r"Evaluator \{ score_key: (\d+), total_cost: (\d+), tie_break: (\d+) \}")
SUMMARY_RE = re.compile(r"^\[summary\.count\] (?P<key>[^=]+)=(?P<value>.+)$")


def infer_case(path: Path) -> str:
    match = CASE_RE.search(path.name)
    if match:
        return f"{match.group(1)}.txt"
    return path.name


def parse_eval(text: str) -> tuple[int | None, int | None]:
    match = EVAL_RE.search(text)
    if not match:
        return None, None
    return int(match.group(1)), int(match.group(2))


def classify(parent: str, generated: str, kept_hash: str | None) -> str:
    if parent == "missing":
        return "parent_already_dropped"
    if generated == "false":
        return "local_expand_pruned"
    if kept_hash == "false":
        return "beam_width_pruned"
    if kept_hash == "true":
        return "kept_or_hash_equivalent"
    return "unknown"


def parse_log(path: Path) -> dict[str, object]:
    traces: list[dict[str, str]] = []
    first_drop: dict[str, str] | None = None
    summary: dict[str, str] = {}

    with path.open() as f:
        for raw_line in f:
            line = raw_line.rstrip("\n")
            match = TRACE_RE.match(line)
            if match:
                traces.append(match.groupdict())
                continue
            match = DROP_RE.match(line)
            if match and first_drop is None:
                first_drop = match.groupdict()
                continue
            match = SUMMARY_RE.match(line)
            if match:
                summary[match.group("key")] = match.group("value")

    if first_drop is None:
        last = traces[-1] if traces else None
        reason = "no_drop" if last else "no_trace"
        return {
            "case": infer_case(path),
            "path": str(path),
            "reason": reason,
            "depth": "",
            "target": "",
            "parent": "",
            "generated": "",
            "kept_hash": "",
            "child_score": "",
            "child_cost": "",
            "worst_score": "",
            "worst_cost": "",
            "total_cost": summary.get("total_cost", ""),
            "action": "",
        }

    sibling_trace = next(
        (
            t
            for t in traces
            if t["depth"] == first_drop["depth"] and t["target"] == first_drop["target"]
        ),
        None,
    )
    kept_hash = sibling_trace["kept_hash"] if sibling_trace else None
    worst_score, worst_cost = parse_eval(first_drop["worst"])
    return {
        "case": infer_case(path),
        "path": str(path),
        "reason": classify(first_drop["parent"], first_drop["generated"], kept_hash),
        "depth": int(first_drop["depth"]),
        "target": int(first_drop["target"]),
        "parent": first_drop["parent"],
        "generated": first_drop["generated"],
        "kept_hash": "" if kept_hash is None else kept_hash,
        "child_score": int(first_drop["child_score"]),
        "child_cost": int(first_drop["child_cost"]),
        "worst_score": "" if worst_score is None else worst_score,
        "worst_cost": "" if worst_cost is None else worst_cost,
        "total_cost": summary.get("total_cost", ""),
        "action": first_drop["action"],
    }


def load_scores(
    path: Path | None, v002_label: str, v004_label: str
) -> dict[str, tuple[int, int, int]]:
    if path is None:
        return {}
    with path.open() as f:
        rows = list(csv.DictReader(f))
    try:
        v002 = [r for r in rows if r["bin"] == "v002_greedy" and r["label"] == v002_label][-1]
        v004 = [r for r in rows if r["bin"] == "v004_beam" and r["label"] == v004_label][-1]
    except IndexError as exc:
        raise SystemExit("requested score rows were not found in score_detail.csv") from exc

    scores: dict[str, tuple[int, int, int]] = {}
    for key, value in v002.items():
        if not key.endswith(".txt"):
            continue
        v002_score = int(value)
        v004_score = int(v004[key])
        scores[key] = (v002_score, v004_score, v004_score - v002_score)
    return scores


def print_table(rows: list[dict[str, object]], scores: dict[str, tuple[int, int, int]]) -> None:
    header = [
        "case",
        "v002",
        "v004",
        "diff",
        "drop_depth",
        "target",
        "reason",
        "parent",
        "generated",
        "kept_hash",
        "child_score",
        "child_cost",
        "worst_score",
        "worst_cost",
        "total_cost",
        "action",
    ]
    writer = csv.writer(sys.stdout)
    writer.writerow(header)
    for row in rows:
        case = str(row["case"])
        score = scores.get(case, ("", "", ""))
        values = [
            case,
            score[0],
            score[1],
            score[2],
            row["depth"],
            row["target"],
            row["reason"],
            row["parent"],
            row["generated"],
            row["kept_hash"],
            row["child_score"],
            row["child_cost"],
            row["worst_score"],
            row["worst_cost"],
            row["total_cost"],
            row["action"],
        ]
        writer.writerow(values)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("logs", nargs="+", type=Path)
    parser.add_argument("--score-detail", type=Path)
    parser.add_argument("--v002-label", default=DEFAULT_V002_LABEL)
    parser.add_argument("--v004-label", default=DEFAULT_V004_LABEL)
    args = parser.parse_args()

    scores = load_scores(args.score_detail, args.v002_label, args.v004_label)
    rows = [parse_log(path) for path in args.logs]
    rows.sort(key=lambda row: str(row["case"]))
    print_table(rows, scores)


if __name__ == "__main__":
    main()
