#!/usr/bin/env python3
"""v080の保存対照に対し、手数差で重み付けした学習を1条件だけ比較する。"""
import argparse
import csv
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback

import numpy as np

from v080_data import ROOT, STEPS, load, now, save, sha, status, subsets
from run_v080_scaling import SOURCES, cpp_benchmark, input_average, interval, metrics


BASE = ROOT / "results/nn_rank/v080/20261002T113935_studio"
NAME = "n49152_h16"
SOURCE_FILES = [*SOURCES, "adhoc/scripts/train_v081_gap.py", "adhoc/scripts/run_v081_loss.py"]


def prepare(run):
    baseline_config = json.loads((BASE / "config.json").read_text())
    assert json.loads((BASE / "exit.json").read_text())["exit_code"] == 0
    # 同じ監視コードの人工停止試験はv080で通過済み。変更なしを確認して再利用する。
    assert sha(ROOT / "adhoc/scripts/memory_guard.py") == baseline_config["source_sha256"]["adhoc/scripts/memory_guard.py"]
    assert json.loads((BASE / "memory_checks/passed.json").read_text())["passed"]
    dataset = json.loads((BASE / "dataset.json").read_text())
    data_hashes = dataset["array_sha256"]
    for name, digest in data_hashes.items():
        assert sha(BASE / "data" / f"{name}.npy") == digest
    baseline = {}
    for steps in STEPS:
        directory = BASE / "models" / NAME
        finished = json.loads((directory / f"finished_{steps}.json").read_text())
        assert sha(directory / f"step_{steps}.pt") == finished["checkpoint_sha256"]
        assert sha(directory / f"prediction_{steps}.npy") == finished["prediction_sha256"]
        baseline[str(steps)] = finished
    sources = {p: sha(ROOT / p) for p in SOURCE_FILES}
    path = run / "config.json"
    if path.exists():
        config = json.loads(path.read_text())
        assert config["source_sha256"] == sources
        assert config["data_sha256"] == data_hashes and config["baseline_checkpoints"] == baseline
        assert config["baseline_cpp_sha256"] == sha(BASE / "cpp_result.json")
        for p, digest in config["probe_sha256"].items():
            assert sha(run / p) == digest
        assert (run / "data").resolve() == (BASE / "data").resolve()
        assert sha(run / "dataset.json") == sha(BASE / "dataset.json")
        return
    (run / "data").symlink_to(BASE / "data", target_is_directory=True)
    shutil.copyfile(BASE / "dataset.json", run / "dataset.json")
    shutil.copytree(BASE / "saved_probes", run / "saved_probes")
    shutil.copyfile(BASE / "probe_manifest.json", run / "probe_manifest.json")
    for p in SOURCE_FILES:
        target = run / "frozen" / p
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / p, target)
    config = {"experiment": "v081", "created_at": now(), "baseline": str(BASE),
         "condition": NAME, "loss": "gap_weighted_softplus_divided_by_differing_pair_count",
         "seed": 78001, "batch_size": 256, "lr": .001, "checkpoints": STEPS,
         "train_inputs": 49152, "primary_inputs": 2048, "additional_inputs": 14336,
         "source_sha256": sources, "data_sha256": data_hashes, "baseline_checkpoints": baseline,
         "baseline_cpp_sha256": sha(BASE / "cpp_result.json"),
         "normalization_sha256": baseline_config["normalization_sha256"],
         "probe_sha256": {p: sha(run / p) for p in baseline_config["probe_sha256"]},
         "maximum_seconds": 5400, "memory_stop_bytes": 56000000000, "gpu_cap_bytes": 16000000000,
         "final_comparisons": 1, "alpha": .05, "bootstrap_replicates": 6000, "bootstrap_seed": 80002}
    save(path, config)


