#!/usr/bin/env python3
"""Summarize the saved v014 evaluation; never execute a solver."""

import hashlib
import json
from pathlib import Path
import re

from replay_slime_output import replay


ROOT = Path(__file__).resolve().parents[2]
VERSIONS = ("v013_unified", "v014_joint_delivery")


def main():
    records = [json.loads(line) for line in
               (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    runs = {}
    for version in VERSIONS:
        rows = [r for r in records if r["bin"] == version]
        run_id = rows[-1]["run_id"]
        runs[version] = {r["case_name"]: r for r in rows if r["run_id"] == run_id}
        assert len(runs[version]) == 100, (version, len(runs[version]))

    rows = []
    for case in sorted(runs[VERSIONS[0]]):
        row = {"case": case}
        for version in VERSIONS:
            record = runs[version][case]
            path = ROOT / record["stdout_path"]
            counts, times = {}, {}
            for kind, key, value in re.findall(
                    r"\[summary\.(count|time_ms)\] ([^=]+)=([^\n]+)",
                    path.with_suffix(path.suffix + ".err").read_text()):
                (counts if kind == "count" else times)[key] = float(value) if kind == "time_ms" else int(value)
            row[version] = {"score": record["score"], "elapsed_ms": record["elapsed"],
                            "status": record["status"], "counts": counts, "times_ms": times}
            if version == VERSIONS[1] and record["status"] == "ok":
                validation = replay(ROOT / "tools/in" / case, path)
                assert validation["metrics"]["E"] == 0
                assert validation["metrics"]["S"] == record["score"]
                row[version]["replay"] = validation["metrics"]
                if case == "0000.txt":
                    (ROOT / "adhoc/v014_case0000_trace.json").write_text(
                        json.dumps(validation, ensure_ascii=False, indent=2) + "\n")
        a, b = (row[v]["score"] for v in VERSIONS)
        row["delta"] = b - a if a is not None and b is not None else None
        rows.append(row)

    report = {"source_sha256": {v: hashlib.sha256((ROOT / "src/bin" / (v + ".cpp")).read_bytes()).hexdigest()
                                for v in VERSIONS}, "cases": rows}
    for label, subset in (("all100", rows), ("normal99", rows[1:])):
        summary = {"cases": len(subset)}
        for version in VERSIONS:
            values = [r[version] for r in subset]
            keys = sorted({key for v in values for key in v["counts"]})
            summary[version] = {
                "total_T": sum(v["score"] for v in values if v["score"] is not None),
                "max_elapsed_ms": max(v["elapsed_ms"] for v in values),
                "ok_cases": sum(v["status"] == "ok" for v in values),
                "counts_total": {key: sum(v["counts"].get(key, 0) for v in values) for key in keys},
                "counts_nonzero_cases": {key: sum(v["counts"].get(key, 0) > 0 for v in values) for key in keys},
                "counts_max": {key: max(v["counts"].get(key, 0) for v in values) for key in keys},
                "mean_cooperative_ms": sum(v["times_ms"].get("cooperative", 0) for v in values) / len(values),
            }
        deltas = [r["delta"] for r in subset if r["delta"] is not None]
        summary.update(delta=sum(deltas), wins=sum(d < 0 for d in deltas),
                       ties=sum(d == 0 for d in deltas), losses=sum(d > 0 for d in deltas))
        report[label] = summary
    target = ROOT / "adhoc/v014_evaluation_analysis.json"
    target.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    compact = {label: {key: value for key, value in report[label].items() if key not in VERSIONS}
               for label in ("all100", "normal99")}
    compact["case0000"] = {"v013": rows[0][VERSIONS[0]]["score"], "v014": rows[0][VERSIONS[1]]["score"],
                            "counts": rows[0][VERSIONS[1]]["counts"]}
    compact["mechanism_normal99"] = {
        key: report["normal99"][VERSIONS[1]][key] for key in
        ("counts_total", "counts_nonzero_cases", "max_elapsed_ms", "mean_cooperative_ms")}
    print(json.dumps(compact, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
