#!/usr/bin/env python3
"""Analyze saved logs and completed routes without executing solver code."""
from collections import Counter, defaultdict
import csv
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v028_behavior_analysis"
PREFIX = "lns_two_order_"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ratio(n, d):
    return 100*n/d if d else None


def schedule_accounting(data):
    total, exceptions = Counter(), []
    for case in data["cases"]:
        if case["case_name"] == "0000.txt":
            continue
        c = case["counts"]
        assert c["lns_deadlines"] == c["lns_errors"] == 0
        loops = c["lns_attempts"]+c["lns_dependency_selected"]-c["lns_dependency_generated"]+c["lns_dependency_cooldown"]
        # Reordering preempts the other neighborhoods at multiples of 113.
        assert c["lns_reorder_attempts"] == 1+loops//113
        paired = loops//17-loops//(17*113)
        assert paired == c["lns_paired_attempts"]
        block = loops//7-loops//(7*113)-loops//(7*17)+loops//(7*17*113)
        assert block >= c["lns_block_attempts"]
        total.update({"loops": loops, "paired_scheduled": paired, "paired_attempted": c["lns_paired_attempts"],
                      "block_scheduled": block, "block_attempted": c["lns_block_attempts"]})
        if block != c["lns_block_attempts"]:
            exceptions.append({"case": case["case_name"], "block_skipped": block-c["lns_block_attempts"]})
    return {"totals": total, "block_skipped_cases": exceptions,
            "paired_time_cap_blocked_attempts": 0,
            "block_missing_reason": "An empty packet candidate bank or its time cap; logs do not separate these."}


