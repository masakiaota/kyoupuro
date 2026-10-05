#!/usr/bin/env python3
"""Aggregate the frozen pre-experiment profile without running solver code."""
import collections
import csv
import json
from pathlib import Path

audit = Path(__file__).resolve().parents[2] / "adhoc/v024_audit"
rows = list(csv.DictReader((audit / "reinsertion_profile.csv").open()))
summary, by_outcome = {}, {}
metrics = ("total_ns", "setup_ns", "spatial_ns", "event_ns", "seed_ns",
           "reconstruct_ns", "layers", "scanned", "eligible")
for kind in ("0", "1"):
    selected = [r for r in rows if r["kind"] == kind and r["case"] != "0000"]
    out = {}
    for stage in ("extract", "insert"):
        group = [r for r in selected if (r["stage"] == "0") == (stage == "extract")]
        out[stage] = {}
        for status in ("all", "success", "failure"):
            chosen = group if status == "all" else [r for r in group if (r["success"] == "1") == (status == "success")]
            out[stage][status] = {"calls": len(chosen), **{k: sum(float(r[k]) for r in chosen) for k in metrics}}
    out["failure_stage"] = dict(collections.Counter(r["stage"] for r in selected if r["success"] == "0"))
    summary[kind] = out
    groups = collections.defaultdict(list)
    for r in selected:
        groups[(r["case"], r["candidate"])].append(r)
    buckets = collections.defaultdict(lambda: dict(candidates=0, total_ns=0, extract_ns=0,
        insert_ns=0, insert_calls=0, successful_insert_ns=0))
    for group in groups.values():
        status = "extract_failed" if group[0]["success"] == "0" else ("insert_failed" if group[-1]["success"] == "0" else "completed")
        bucket = buckets[status]
        bucket["candidates"] += 1
        for row in group:
            elapsed = float(row["total_ns"])
            bucket["total_ns"] += elapsed
            if row["stage"] == "0":
                bucket["extract_ns"] += elapsed
            else:
                bucket["insert_ns"] += elapsed
                bucket["insert_calls"] += 1
                if row["success"] == "1":
                    bucket["successful_insert_ns"] += elapsed
    by_outcome[kind] = dict(buckets)
for name, data in (("reinsertion_profile_summary.json", summary), ("profile_by_candidate_outcome.json", by_outcome)):
    (audit / name).write_text(json.dumps(data, indent=2) + "\n")
