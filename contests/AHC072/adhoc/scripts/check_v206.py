#!/usr/bin/env python3
"""v206の固定・機構診断・集計。auditとanalyzeはsolverを実行しない。"""
import argparse
from collections import Counter
import csv
import json
from pathlib import Path
import subprocess
import sys

from build_v202_v203 import sha, save
from check_v037_results import ERRORS, log_values, verify_output
from prepare_v206 import AUDIT, CHILD, PARENT, ROOT
from tune_v204 import evaluator, mechanism as race_mechanism

OUT = ROOT / "results/analysis/v206"


def frozen():
    info = json.loads((AUDIT / "preflight.json").read_text())
    for name, expected in info["solver_sha256"].items():
        assert sha(ROOT / f"src/bin/{name}.cpp") == expected, name
    for name, expected in info["input_sha256"].items():
        assert sha(ROOT / "tools/in" / name) == expected, name
    for path, expected in info.get("support_sha256", {}).items():
        assert sha(ROOT / path) == expected, path
    for path, expected in info.get("parent_files_sha256", {}).items():
        assert sha(ROOT / path) == expected, path
    return info


def aggregate(metrics):
    counts, times = Counter(), Counter()
    for m in metrics:
        counts.update(m["counts"])
        times.update(m["times_ms"])
    return dict(cases=len(metrics), counts=dict(counts),
                average_times_ms={k: v/len(metrics) for k, v in times.items()})


def audit():
    info = frozen()
    assert not (AUDIT / "diagnostic_started.json").exists()
    assert not (OUT / "parent_metrics.json").exists()
    OUT.mkdir(parents=True, exist_ok=True)
    text = (ROOT / f"src/bin/{CHILD}.cpp").read_text()
    for token in ("FiniteRouter", "FinitePlanner", "FiniteStats", "FINAL_FINITE", "FinalReductionStats", "polish_final_reductions"):
        assert token not in text
        for mode in ("local", "production"):
            assert token not in (AUDIT / f"{CHILD}_{mode}.ii").read_text()
    assert "return 0.01*max(PROGRAM_TIME_LIMIT_SEC*0.02,exact_elapsed_sec)-spent;" in text
    assert "end=PROGRAM_TIME_LIMIT_SEC*1.013;" in text
    metrics, manifest = [], {}
    for r in info["parent_records"][PARENT]:
        answer = ROOT / r["stdout_path"]
        log = answer.with_suffix(answer.suffix+".err")
        c, times = race_mechanism(ROOT / "tools/in" / r["case_name"], answer, (5, 15))
        assert r["score"] == c["T"]
        metrics.append(dict(case=r["case_name"], counts=c, times_ms=times))
        for p in (answer, log):
            manifest[str(p.relative_to(ROOT))] = sha(p)
    save(OUT / "parent_metrics.json", {**aggregate(metrics), "details": metrics})
    paths = [Path(__file__), ROOT / "adhoc/scripts/prepare_v206.py",
             ROOT / "adhoc/scripts/tune_v204.py", ROOT / "adhoc/scripts/tune_v073.py",
             ROOT / "adhoc/scripts/check_v037_results.py", ROOT / "adhoc/scripts/check_v028_two_orders.py",
             ROOT / "scripts/eval.py", ROOT / "scripts/build_solver.sh"]
    info["support_sha256"] = {str(p.relative_to(ROOT)): sha(p) for p in paths}
    info["parent_files_sha256"] = manifest
    info["preregistration_sha256"] = sha(ROOT / "notes/experiments/v206.md")
    (AUDIT / "preregistration.md").write_bytes((ROOT / "notes/experiments/v206.md").read_bytes())
    save(AUDIT / "preflight.json", info)
    print("親の保存100件の合法性・計数・時間を照合し、ソース・入力・検証器・親出力を固定した。solver実行0回。")


def mechanism(case, answer):
    c, times = race_mechanism(ROOT / "tools/in" / case, answer, (5, 15))
    assert not any(k.startswith(("finite_", "final_reductions_", "search_reductions_finite")) for k in c)
    assert "final_reductions" not in times
    assert abs(times["lns_end_limit"]-1539.76) < 0.002
    assert times["search_limit"] == 1544.0
    total = c["search_reductions_heavy_calls"]
    for pos, kind in enumerate(("joint", "strict", "slack")):
        assert c[f"search_reductions_{kind}_calls"] == (total+2-pos)//3
    assert c["T"] <= c["pre_joint_ops"] <= c["pre_pair_ops"] <= c["pre_lns_ops"]
    assert c["lns_saved"] == c["pre_lns_ops"]-c["pre_pair_ops"]
    return dict(case=case, counts=c, times_ms=times)


