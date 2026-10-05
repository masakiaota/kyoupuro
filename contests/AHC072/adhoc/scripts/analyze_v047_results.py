#!/usr/bin/env python3
"""保存済みv047の記録を監査し、比較用CSVを作る。探索は実行しない。"""

import argparse
from collections import Counter
import csv
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_csv(path, rows):
    with path.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    args = parser.parse_args()
    run = args.run.resolve()
    root = Path(__file__).resolve().parents[2]
    manifest = json.loads((run / "manifest.json").read_text())
    status = json.loads((run / "status.json").read_text())
    assert status["status"] == "completed", status
    assert status["max_active_search_processes"] <= 2
    for name, record in manifest["files"].items():
        assert sha256(run / name) == record["sha256"], name
    assert sha256(run / "snapshot/v047_long_search") == manifest["build"]["binary_sha256"]

    # Use the frozen replay implementation without invoking its command entry point.
    spec = importlib.util.spec_from_file_location("v047_frozen_replay", run / "snapshot/run_v047_long_search.py")
    replay = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = replay
    spec.loader.exec_module(replay)
    out = run / "analysis"
    out.mkdir(exist_ok=True)
    duration = manifest["config"]["minutes_per_case"] * 60
    marks = sorted({0, 2, 10, 30, 60, 120, 300, 600, 900, 1800, duration})
    modes = ("continuous", "multistart")
    cases, curves, officials, rounds, worker_rows, records = [], [], [], [], [], []
    event_counts, checked, reasons = Counter(), Counter(), Counter()
    random_seeds = set()
    all_intervals = []
    expected_summary = {(r["case"], r["mode"]): r for r in csv.DictReader((run / "summary.csv").open())}
    expected_curves = {(r["case"], r["mode"], float(r["elapsed_sec"])): int(r["best_T"])
                       for r in csv.DictReader((run / "learning_curve.csv").open())}

    for case in manifest["cases"]:
        case_id = case["case"]
        inp = run / case["input"]
        assert sha256(inp) == case["input_sha256"]
        problem = replay.Problem.read(inp)
        cache = {}

        def check(path, expected=None):
            text = path.read_text()
            if text not in cache:
                cache[text] = problem.replay(text)
                checked["unique_plans"] += 1
            result = cache[text]
            assert result["E"] == 0, path
            if expected is not None:
                assert result["T"] == expected, (path, result, expected)
            checked["plan_references"] += 1
            return result

        for seed in case["seeds"]:
            path = run / seed["path"]
            assert sha256(path) == seed["plan_sha256"], path
            assert check(path) == seed["metrics"], path
            checked["seeds"] += 1
        histories, mode_best = {}, {}
        for mode in modes:
            worker = run / "cases" / case_id / mode
            worker_status = json.loads((worker / "status.json").read_text())
            assert worker_status["status"] == "completed"
            history = [(0.0, case["reference_T"])]
            worker_best = case["reference_T"]
            worker_updates = 0
            folders = sorted(worker.glob("round_*"))
            for folder in folders:
                meta = json.loads((folder / "round.json").read_text())
                assert meta["status"] == "completed" and meta["returncode"] == 0, folder
                assert meta["seed"] not in random_seeds
                random_seeds.add(meta["seed"])
                assert sha256(folder / "input_plan.txt") == meta["initial_sha256"]
                events = [json.loads(line) for line in (folder / "search/events.jsonl").read_text().splitlines()]
                assert events[0]["type"] == "start" and events[-1]["type"] == "finish"
                assert events[-1]["status"] in ("time_limit", "completed")
                assert events[-1]["E"] == 0
                assert all(a["elapsed_sec"] <= b["elapsed_sec"] for a, b in zip(events, events[1:]))
                current_best, previous_plan = None, None
                improvements = 0
                for event in events:
                    event_counts[event["type"]] += 1
                    if event["type"] not in ("initial", "improvement"):
                        continue
                    path = folder / "search" / event["plan"]
                    metrics = check(path, event["T"])
                    for key in ("E", "W", "mixed_moves", "long_jumps"):
                        assert metrics[key] == event[key], (path, key)
                    if event["type"] == "initial":
                        assert current_best is None
                        assert path.read_bytes() == (folder / "input_plan.txt").read_bytes()
                    else:
                        assert event["T"] < current_best
                        assert event["previous_best_T"] == current_best
                        assert event["saved"] == current_best - event["T"]
                        assert event["previous_best_plan"] == previous_plan
                        if event["before_plan"]:
                            check(folder / "search" / event["before_plan"], event["before_T"])
                            checked["before_plans"] += 1
                        improvements += 1
                        reasons[(mode, event["reason"])] += 1
                    current_best, previous_plan = event["T"], event["plan"]
                    elapsed = meta["started_offset_sec"] + event["elapsed_sec"]
                    history.append((elapsed, current_best))
                    if current_best < worker_best:
                        records.append(dict(case=case_id, mode=mode, elapsed_sec=elapsed,
                                            saved=worker_best-current_best, T=current_best,
                                            reason=event["reason"], round=meta["round"],
                                            plan=str(path.relative_to(run)),
                                            before_plan=str((folder / "search" / event["before_plan"]).relative_to(run)) if event["before_plan"] else ""))
                        worker_best = current_best
                        worker_updates += 1
                assert current_best == events[-1]["T"]
                check(folder / "search/best.txt", current_best)
                check(folder / "stdout.txt", current_best)
                finish = events[-1]
                rounds.append(dict(case=case_id, mode=mode, round=meta["round"],
                                   initial_T=check(folder / "input_plan.txt")["T"], best_T=current_best,
                                   initial_source=meta["initial_source"], seed=meta["seed"],
                                   elapsed_sec=finish["elapsed_sec"], cpu_sec=finish["cpu_sec"],
                                   attempts=finish["attempts"], improvements=improvements,
                                   peak_rss_bytes=finish["peak_rss_bytes"]))
                all_intervals += [(meta["started_at"], 1), (meta["finished_at"], -1)]
            assert len(folders) == worker_status["rounds"]
            assert len(folders) == 1 if mode == "continuous" else True
            assert worker_updates == worker_status["updates"]
            check(worker / "best.txt", worker_best)
            assert worker_best == worker_status["T"] == int(expected_summary[(case_id, mode)]["best_T"])
            histories[mode], mode_best[mode] = history, worker_best
            row_records = [r for r in records if r["case"] == case_id and r["mode"] == mode]
            worker_rows.append(dict(case=case_id, mode=mode, best_T=worker_best,
                                    worker_improvements=worker_updates,
                                    last_improvement_sec=row_records[-1]["elapsed_sec"] if row_records else 0))
        best = min(mode_best.values())
        final_metrics = check(run / "cases" / case_id / "best.txt", best)
        baseline_metrics = check(run / case["seeds"][0]["path"])
        winner = "tie" if mode_best["continuous"] == mode_best["multistart"] else min(mode_best, key=mode_best.get)
        cases.append(dict(case=case_id, **case["features"], initial_T=case["reference_T"],
                          continuous_T=mode_best["continuous"], multistart_T=mode_best["multistart"],
                          best_T=best, saved=case["reference_T"]-best,
                          saved_percent=100*(case["reference_T"]-best)/case["reference_T"], winner=winner,
                          initial_W=baseline_metrics["W"], best_W=final_metrics["W"],
                          initial_mixed_moves=baseline_metrics["mixed_moves"], best_mixed_moves=final_metrics["mixed_moves"],
                          initial_long_jumps=baseline_metrics["long_jumps"], best_long_jumps=final_metrics["long_jumps"]))
        for mark in marks:
            values = {mode: min(T for t, T in history if t <= mark) for mode, history in histories.items()}
            for mode in modes:
                if mark:
                    assert values[mode] == expected_curves[(case_id, mode, mark)]
            curves.append(dict(case=case_id, elapsed_sec=mark, continuous_T=values["continuous"],
                               multistart_T=values["multistart"], best_T=min(values.values()),
                               saved=case["reference_T"]-min(values.values())))

        # The official tool evaluates fixed saved plans only; no solver is invoked.
        for mode, path in [("initial", run / case["seeds"][0]["path"])] + [
                (mode, run / "cases" / case_id / mode / "best.txt") for mode in modes]:
            result = subprocess.run([str(root / "tools/target/release/vis"), "--no-vis", str(inp), str(path)],
                                    text=True, capture_output=True, check=True)
            expected = case["reference_T"] if mode == "initial" else mode_best[mode]
            assert result.stdout.strip() == f"Score = {expected}", (path, result.stdout, result.stderr)
            officials.append(dict(case=case_id, mode=mode, T=expected, official_score=expected,
                                  plan=str(path.relative_to(run))))
        print(f"{case_id}: {case['reference_T']} -> {best}, 保存記録の再生と公式採点に成功", flush=True)

    active = peak = 0
    for _, delta in sorted(all_intervals, key=lambda x: (x[0], x[1])):
        active += delta
        peak = max(peak, active)
    assert peak <= 2 and active == 0
    aggregate = [dict(elapsed_sec=mark, **{key: sum(r[key] for r in curves if r["elapsed_sec"] == mark)
                                         for key in ("continuous_T", "multistart_T", "best_T", "saved")}) for mark in marks]
    initial_sum = sum(r["initial_T"] for r in cases)
    final_sum = sum(r["best_T"] for r in cases)
    audit = dict(checked_at=datetime.now(timezone.utc).isoformat(), run_id=run.name, status="passed",
                 minutes_per_case=manifest["config"]["minutes_per_case"], cases=len(cases),
                 improved_cases=sum(r["saved"] > 0 for r in cases), initial_sum=initial_sum,
                 best_sum=final_sum, saved=initial_sum-final_sum, saved_percent=100*(initial_sum-final_sum)/initial_sum,
                 mode_totals={m: sum(r[m+"_T"] for r in cases) for m in modes},
                 winners=dict(Counter(r["winner"] for r in cases)),
                 event_counts=dict(event_counts), plan_checks=dict(checked), official_score_checks=len(officials),
                 unique_random_seeds=len(random_seeds), max_overlapping_rounds=peak,
                 cpu_hours=sum(r["cpu_sec"] for r in rounds)/3600,
                 peak_process_rss_bytes=max(r["peak_rss_bytes"] for r in rounds),
                 elapsed_sec=(datetime.fromisoformat(status["updated_at"])-datetime.fromisoformat(manifest["created_at"])).total_seconds(),
                 solver_executions_during_analysis=0, method="Frozen Python replay for all plans; official scorer for initial and both final plans")
    for name, rows in [("case_comparison", cases), ("paired_learning_curve", curves),
                       ("aggregate_learning_curve", aggregate), ("rounds", rounds),
                       ("worker_improvements", records), ("workers", worker_rows), ("official_scores", officials)]:
        write_csv(out / (name + ".csv"), rows)
    (out / "audit.json").write_text(json.dumps(audit, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(audit, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