def worker(run, mode, deadline, steps=None):
    if time.time() >= deadline:
        raise TimeoutError("registered_time_limit")
    command = [sys.executable, str(ROOT / "adhoc/scripts/train_v081_gap.py"), "--run", str(run),
               "--mode", mode, "--deadline", str(deadline)]
    if steps is not None:
        command += ["--steps", str(steps)]
    log_path = run / f"worker_{mode}_{steps or 0}.log"
    with log_path.open("a") as log:
        # 親の監視対象群から離れず、GPUを使う子は1つだけにする。
        result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
    save(run / "gpu_state.json", {"active": False, "updated_unix": time.time(), "driver_bytes": 0})
    if result.returncode == 75:
        raise TimeoutError("registered_time_limit")
    if result.returncode:
        raise RuntimeError(f"GPU child failed: {log_path.name}, exit={result.returncode}")


def evaluate(run, deadline):
    data = load(run)
    sets = subsets(data)
    report = {"baseline": str(BASE), "loss_change_only": True, "checkpoints": {},
              "scope": "candidate_for_next_full_search_evaluation"}
    for steps in STEPS:
        result = {"models": {}, "comparisons": {}}
        groups = {"training": sets["train49152"], "primary": sets["primary"]}
        if steps == STEPS[-1]:
            groups["additional"] = sets["additional"]
        selected_by_model, shorter_by_model = {}, {}
        for label, directory in (("plain", BASE), ("gap_weighted", run)):
            prediction = np.load(directory / "models" / NAME / f"prediction_{steps}.npy", mmap_mode="r")
            result["models"][label] = {}
            for split, ids in groups.items():
                if time.time() >= deadline:
                    raise TimeoutError("registered_time_limit")
                status(run, "evaluating_saved_candidates", model=label, steps=steps, subset=split)
                stats, chosen, shortened = metrics(data, ids, prediction[ids])
                result["models"][label][split] = {"all": stats}
                selected_by_model[label, split] = chosen
                shorter_by_model[label, split] = shortened
                for title, selection in (("M_lt80", data["meta"][ids, 2] < 80),
                                         ("M_ge80", data["meta"][ids, 2] >= 80)):
                    result["models"][label][split][title] = metrics(data, ids[selection], prediction[ids[selection]])[0]
        for split, ids in groups.items():
            if split == "training":
                continue
            values = np.column_stack((selected_by_model["plain", split], selected_by_model["gap_weighted", split]))
            unique, high, means = input_average(values, data["meta"][ids])
            statistics = interval(means[:, 1:] - means[:, :1], high)
            # 最終比較は1つ。v080の共通集計関数が返す補正区間は今回使わない。
            result["comparisons"][split] = {"mean": statistics["mean"][0], "ci95": statistics["ci95"][0]}
            shorter = np.column_stack((shorter_by_model["plain", split], shorter_by_model["gap_weighted", split]))
            _, _, short_means = input_average(shorter, data["meta"][ids])
            result["comparisons"][split]["shorter_rate_difference"] = float((short_means[:, 1]-short_means[:, 0]).mean())
            with (run / f"{split}_per_input_{steps}.csv").open("w", newline="") as f:
                writer = csv.writer(f)
                writer.writerow(["index", "M_ge80", "plain_first_T", "gap_weighted_first_T", "difference"])
                writer.writerows([int(i), int(h), a, b, b-a] for i, h, (a, b) in zip(unique, high, means))
        report["checkpoints"][str(steps)] = result
        save(run / "comparison.partial.json", report)
    cpp = cpp_benchmark(run, deadline, condition_names=[NAME])
    baseline_cpp = json.loads((BASE / "cpp_result.json").read_text())
    assert cpp["compiler"] == baseline_cpp["compiler"]
    ratios = {mode: cpp["conditions"][NAME][mode]["mean_selection_ms"] /
                   baseline_cpp["conditions"][NAME][mode]["mean_selection_ms"] for mode in ("local", "nonlocal")}
    effect = report["checkpoints"][str(STEPS[-1])]["comparisons"]["primary"]
    gate = {"passed": effect["ci95"][1] < 0 and effect["shorter_rate_difference"] >= 0 and max(ratios.values()) <= 2,
            "effect": effect, "selection_time_ratio": ratios, "mechanism_checks_passed": True}
    report["gate"], report["finished_at"] = gate, now()
    save(run / "comparison.json", report)
    end = "次の探索全体の評価へ進める候補" if gate["passed"] else "事前登録した基準に未達"
    lines = ["# v081 手数差で重み付けした学習", "", "判定: " + end + "。", "",
             "対照は同じデータ、モデル構造、92,160更新のv080大量小型。負の差が改善を表す。", "",
             f"- 主検証の選択直後手数の平均差: {effect['mean']:.6f}手。",
             f"- 入力単位の95%区間: {effect['ci95'][0]:.6f}〜{effect['ci95'][1]:.6f}手。",
             f"- 短縮候補を選ぶ割合の差: {100*effect['shorter_rate_difference']:+.3f}ポイント。",
             f"- C++選択時間比: LOCAL {ratios['local']:.3f}、非LOCAL {ratios['nonlocal']:.3f}。", "",
             "全機構確認を通過した。追加診断と途中更新の結果はcomparison.jsonを参照する。",
             "探索全体の改善とM3での最終採否は未評価。結果に応じた再学習や追加のsolver実行は行っていない。"]
    (run / "REPORT.md").write_text("\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--calibrate", action="store_true")
    parser.add_argument("--seconds", type=int, default=5340)
    args = parser.parse_args()
    assert 0 < args.seconds <= 5340
    run = args.run.resolve()
    run.mkdir(parents=True, exist_ok=True)
    lock = run / "running.lock"
    if lock.exists():
        try:
            os.kill(int(lock.read_text()), 0)
        except ProcessLookupError:
            lock.unlink()
        else:
            raise RuntimeError("run already active")
    fd = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    os.write(fd, str(os.getpid()).encode())
    os.close(fd)
    deadline = time.time() + args.seconds
    try:
        prepare(run)
        if args.calibrate:
            worker(run, "calibrate", deadline)
            calibration = json.loads((run / "gpu_calibration.json").read_text())
            assert calibration["passed"]
            expected = STEPS[-1] * calibration["seconds_per_update"] + 90
            save(run / "estimate.json", {"expected_seconds": expected, "overhead_allowance_seconds": 90,
                                        "calibrated_at": now(), "kind": "synthetic_update_estimate"})
            status(run, "ready_to_launch", expected_seconds=expected)
            return
        assert json.loads((run / "gpu_calibration.json").read_text())["passed"]
        if (run / "comparison.json").exists():
            raise ValueError("comparison already finished")
        if not (run / "tiny.json").exists():
            status(run, "tiny_fit_check")
            worker(run, "tiny", deadline)
        tiny = json.loads((run / "tiny.json").read_text())
        assert tiny["passed"] and tiny["pair_accuracy"] >= .95
        for steps in STEPS:
            marker = run / "models" / NAME / f"finished_{steps}.json"
            if marker.exists():
                state = json.loads(marker.read_text())
                assert sha(marker.parent / f"step_{steps}.pt") == state["checkpoint_sha256"]
                assert sha(marker.parent / f"prediction_{steps}.npy") == state["prediction_sha256"]
            else:
                status(run, "starting_training", target_steps=steps)
                worker(run, "train", deadline, steps)
        evaluate(run, deadline)
        status(run, "finished", report=str(run / "REPORT.md"))
        save(run / "exit.json", {"reason": "registered_comparison_finished", "exit_code": 0, "finished_at": now()})
    except (TimeoutError, subprocess.TimeoutExpired):
        status(run, "paused", reason="registered_time_limit", resumable=True)
        save(run / "exit.json", {"reason": "registered_time_limit", "exit_code": 75, "finished_at": now()})
        raise SystemExit(75)
    except BaseException as error:
        status(run, "failed", error=str(error))
        save(run / "exit.json", {"reason": type(error).__name__, "error": str(error), "exit_code": 1, "finished_at": now()})
        traceback.print_exc()
        raise SystemExit(1)
    finally:
        lock.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