def diagnostic():
    info = frozen()
    assert "support_sha256" in info
    marker = AUDIT / "diagnostic_started.json"
    assert not marker.exists()
    args = argparse.Namespace(bin_name=CHILD, jobs=2, wait_lock=False)
    with evaluator.acquire_eval_lock(args, "v206_diagnostic"):
        save(marker, dict(cases=["0000.txt", "0001.txt"], modes=["local", "production"]))
        rows = []
        for mode in ("local", "production"):
            binary = AUDIT / f"{CHILD}_{mode}"
            expected = next(b["binary_sha256"] for b in info["builds"] if b["mode"] == mode)
            assert sha(binary) == expected
            for case in ("0000.txt", "0001.txt"):
                answer = AUDIT / f"{mode}_{case}"
                with (ROOT / "tools/in" / case).open() as stdin, answer.open("w") as stdout, answer.with_suffix(".txt.err").open("w") as stderr:
                    subprocess.run([str(binary)], stdin=stdin, stdout=stdout, stderr=stderr,
                                   check=True, cwd=ROOT, timeout=15)
                row = dict(mode=mode, case=case, T=verify_output(case, answer))
                if mode == "local":
                    row.update(mechanism(case, answer))
                else:
                    assert "diagnostic:" not in answer.with_suffix(".txt.err").read_text()
                rows.append(row)
                print(f"{mode} {case}: 合法性と機構確認に成功", flush=True)
        assert any(r["counts"]["search_reductions_heavy_calls"] >= 3 for r in rows if r["mode"] == "local")
        save(AUDIT / "diagnostic.json", rows)


def analyze():
    info = frozen()
    assert (AUDIT / "diagnostic.json").exists()
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()]
    selected = [r for r in records if r["bin"] == CHILD]
    assert len(selected) == 100 and len({r["run_id"] for r in selected}) == 1
    parent = {r["case_name"]: r for r in info["parent_records"][PARENT]}
    assert {r["case_name"] for r in selected} == parent.keys()
    old = json.loads((OUT / "parent_metrics.json").read_text())
    pc = {r["case"]: r["counts"] for r in old["details"]}
    rows, metrics = [], []
    for r in sorted(selected, key=lambda r: r["case_name"]):
        assert r["status"] == "ok" and r["local"] and r["input_dir"] == "tools/in"
        m = mechanism(r["case_name"], ROOT / r["stdout_path"])
        assert m["counts"]["T"] == r["score"]
        metrics.append(m)
        c, case = m["counts"], r["case_name"]
        rows.append(dict(case=case, parent_T=parent[case]["score"], T=r["score"],
                         delta_T=r["score"]-parent[case]["score"], elapsed_ms=r["elapsed"],
                         pre_lns_delta=c["pre_lns_ops"]-pc[case]["pre_lns_ops"],
                         pre_pair_delta=c["pre_pair_ops"]-pc[case]["pre_pair_ops"],
                         attempts=c["lns_attempts"], parent_attempts=pc[case]["lns_attempts"],
                         seeds=c["race_seed_count"], winner_rank=c["race_winner_initial_rank"]))
    summary = aggregate(metrics)
    counts, times = summary["counts"], summary["average_times_ms"]
    parent_times = old["average_times_ms"]
    budget_back = times["temporal_lns"]-times["search_reductions_heavy"]-(
        parent_times["temporal_lns"]-parent_times["search_reductions_heavy"])
    total, before = sum(r["T"] for r in rows), sum(r["parent_T"] for r in rows)
    maximum = max(r["elapsed_ms"] for r in rows)
    result = dict(bin=CHILD, parent=PARENT, run_id=selected[0]["run_id"],
                  parent_run_id=next(iter(parent.values()))["run_id"], cases=len(rows),
                  total_sum=total, total_avg=total/100, parent_total_sum=before,
                  delta_sum=total-before, delta_percent=100*(total/before-1),
                  wins=sum(r["delta_T"]<0 for r in rows), draws=sum(r["delta_T"]==0 for r in rows),
                  losses=sum(r["delta_T"]>0 for r in rows), max_elapsed_ms=maximum,
                  pre_lns_delta=sum(r["pre_lns_delta"] for r in rows),
                  pre_pair_delta=sum(r["pre_pair_delta"] for r in rows),
                  lns_attempts=counts["lns_attempts"], parent_lns_attempts=old["counts"]["lns_attempts"],
                  lns_excluding_heavy_increase_ms=budget_back,
                  average_times_ms=times, parent_average_times_ms=parent_times,
                  seed_counts=dict(Counter(r["seeds"] for r in rows)),
                  winner_ranks=dict(Counter(r["winner_rank"] for r in rows)),
                  errors={k:counts[k] for k in ERRORS}, all_outputs_verified=True,
                  adopted=total<before and maximum<=2000 and budget_back>0)
    save(OUT / "comparison.json", result)
    save(OUT / "mechanism.json", {**summary, "details": metrics})
    with (OUT / "cases.csv").open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    print(json.dumps({k:v for k,v in result.items() if "average_times" not in k}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    {"audit":audit, "diagnostic":diagnostic, "analyze":analyze}[sys.argv[1]]()
