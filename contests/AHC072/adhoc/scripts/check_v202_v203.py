#!/usr/bin/env python3
"""固定済み2版の診断と保存結果の比較。analyzeはsolverを実行しない。"""
from collections import Counter
import csv
import json
from pathlib import Path
import subprocess
import sys

from build_v202_v203 import AUDIT, PAIRS, ROOT, save, sha
from check_v037_results import ERRORS, log_values, verify_output

OUT = ROOT / "results/analysis/v202_v203"


def frozen():
    info = json.loads((AUDIT / "preflight.json").read_text())
    for name, digest in info["solver_sha256"].items():
        assert sha(ROOT / f"src/bin/{name}.cpp") == digest, name
    for name, digest in info["input_sha256"].items():
        assert sha(ROOT / "tools/in" / name) == digest, name
    return info


def check_mechanism(case, path, child):
    T = verify_output(case, path)
    c, times, diagnostics = log_values(path.with_suffix(path.suffix + ".err"))
    assert T == c["T"] == c["final_ops"] == c["validated_moves"] and c["E"] == 0
    assert not diagnostics and not any(c[key] for key in ERRORS), (case, diagnostics)
    assert c["state_pool_free_at_end"] == c["state_slots"] == 4
    count = c["race_seed_count"]
    assert 1 <= count <= 4
    initial = {i: c[f"race_seed_{i}_initial_ops"] for i in range(count)}
    assert list(initial.values()) == sorted(initial.values())
    latest = initial.copy()
    active = list(range(count))
    attempts = 0
    resumes = 0
    for round_id in range(2):
        if len(active) == 1:
            break
        assert c[f"race_round_{round_id}_grown"] == len(active)
        if round_id:
            resumes += len(active)
        for i in active:
            n = c[f"race_round_{round_id}_seed_{i}_ops"]
            assert n <= latest[i]
            latest[i] = n
            attempts += c[f"race_round_{round_id}_seed_{i}_attempts"]
        active.sort(key=lambda i: (latest[i], i))
        active = active[:(len(active) + 1) // 2 if round_id == 0 else 1]
        assert c[f"race_round_{round_id}_survivors"] == len(active)
    winner = active[0]
    assert c["race_winner_initial_rank"] == winner
    assert c["race_winner_changed"] == int(winner != 0)
    assert c["race_selection_saved"] == latest[0] - latest[winner]
    assert c["race_round_2_grown"] == 1
    assert c["race_round_2_seed_" + str(winner) + "_ops"] <= latest[winner]
    latest[winner] = c["race_round_2_seed_" + str(winner) + "_ops"]
    attempts += c["race_round_2_seed_" + str(winner) + "_attempts"]
    resumes += count > 1
    assert c["race_resumes"] == resumes
    assert attempts == c["lns_attempts"]
    for i in range(count):
        assert latest[i] == c[f"race_seed_{i}_final_ops"]
    assert T <= min(latest.values()) <= initial[0]
    if child.startswith("v203"):
        expected = int(c["floor_cells"] >= 193)
        assert c["selector_nn_enabled"] == expected
        assert (c["nn_calls"] > 0) == bool(expected)
    return {"T": T, "counts": c, "times_ms": times}


def diagnostic():
    info = frozen()
    marker = AUDIT / "diagnostic_started.json"
    assert not marker.exists(), "diagnostic already started"
    save(marker, {"cases": ["0000.txt", "0001.txt"], "modes": ["local", "production"]})
    checked = []
    for _, child in PAIRS:
        for mode in ("local", "production"):
            binary = AUDIT / f"{child}_{mode}"
            expected = next(b["binary_sha256"] for b in info["builds"] if b["bin"] == child and b["mode"] == mode)
            assert sha(binary) == expected
            for case in ("0000.txt", "0001.txt"):
                out = AUDIT / f"{child}_{mode}_{case}"
                with (ROOT / "tools/in" / case).open() as stdin, out.open("w") as stdout, out.with_suffix(out.suffix + ".err").open("w") as stderr:
                    subprocess.run([str(binary)], stdin=stdin, stdout=stdout, stderr=stderr,
                                   cwd=ROOT, check=True, timeout=15)
                row = {"bin": child, "mode": mode, "case": case, "T": verify_output(case, out)}
                if mode == "local":
                    row.update(check_mechanism(case, out, child))
                    assert row["counts"]["race_seed_count"] > 1
                else:
                    assert "diagnostic:" not in out.with_suffix(out.suffix + ".err").read_text()
                checked.append(row)
                print(f"{child} {mode} {case}: legality and applicable mechanism checks passed", flush=True)
    save(AUDIT / "diagnostic.json", checked)


def analyze():
    info = frozen()
    assert (AUDIT / "diagnostic.json").exists()
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()]
    OUT.mkdir(parents=True, exist_ok=True)
    summaries = {}
    for parent, child in PAIRS:
        selected = [r for r in records if r["bin"] == child]
        assert len(selected) == 100 and len({r["run_id"] for r in selected}) == 1
        assert {r["case_name"] for r in selected} == set(info["input_sha256"])
        baseline = {r["case_name"]: r for r in info["parent_records"][parent]}
        rows = []
        for r in sorted(selected, key=lambda r: r["case_name"]):
            assert r["status"] == "ok" and r["local"] and r["input_dir"] == "tools/in"
            result = check_mechanism(r["case_name"], ROOT / r["stdout_path"], child)
            assert result["T"] == r["score"]
            rows.append({**r, **result, "parent_T": baseline[r["case_name"]]["score"],
                         "delta_T": r["score"] - baseline[r["case_name"]]["score"]})
        sums = Counter()
        for row in rows:
            sums.update(row["counts"])
        assert sums["race_resumes"] > 0 and sums["race_round_0_grown"] > 0
        total = sum(r["score"] for r in rows)
        parent_total = sum(r["score"] for r in baseline.values())
        summary = {
            "bin": child, "parent": parent, "run_id": selected[0]["run_id"],
            "parent_run_id": next(iter(baseline.values()))["run_id"], "cases": 100,
            "total_sum": total, "total_avg": total / 100,
            "parent_total_sum": parent_total, "parent_total_avg": parent_total / 100,
            "delta_sum": total - parent_total, "delta_avg": (total - parent_total) / 100,
            "delta_percent": 100 * (total / parent_total - 1),
            "wins": sum(r["delta_T"] < 0 for r in rows),
            "draws": sum(r["delta_T"] == 0 for r in rows),
            "losses": sum(r["delta_T"] > 0 for r in rows),
            "max_elapsed_ms": max(r["elapsed"] for r in rows),
            "avg_elapsed_ms": sum(r["elapsed"] for r in rows) / 100,
            "seed_counts": dict(Counter(r["counts"]["race_seed_count"] for r in rows)),
            "winner_initial_ranks": dict(Counter(r["counts"]["race_winner_initial_rank"] for r in rows)),
            "selection_changed_cases": sums["race_winner_changed"],
            "race_resumes": sums["race_resumes"], "lns_attempts": sums["lns_attempts"],
            "selection_saved": sums["race_selection_saved"],
            "grown_candidates": {str(i): sums[f"race_round_{i}_grown"] for i in range(3)},
            "errors": {key: sums[key] for key in ERRORS},
            "nn_enabled_cases": sums["selector_nn_enabled"] if child.startswith("v203") else None,
            "all_outputs_verified": True,
        }
        summary["adopt"] = total < parent_total and summary["max_elapsed_ms"] <= 2000
        summaries[child] = summary
        save(OUT / f"{child}_mechanism.json", rows)
        with (OUT / f"{child}_cases.csv").open("w", newline="") as out:
            writer = csv.writer(out)
            writer.writerow(["case", "parent_T", "T", "delta_T", "elapsed_ms", "seed_count", "winner_initial_rank"])
            for row in rows:
                writer.writerow([row["case_name"], row["parent_T"], row["T"], row["delta_T"], row["elapsed"],
                                 row["counts"]["race_seed_count"], row["counts"]["race_winner_initial_rank"]])
    save(OUT / "comparison.json", summaries)
    print(json.dumps(summaries, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    {"diagnostic": diagnostic, "analyze": analyze}[sys.argv[1]]()
