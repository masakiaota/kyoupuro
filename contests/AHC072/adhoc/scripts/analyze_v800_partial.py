#!/usr/bin/env python3
"""完了済みのv800記録を読み、探索を動かさず途中分析用のCSVを作る。"""

import argparse
from collections import Counter
import csv
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import re
import statistics
import sys


def read_json(path):
    return json.loads(path.read_text())


def write_csv(path, rows):
    if not rows:
        return
    keys = list(dict.fromkeys(key for row in rows for key in row))
    with path.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=keys)
        writer.writeheader()
        writer.writerows(rows)


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def contact_history(path):
    """独立な操作の並べ替えで変わらない、各マスの接触順序を比較する。"""
    histories = {}
    directions = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}
    for line in path.read_text().splitlines():
        i, j, k, d, length = line.split()
        i, j, k, length = map(int, (i, j, k, length))
        di, dj = directions[d]
        command = (i, j, k, d, length)
        for cell in ((i, j), (i + di*length, j + dj*length)):
            histories.setdefault(cell, []).append(command)
    return histories


def summarize(out, cases, workers, rounds, events, counters, curves):
    # 0000は通常generatorと異なる入力なので、傾向の主集計から外す。
    ordinary = [r for r in cases if r["cohort"] == "fresh_5min"]
    ids = {r["case"] for r in ordinary}
    group_rows = []
    for label, low, high in [("T_under_150", 0, 150), ("T_150_to_299", 150, 300),
                             ("T_at_least_300", 300, math.inf)]:
        group = [r for r in ordinary if low <= r["initial_T"] < high]
        if not group:
            continue
        group_ids = {r["case"] for r in group}
        ws = [r for r in workers if r["case"] in group_ids and r["mode"] == "continuous"]
        cs = [r for r in counters if r["case"] in group_ids and r["mode"] == "continuous"]
        def count(key):
            return sum(r.get(key, 0) for r in cs)
        def percent(a, b):
            return 100*a/b if b else None
        # gross/netはともに除去後の修復に成功した候補の計数で、母集団をそろえる。
        group_rows.append(dict(
            group=label, cases=len(group), saved=sum(r["saved"] for r in group),
            saved_percent=percent(sum(r["saved"] for r in group), sum(r["initial_T"] for r in group)),
            continuous_wins=sum(r["winner"] == "continuous" for r in group),
            multistart_wins=sum(r["winner"] == "multistart" for r in group),
            ties=sum(r["winner"] == "tie" for r in group),
            attempts_per_second_median=statistics.median(r["attempts_per_sec"] for r in ws),
            attempts_per_initial_T_median=statistics.median(r["attempts"]/r["initial_T"] for r in ws),
            dependency_completion_percent=percent(count("lns_dependency_completed"), count("lns_dependency_attempts")),
            dependency_extract_failure_percent=percent(count("lns_dependency_extract_failed"), count("lns_dependency_attempts")),
            dependency_insert_failure_percent=percent(count("lns_dependency_insert_failed"), count("lns_dependency_attempts")),
            dependency_gross_removed=count("lns_dependency_gross_removed"),
            dependency_net_removed=count("lns_dependency_net_removed"),
            dependency_repair_consumed_percent=percent(count("lns_dependency_gross_removed")-count("lns_dependency_net_removed"), count("lns_dependency_gross_removed")),
            dependency_time_percent=percent(count("ms_lns_dependency"), count("ms_total"))))
    write_csv(out / "groups.csv", group_rows)

    def ranks(values):
        positions = {}
        for i, value in enumerate(sorted(values)):
            positions.setdefault(value, []).append(i)
        return [statistics.mean(positions[value]) for value in values]

    def correlation(xs, ys):
        mx, my = statistics.mean(xs), statistics.mean(ys)
        denominator = math.sqrt(sum((x-mx)**2 for x in xs)*sum((y-my)**2 for y in ys))
        return sum((x-mx)*(y-my) for x, y in zip(xs, ys))/denominator if denominator else None

    target = ranks([r["saved_percent"] for r in ordinary])
    correlations = {key: correlation(ranks([r[key] for r in ordinary]), target)
                    for key in ("initial_T", "M", "N", "K", "wall_fraction", "density", "dominant_fraction")} if ordinary else {}
    records = [r for r in events if r["case"] in ids and r["mode"] == "continuous" and r["worker_saved"]]
    mode_curves = []
    for mark in sorted({r["elapsed_sec"] for r in curves}):
        selected = [r for r in curves if r["case"] in ids and r["elapsed_sec"] == mark]
        mode_curves.append(dict(elapsed_sec=mark, **{key:sum(r[key] for r in selected)
                               for key in ("continuous_T", "multistart_T", "best_T", "saved")}))
    write_csv(out / "aggregate_learning_curve.csv", mode_curves)
    round_totals = []
    for index in sorted({r["round"] for r in rounds if r["mode"] == "multistart"}):
        selected = [r for r in rounds if r["case"] in ids and r["mode"] == "multistart" and r["round"] == index]
        round_totals.append(dict(round=index, cases=len(selected),
                                new_saved=sum(r["worker_saved"] for r in selected),
                                within_round_saved=sum(r["initial_T"]-r["final_T"] for r in selected),
                                initial_gap_to_worker_best=sum(max(0,r["initial_T"]-r["prior_worker_T"]) for r in selected),
                                search_seconds=sum(r["elapsed_sec"] for r in selected)))
    write_csv(out / "aggregate_multistart_rounds.csv", round_totals)
    summary = dict(cases=len(ordinary), totals={key:sum(r[key] for r in ordinary)
                   for key in ("initial_T", "continuous_T", "multistart_T", "best_T", "saved")},
                   wins=dict(Counter(r["winner"] for r in ordinary)),
                   spearman_with_saved_percent=correlations,
                   continuous_records=dict(count=len(records), single_move=sum(r["worker_saved"]==1 for r in records),
                       same_length_different_plan_before=sum(r["before_T"]==r["previous_best_T"] and r["before_same_as_record"]==0 for r in records),
                       same_length_different_contact_before=sum(r["before_T"]==r["previous_best_T"] and r["before_same_contact_history"]==0 for r in records),
                       worse_length_before=sum(r["before_T"]>r["previous_best_T"] for r in records)),
                   caveats=["集計は今回の5分探索のみ。再利用10件と0000を除く。",
                            "改善率は群の合計短縮手数/合計初期手数。相関はケース別改善率の順位相関。",
                            "再出発の各回の短縮には、以前の最良値へ追いつくまでの重複を含む。",
                            "最良更新の直前だけを保存したログでは、それ以前の受理経路や変更の必要性を確定できない。"])
    (out / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2)+"\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("--through", required=True, help="集計を固定する最終ケース番号")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    run, out = args.run.resolve(), args.out.resolve()
    # 実行管理が使うファイルには書き込まない。分析資料も実行フォルダの外へ置く。
    if out == run or run in out.parents:
        parser.error("--out は実行フォルダの外を指定する")
    out.mkdir(parents=True, exist_ok=True)
    manifest = read_json(run / "manifest.json")
    spec = importlib.util.spec_from_file_location("v800_frozen_replay", run / "snapshot/run_v047_long_search.py")
    replay = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = replay
    spec.loader.exec_module(replay)
    cases, rounds, workers, events, progress, counters, curves, reused = [], [], [], [], [], [], [], []
    marks = [0, 2, 10, 30, 60, 120, 180, 240, 300]
    checked_files = {}
    snapshot_status = read_json(run / "status.json")
    for case in manifest["cases"]:
        case_id = case["case"]
        base = run / "cases" / case_id
        status = read_json(base / "status.json")
        if case["reused"]:
            reused.append(dict(case=case_id, **case["features"], reference_T=case["reference_T"],
                               best_T=status["T"], elapsed_ms=status["elapsed_ms"]))
            continue
        if int(case_id) > int(args.through) or status["status"] != "completed":
            continue
        # 中断ケースは1回5分と同列に扱えないため、この集計では明示的に拒否する。
        assert len(status["attempts"]) == 1, (case_id, status["attempts"])
        attempt = run / status["attempts"][0]["path"]
        assert read_json(attempt / "status.json")["status"] == "completed"
        problem = replay.Problem.read(run / case["input"])
        assert sha256(run / case["input"]) == case["input_sha256"]
        initial = problem.replay((run / case["seeds"][0]["path"]).read_text())
        final = problem.replay((base / "best.txt").read_text())
        assert initial["T"] == case["reference_T"] and final["T"] == status["T"]
        assert sha256(base / "best.txt") == status["best_sha256"]
        checked_files[str((base / "status.json").relative_to(run))] = sha256(base / "status.json")
        histories, mode_results = {}, {}
        for mode in ("continuous", "multistart"):
            worker = attempt / "cases" / case_id / mode
            worker_status = read_json(worker / "status.json")
            assert worker_status["status"] == "completed"
            worker_best = case["reference_T"]
            history = [(0.0, worker_best)]
            counts, times = Counter(), Counter()
            n_rounds = updates = total_attempts = 0
            wall = cpu = last_improvement = 0.0
            for folder in sorted(worker.glob("round_*")):
                meta = read_json(folder / "round.json")
                assert meta["status"] == "completed" and meta["returncode"] == 0
                path = folder / "search/events.jsonl"
                checked_files[str(path.relative_to(run))] = sha256(path)
                records = [json.loads(line) for line in path.read_text().splitlines()]
                assert records[0]["type"] == "start" and records[-1]["type"] == "finish"
                assert records[-1]["E"] == 0
                assert all(a["elapsed_sec"] <= b["elapsed_sec"] for a, b in zip(records, records[1:]))
                old_worker_best = worker_best
                kind = "first" if meta["round"] == 0 else ("alternate_seed" if meta["round"] % 2 else "own_best")
                round_initial = None
                for event in records:
                    elapsed = meta["started_offset_sec"] + event["elapsed_sec"]
                    common = dict(case=case_id, mode=mode, round=meta["round"], origin=kind,
                                  elapsed_sec=elapsed, round_elapsed_sec=event["elapsed_sec"])
                    if event["type"] == "progress":
                        progress.append(dict(**common, **{k: event[k] for k in (
                            "T", "current_T", "iteration", "attempts", "accepted", "temperature", "cooldown_entries")}))
                    if event["type"] not in ("initial", "improvement"):
                        continue
                    if round_initial is None:
                        assert event["type"] == "initial"
                        round_initial = event["T"]
                    worker_saved = max(0, worker_best - event["T"])
                    if worker_saved:
                        worker_best = event["T"]
                        updates += 1
                        last_improvement = elapsed
                        history.append((elapsed, worker_best))
                    before_same = ""
                    before_same_contact = ""
                    if event["before_plan"]:
                        before_path = folder / "search" / event["before_plan"]
                        previous_path = folder / "search" / event["previous_best_plan"]
                        before_same = int(before_path.read_bytes() == previous_path.read_bytes())
                        # 継続側だけなら、別の初期解の取り直しを混ぜずに比較できる。
                        if mode == "continuous":
                            before_same_contact = int(contact_history(before_path) == contact_history(previous_path))
                    events.append(dict(**common, **{k: event[k] for k in (
                        "type", "reason", "iteration", "attempts", "T", "previous_best_T", "saved",
                        "before_T", "common_prefix_moves", "common_suffix_moves", "W", "mixed_moves", "long_jumps")},
                        worker_saved=worker_saved, before_same_as_record=before_same,
                        before_same_contact_history=before_same_contact,
                        plan=str((folder / "search" / event["plan"]).relative_to(run)),
                        before_plan=str((folder / "search" / event["before_plan"]).relative_to(run)) if event["before_plan"] else ""))
                finish = records[-1]
                assert finish["status"] in ("time_limit", "completed")
                per_counts, per_times = {}, {}
                for category, key, value in re.findall(r"\[summary\.(count|time_ms)\] (\w+)=([\d.e+-]+)", (folder / "stderr.log").read_text()):
                    (per_counts if category == "count" else per_times)[key] = int(value) if category == "count" else float(value)
                counts.update(per_counts)
                times.update(per_times)
                rounds.append(dict(case=case_id, mode=mode, round=meta["round"], origin=kind,
                                   initial_source=meta["initial_source"], initial_T=round_initial, final_T=finish["T"],
                                   prior_worker_T=old_worker_best, worker_T=worker_best,
                                   worker_saved=old_worker_best-worker_best,
                                   elapsed_sec=finish["elapsed_sec"], cpu_sec=finish["cpu_sec"],
                                   attempts=finish["attempts"], improvements=finish["search_improvements"]))
                n_rounds += 1
                total_attempts += finish["attempts"]
                wall += finish["elapsed_sec"]
                cpu += finish["cpu_sec"]
            assert worker_best == worker_status["T"] and updates == worker_status["updates"]
            assert n_rounds == worker_status["rounds"]
            assert problem.replay((worker / "best.txt").read_text())["T"] == worker_best
            workers.append(dict(case=case_id, mode=mode, initial_T=case["reference_T"], best_T=worker_best,
                                saved=case["reference_T"]-worker_best, updates=updates, rounds=n_rounds,
                                last_improvement_sec=last_improvement, wall_sec=wall, cpu_sec=cpu,
                                attempts=total_attempts, accepted=counts["lns_accepted"],
                                attempts_per_sec=total_attempts/wall))
            counters.append(dict(case=case_id, mode=mode, **counts, **{"ms_"+k:v for k,v in times.items()}))
            histories[mode] = history
            mode_results[mode] = worker_best
        assert final["T"] == min(mode_results.values())
        cases.append(dict(case=case_id, cohort="handcrafted" if case_id == "0000" else "fresh_5min",
                          **case["features"], initial_T=initial["T"],
                          continuous_T=mode_results["continuous"], multistart_T=mode_results["multistart"],
                          best_T=final["T"], saved=initial["T"]-final["T"],
                          saved_percent=100*(initial["T"]-final["T"])/initial["T"],
                          winner="tie" if len(set(mode_results.values()))==1 else min(mode_results, key=mode_results.get),
                          **{"initial_"+k:initial[k] for k in ("W","mixed_moves","long_jumps")},
                          **{"best_"+k:final[k] for k in ("W","mixed_moves","long_jumps")}))
        for mark in marks:
            values = {mode:min(T for t,T in history if t<=mark) for mode,history in histories.items()}
            curves.append(dict(case=case_id, elapsed_sec=mark, continuous_T=values["continuous"],
                               multistart_T=values["multistart"], best_T=min(values.values()),
                               saved=initial["T"]-min(values.values())))
    for name, rows in [("cases",cases),("workers",workers),("rounds",rounds),("events",events),
                       ("progress",progress),("counters",counters),("learning_curve",curves),("reused",reused)]:
        write_csv(out / (name+".csv"), rows)
    summarize(out, cases, workers, rounds, events, counters, curves)
    audit = dict(run=str(run), through=args.through, created_at=datetime.now(timezone.utc).isoformat(),
                 run_status_at_read=snapshot_status, completed_cases=[r["case"] for r in cases],
                 reused_cases=[r["case"] for r in reused], manifest_sha256=sha256(run/"manifest.json"),
                 solver_sha256=sha256(run/"snapshot/v800.cpp"), files_sha256=checked_files,
                 validation="完了状態・イベントの順序・各方式の最良更新と終了値を照合。初期解、方式別最良解、採用解を独立再生。探索・公式採点・実行管理の更新は行わない。")
    (out / "audit.json").write_text(json.dumps(audit, ensure_ascii=False, indent=2)+"\n")
    print(json.dumps(dict(out=str(out), completed=len(cases), reused=len(reused),
                          cases=[r["case"] for r in cases], rows=len(events)), ensure_ascii=False))


if __name__ == "__main__":
    main()
