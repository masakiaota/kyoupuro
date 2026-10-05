#!/usr/bin/env python3
"""v205の機構診断と保存結果の集計。analyzeはsolverを実行しない。"""
from collections import Counter
import csv
import json
from pathlib import Path
import subprocess
import sys

from build_v202_v203 import sha, save
from check_v037_results import ERRORS, log_values, verify_output
from prepare_v205 import AUDIT, CHILD, PARENT, ROOT

OUT = ROOT / "results/analysis/v205"


def frozen():
    info = json.loads((AUDIT / "preflight.json").read_text())
    for name, expected in info["solver_sha256"].items():
        assert sha(ROOT / f"src/bin/{name}.cpp") == expected, name
    for name, expected in info["input_sha256"].items():
        assert sha(ROOT / "tools/in" / name) == expected, name
    return info


def mechanism(case, path):
    T = verify_output(case, path)
    c, times, diagnostics = log_values(path.with_suffix(path.suffix+".err"))
    assert T == c["T"] == c["final_ops"] == c["validated_moves"] and c["E"] == 0
    assert not diagnostics and not any(c[k] for k in ERRORS)
    assert c["state_pool_free_at_end"] == c["state_slots"] == 4
    count = c["race_seed_count"]
    assert 1 <= count <= 8
    initial = {i:c[f"race_seed_{i}_initial_ops"] for i in range(count)}
    assert list(initial.values()) == sorted(initial.values())
    latest, active = initial.copy(), list(range(count))
    attempts, resumes, completed_rounds = 0, 0, 0
    for stage in range(3):
        if len(active) == 1:
            assert not c.get(f"race_round_{stage}_grown", 0)
            continue
        assert c[f"race_round_{stage}_grown"] == len(active)
        completed_rounds += 1
        if stage:
            resumes += len(active)
        for i in active:
            current = c[f"race_round_{stage}_seed_{i}_ops"]
            assert current <= latest[i]
            latest[i] = current
            attempts += c[f"race_round_{stage}_seed_{i}_attempts"]
        active.sort(key=lambda i: (latest[i], i))
        active = active[:(len(active)+1)//2]
        assert c[f"race_round_{stage}_survivors"] == len(active)
    assert len(active) == 1
    winner = active[0]
    assert c["race_winner_initial_rank"] == winner
    assert c["race_winner_changed"] == int(winner != 0)
    assert c["race_selection_saved"] == latest[0]-latest[winner]
    assert c["race_round_3_grown"] == 1
    assert c[f"race_round_3_seed_{winner}_ops"] <= latest[winner]
    latest[winner] = c[f"race_round_3_seed_{winner}_ops"]
    attempts += c[f"race_round_3_seed_{winner}_attempts"]
    resumes += count > 1
    assert c["race_resumes"] == resumes and c["lns_attempts"] == attempts
    assert all(latest[i] == c[f"race_seed_{i}_final_ops"] for i in range(count))
    assert T <= min(latest.values()) <= initial[0]
    enabled = int(c["floor_cells"] >= 193)
    assert c["selector_nn_enabled"] == enabled and (c["nn_calls"] > 0) == bool(enabled)
    return dict(T=T, counts=c, times_ms=times, selection_rounds=completed_rounds)


def diagnostic():
    info = frozen()
    marker = AUDIT / "diagnostic_started.json"
    assert not marker.exists()
    save(marker, dict(cases=["0000.txt", "0001.txt"], modes=["local", "production"]))
    rows = []
    for mode in ("local", "production"):
        binary = AUDIT / f"{CHILD}_{mode}"
        expected = next(b["binary_sha256"] for b in info["builds"] if b["mode"] == mode)
        assert sha(binary) == expected
        for case in ("0000.txt", "0001.txt"):
            out = AUDIT / f"{mode}_{case}"
            with (ROOT / "tools/in" / case).open() as stdin, out.open("w") as stdout, out.with_suffix(".txt.err").open("w") as stderr:
                subprocess.run([str(binary)], stdin=stdin, stdout=stdout, stderr=stderr,
                               check=True, cwd=ROOT, timeout=15)
            row = dict(mode=mode, case=case, T=verify_output(case, out))
            if mode == "local":
                row.update(mechanism(case, out))
            else:
                assert "diagnostic:" not in out.with_suffix(".txt.err").read_text()
            rows.append(row)
            print(f"{mode} {case}: 合法性と適用対象の機構確認に成功", flush=True)
    local = [r for r in rows if r["mode"] == "local"]
    assert any(r["counts"]["race_seed_count"] > 5 and r["selection_rounds"] == 3 for r in local)
    save(AUDIT / "diagnostic.json", rows)


def analyze():
    info = frozen()
    assert (AUDIT / "diagnostic.json").exists()
    OUT.mkdir(parents=True, exist_ok=True)
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()]
    selected = [r for r in records if r["bin"] == CHILD]
    assert len(selected) == 100 and len({r["run_id"] for r in selected}) == 1
    parent = {r["case_name"]:r for r in info["parent_records"][PARENT]}
    assert {r["case_name"] for r in selected} == parent.keys()
    rows, mechanisms, counts, times = [], [], Counter(), Counter()
    for r in sorted(selected, key=lambda r: r["case_name"]):
        assert r["status"] == "ok" and r["local"] and r["input_dir"] == "tools/in"
        m = mechanism(r["case_name"], ROOT / r["stdout_path"])
        assert m["T"] == r["score"]
        c = m["counts"]
        counts.update(c)
        times.update(m["times_ms"])
        rows.append(dict(case=r["case_name"], parent_T=parent[r["case_name"]]["score"], T=r["score"],
                         delta_T=r["score"]-parent[r["case_name"]]["score"], elapsed_ms=r["elapsed"],
                         seeds=c["race_seed_count"], winner_rank=c["race_winner_initial_rank"],
                         selection_rounds=m["selection_rounds"]))
        mechanisms.append(dict(case=r["case_name"], **m))
    assert any(r["seeds"] > 5 and r["selection_rounds"] == 3 for r in rows)
    total = sum(r["T"] for r in rows)
    parent_total = sum(r["parent_T"] for r in rows)
    elapsed = max(r["elapsed_ms"] for r in rows)
    result = dict(bin=CHILD, parent=PARENT, run_id=selected[0]["run_id"],
                  parent_run_id=next(iter(parent.values()))["run_id"], cases=100,
                  total_sum=total, total_avg=total/100, parent_total_sum=parent_total,
                  delta_sum=total-parent_total, delta_percent=100*(total/parent_total-1),
                  wins=sum(r["delta_T"]<0 for r in rows), draws=sum(r["delta_T"]==0 for r in rows),
                  losses=sum(r["delta_T"]>0 for r in rows), max_elapsed_ms=elapsed,
                  seed_counts=dict(Counter(r["seeds"] for r in rows)),
                  winner_ranks=dict(Counter(r["winner_rank"] for r in rows)),
                  additional_seed_wins=sum(r["winner_rank"]>=5 for r in rows),
                  third_selection_cases=sum(r["selection_rounds"]==3 for r in rows),
                  race_resumes=counts["race_resumes"], lns_attempts=counts["lns_attempts"],
                  grown_candidates={str(i):counts[f"race_round_{i}_grown"] for i in range(4)},
                  average_stage_ms={str(i):times[f"race_round_{i}"]/100 for i in range(4)},
                  average_construction_ms=times["construction"]/100,
                  nn_enabled_cases=counts["selector_nn_enabled"],
                  errors={k:counts[k] for k in ERRORS}, all_outputs_verified=True,
                  adopted=total<parent_total and elapsed<=2000)
    save(OUT / "comparison.json", result)
    save(OUT / "mechanism.json", mechanisms)
    with (OUT / "cases.csv").open("w", newline="") as f:
        writer=csv.DictWriter(f,fieldnames=list(rows[0])); writer.writeheader(); writer.writerows(rows)
    print(json.dumps(result,ensure_ascii=False,indent=2))


if __name__ == "__main__":
    {"diagnostic":diagnostic, "analyze":analyze}[sys.argv[1]]()
