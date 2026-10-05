#!/usr/bin/env python3
"""Validate the single v040 evaluation without invoking the solver."""
import csv
import hashlib
import json
from pathlib import Path

from check_v037_results import log_values, verify_output, ERRORS

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v040_audit"
BIN = "v040_search_pair_compression"
PARENT_RUN = "20260928T020458+0900_v037_pair_transfer_compression_6086dd"


def main():
    audit = json.loads((AUDIT / "static_verification.json").read_text())
    for name, key in ((BIN, "solver_sha256"), ("v037_pair_transfer_compression", "parent_sha256")):
        assert hashlib.sha256((ROOT / f"src/bin/{name}.cpp").read_bytes()).hexdigest() == audit[key]
    for name, digest in audit["input_sha256"].items():
        assert hashlib.sha256((ROOT / "tools/in" / name).read_bytes()).hexdigest() == digest
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").read_text().splitlines()]
    current = [r for r in records if r["bin"] == BIN]
    parent = {r["case_name"]: r for r in records if r["run_id"] == PARENT_RUN}
    assert len(current) == len(parent) == 100
    assert len({r["run_id"] for r in current}) == 1
    assert {r["case_name"] for r in current} == parent.keys()
    cases = []
    for r in sorted(current, key=lambda x: x["case_name"]):
        assert r["status"] == "ok" and r["local"] and r["input_dir"] == "tools/in"
        path = ROOT / r["stdout_path"]
        T = verify_output(r["case_name"], path)
        c, t, diagnostics = log_values(path.with_suffix(".txt.err"))
        assert T == r["score"] == c["T"] == c["final_ops"] == c["validated_moves"] and c["E"] == 0
        assert c["final_pair_saved"] == c["pre_pair_ops"] - T
        assert c["pair_transfer_saved"] == c["search_pair_saved"] + c["final_pair_saved"]
        assert c["pair_transfer_saved"] == c["pair_transfer_merges"] + c["pair_transfer_cancellations"]
        assert c["search_pair_saved"] == sum(c[f"search_pair_{phase}_saved"] for phase in ("initial", "initial_reorder", "trial"))
        assert c["search_pair_accepted_saved"] <= c["search_pair_trial_saved"]
        assert c["lns_saved"] == c["pre_lns_ops"] - c["pre_pair_ops"]
        assert c["lns_saved"] == c["lns_initial_shortcut_saved"] + c["search_pair_initial_saved"] + c["lns_initial_reorder_saved"] + c["lns_best_saved"]
        assert t["search_limit"] == 1544.0
        p = parent[r["case_name"]]
        pc, _, _ = log_values((ROOT / p["stdout_path"]).with_suffix(".txt.err"))
        cases.append({**r, "parent_T": p["score"], "delta_T": T-p["score"],
                      "pre_lns_delta": c["pre_lns_ops"]-pc["pre_lns_ops"],
                      "parent_attempts": pc["lns_attempts"], "counts": c, "times_ms": t, "diagnostics": diagnostics})
    total = sum(r["score"] for r in cases)
    baseline = sum(r["score"] for r in parent.values())
    counts = {k: sum(r["counts"][k] for r in cases) for k in cases[0]["counts"] if k.startswith("search_pair_")}
    errors = {k: sum(r["counts"][k] for r in cases) for k in ERRORS}
    attempts = sum(r["counts"]["lns_attempts"] for r in cases)
    parent_attempts = sum(r["parent_attempts"] for r in cases)
    result = {
        "run_id": current[0]["run_id"], "parent_run_id": PARENT_RUN,
        "solver_sha256": audit["solver_sha256"], "verified_cases": 100,
        "total_T": total, "parent_T": baseline, "delta_T": total-baseline,
        "delta_percent": 100*(total/baseline-1),
        "wins": sum(r["delta_T"]<0 for r in cases), "draws": sum(r["delta_T"]==0 for r in cases),
        "losses": sum(r["delta_T"]>0 for r in cases), "case0000_T": cases[0]["score"],
        "max_elapsed_ms": max(r["elapsed"] for r in cases), "errors": errors,
        "diagnostic_count": sum(len(r["diagnostics"]) for r in cases), "counts": counts,
        "mean_pair_ms": sum(r["times_ms"]["search_pair_compression"] for r in cases)/100,
        "max_pair_ms": max(r["times_ms"]["search_pair_compression"] for r in cases),
        "pair_time_fraction": sum(r["times_ms"]["search_pair_compression"] for r in cases)/sum(r["times_ms"]["temporal_lns"] for r in cases),
        "accepted_compression_cases": sum(r["counts"]["search_pair_accepted_saved"]>0 for r in cases),
        "best_created_cases": sum(r["counts"]["search_pair_best_created"]>0 for r in cases),
        "attempts": attempts, "parent_attempts": parent_attempts, "attempts_delta_percent": 100*(attempts/parent_attempts-1),
        "pre_lns_delta_T": sum(r["pre_lns_delta"] for r in cases),
        "pre_lns_changed_cases": sum(r["pre_lns_delta"]!=0 for r in cases),
        "final_pair_saved": sum(r["counts"]["final_pair_saved"] for r in cases),
    }
    result["adopt"] = not any(errors.values()) and result["diagnostic_count"]==0 and result["max_elapsed_ms"]<=2000 and result["case0000_T"]<=43 and total<baseline and counts["search_pair_accepted_saved"]>0
    with (ROOT / "adhoc/v040_comparison.csv").open("w", newline="") as f:
        writer=csv.writer(f);writer.writerow(("case","parent_T","T","delta_T","pair_calls","accepted_saved","best_created","pair_ms"))
        for r in cases:
            c=r["counts"];writer.writerow((r["case_name"],r["parent_T"],r["score"],r["delta_T"],c["search_pair_calls"],c["search_pair_accepted_saved"],c["search_pair_best_created"],r["times_ms"]["search_pair_compression"]))
    (ROOT / "adhoc/v040_evaluation_summary.json").write_text(json.dumps({**result,"cases":cases},indent=2)+"\n")
    print(json.dumps(result,indent=2))


if __name__ == "__main__":
    main()