def main():
    OUT.mkdir(exist_ok=True)
    parent = json.loads((ROOT / "adhoc/v026_evaluation_summary.json").read_text())
    child = json.loads((ROOT / "adhoc/v028_evaluation_summary.json").read_text())
    for data in (parent, child):
        assert digest(ROOT / f"src/bin/{data['bin']}.cpp") == data["solver_sha256"]
        for case in data["cases"]:
            path = ROOT / case["stdout_path"]
            log = path.with_name(path.name+".err").read_text()
            counts = {k: int(v) for k, v in re.findall(r"\[summary.count\] (\w+)=(-?\d+)", log)}
            times = {k: float(v) for k, v in re.findall(r"\[summary.time_ms\] (\w+)=([0-9.]+)", log)}
            assert counts == case["counts"] and times == case["times_ms"]
    inputs = [ROOT / "adhoc/v026_evaluation_summary.json", ROOT / "adhoc/v028_evaluation_summary.json",
              ROOT / "adhoc/v028_audit/diagnostic.csv", ROOT / "adhoc/v028_audit/diagnostic_moves.jsonl"]
    original = {}
    for case in parent["cases"]:
        path = ROOT / case["stdout_path"]
        original[case["case_name"]] = [[int(i), int(j), int(k), d, int(l)]
            for i, j, k, d, l in (line.split() for line in path.read_text().splitlines())]
        assert len(original[case["case_name"]]) == case["score"]
    with inputs[2].open() as f:
        diagnostics = list(csv.DictReader(f))
    by_key = {(r["case"], r["hash"]): r for r in diagnostics}
    counts, by_size, by_primary = Counter(), defaultdict(Counter), defaultdict(Counter)
    comparison_rows, examples, gate_losses = [], [], []
    for line in inputs[3].open():
        record = json.loads(line)
        row = by_key.pop((record["case"], record["hash"]))
        p, a, chosen = record["primary"], record["alternate"], record["chosen"]
        old = original[record["case"]]
        current_T = len(old)
        expected = a if a is not None and (p is None or len(a) < len(p)) else p
        assert chosen == expected
        for kind in ("primary", "alternate", "chosen"):
            moves = record[kind]
            assert int(row[kind+"_T"]) == (len(moves) if moves is not None else -1)
        if record["case"] == "0000.txt":
            continue
        status = "extract_failed" if not int(row["extracted"]) else "primary_failed" if p is None else (
            "primary_duplicate" if p == old else "primary_shorter" if len(p) < current_T else
            "primary_equal_different" if len(p) == current_T else "primary_longer")
        item = Counter(candidates=1, extracted=int(row["extracted"]),
                       alternate_selected=int(row["alternate_selected"]), rescued=int(row["rescued"]),
                       raw_saved=int(row["raw_saved"]),
                       primary_completed=int(p is not None), alternate_completed=int(a is not None))
        # A static counterfactual on already saved completions: the original
        # first order is unchanged, and only whether to use order two differs.
        skip = p is not None and len(p) <= current_T
        filtered = p if skip else chosen
        changed = filtered != chosen
        assert not changed or len(filtered) > len(chosen)
        loss = len(filtered)-len(chosen) if changed else 0
        item["skip_if_primary_not_longer"] += skip
        item["filtered_result_changed"] += changed
        item["filtered_raw_loss"] += loss
        if changed:
            gate_losses.append({"case": record["case"], "hash": record["hash"], "size": int(row["size"]),
                                "current_T": current_T, "primary_T": len(p), "alternate_T": len(a)})
        if p is not None and a is not None:
            item["both_completed"] += 1
            if len(p) == len(a):
                item["equal_length"] += 1
                item["equal_length_different_moves"] += p != a
                if p == old and a != old:
                    item["duplicate_primary_equal_alternative"] += 1
                    examples.append({"case": record["case"], "hash": record["hash"],
                                     "size": int(row["size"]), "T": current_T})
            item["identical_moves"] += p == a
        item["alternate_below_current"] += a is not None and len(a) < current_T
        for counter in (counts, by_size[int(row["size"])], by_primary[status]):
            counter.update(item)
        comparison_rows.append({"case": record["case"], "hash": record["hash"], "size": row["size"],
                                "primary_status": status, "current_T": current_T,
                                "primary_T": row["primary_T"], "alternate_T": row["alternate_T"],
                                "chosen_T": row["chosen_T"], "skip_if_primary_not_longer": int(skip),
                                "filtered_result_changed": int(changed), "filtered_raw_loss": loss,
                                "equal_length_different_moves": item["equal_length_different_moves"]})
    assert not by_key
    c, p = child["generated_99"], parent["generated_99"]
    cc, pc = c["counts_sum"], p["counts_sum"]
    phases = {
        "block": ("lns_block_attempts", "lns_block_search", .22),
        "paired": ("lns_paired_attempts", "lns_paired_search", .11),
        "dependency": ("lns_dependency_attempts", "lns_dependency", None),
    }
    scheduling = {}
    for name, (attempts, timer, cap) in phases.items():
        values = [case["times_ms"][timer]/case["times_ms"]["temporal_lns"]*100
                  for case in child["cases"] if case["case_name"] != "0000.txt"]
        scheduling[name] = {
            "parent_attempts": pc[attempts], "child_attempts": cc[attempts],
            "attempt_change_percent": 100*(cc[attempts]/pc[attempts]-1),
            "parent_mean_ms": p["mean_times_ms"][timer], "child_mean_ms": c["mean_times_ms"][timer],
            "share_of_total_lns_percent": 100*c["mean_times_ms"][timer]/c["mean_times_ms"]["temporal_lns"],
            "max_case_share_of_total_lns_percent": max(values), "source_time_cap_fraction": cap,
        }
    completed = cc[PREFIX+"primary_finished"]
    failed = cc[PREFIX+"primary_failed"]
    alternate = cc[PREFIX+"alternate_selected"]
    result = {
        "scope": "Existing logs and completed diagnostic routes only; no solver execution, no new routes.",
        "source_run_ids": [parent["run_id"], child["run_id"]],
        "input_hashes": {str(path.relative_to(ROOT)): digest(path) for path in inputs},
        "solver_hashes": {data["bin"]: data["solver_sha256"] for data in (parent, child)},
        "evaluation_99": {
            "attempts": cc[PREFIX+"attempts"], "primary_completed": completed, "primary_failed": failed,
            "alternate_selected": alternate, "shorter": cc[PREFIX+"shorter"], "rescued": cc[PREFIX+"rescued"],
            "selected_percent": ratio(alternate, cc[PREFIX+"attempts"]),
            "shorter_given_primary_completed_percent": ratio(cc[PREFIX+"shorter"], completed),
            "rescued_given_primary_failed_percent": ratio(cc[PREFIX+"rescued"], failed),
            "primary_completed_share_of_alternate_selection_percent": ratio(cc[PREFIX+"shorter"], alternate),
            "alternate_mean_ms": c["mean_times_ms"][PREFIX+"alternate_search"],
            "alternate_share_of_total_lns_percent": ratio(c["mean_times_ms"][PREFIX+"alternate_search"], c["mean_times_ms"]["temporal_lns"]),
            "scheduling": scheduling,
            "schedule_accounting": {"parent": schedule_accounting(parent), "child": schedule_accounting(child)},
        },
        "diagnostic_99": {"total": counts, "by_size": by_size, "by_primary_status": by_primary,
                          "skip_fraction_of_extracted_percent": ratio(counts["skip_if_primary_not_longer"], counts["extracted"]),
                          "filtered_raw_loss_share_percent": ratio(counts["filtered_raw_loss"], counts["raw_saved"]),
                          "gate_losses": gate_losses, "equal_route_examples": examples},
        "limits": [
            "The diagnostic samples final v026 solutions, with fixed weights and at most 64 sets per case; it is not the search-time distribution.",
            "No per-attempt timings or best improvements cross-tabulated by primary length, size, or dependency kind were logged.",
            "The gate's call count and raw selected routes can be compared exactly on this dataset; its runtime or final score cannot.",
            "Equal-length distinct routes may merge or change ranking under smoothRoutes; that has not been evaluated here.",
            "The paired time cap is checked by exact modulo counts, not by the final time share. The 13 missing block attempts cannot be separated into time-cap skips and empty-bank skips.",
            "Dependency timings overlap part of the two-order timings; those durations must not be summed as disjoint phases.",
        ],
    }
    (OUT / "summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2)+"\n")
    with (OUT / "diagnostic_comparison.csv").open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(comparison_rows[0]))
        writer.writeheader();writer.writerows(comparison_rows)
    print(json.dumps({"evaluation_99": result["evaluation_99"],
                      "diagnostic_totals": counts,
                      "skip_fraction_percent": result["diagnostic_99"]["skip_fraction_of_extracted_percent"],
                      "filtered_raw_loss_share_percent": result["diagnostic_99"]["filtered_raw_loss_share_percent"],
                      "gate_losses": gate_losses}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
