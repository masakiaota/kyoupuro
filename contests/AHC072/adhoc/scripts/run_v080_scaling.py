#!/usr/bin/env python3
"""v080の事前登録済み4条件比較。結果から条件を変更せず、報告を保存して終了する。"""
import argparse
import csv
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time
import traceback

import numpy as np

from v080_data import (ROOT, SOURCE, PREVIOUS, PROBES, CONDITIONS, STEPS,
                       load, now, prepare, save, sha, status, subsets)


SOURCES = ["adhoc/scripts/run_v080_scaling.py", "adhoc/scripts/train_v080_scaling.py",
           "adhoc/scripts/v080_data.py", "adhoc/scripts/memory_guard.py",
           "adhoc/scripts/check_v080_memory.py",
           "adhoc/scripts/train_v078_targets.py", "adhoc/scripts/train_v077_rank.py",
           "adhoc/bin/check_v080_nn.cpp", "src/bin/v079_nn_immediate.cpp"]


def freeze(run):
    path = run / "config.json"
    if path.exists():
        config = json.loads(path.read_text())
        for name, digest in config["source_sha256"].items():
            assert sha(ROOT / name) == digest, f"source changed since registration: {name}"
        assert sha(SOURCE / "inputs.jsonl") == config["input_manifest_sha256"]
        assert sha(PREVIOUS / "dataset.json") == config["normalization_sha256"]
        return
    frozen = run / "frozen"
    frozen.mkdir()
    sources = {p: sha(ROOT / p) for p in SOURCES}
    for p in SOURCES:
        target = frozen / p
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / p, target)
    # C++診断の入力と参照特徴も固定する。v079の予測値は使わない。
    shutil.copytree(PROBES / "probes", run / "saved_probes",
                    ignore=lambda d, names: [n for n in names if not (n.endswith(".input") or n.endswith(".expected.json"))])
    shutil.copyfile(PROBES / "probe_manifest.json", run / "probe_manifest.json")
    save(path, {"created_at": now(), "source": str(SOURCE), "conditions": CONDITIONS, "checkpoints": STEPS,
         "batch_size": 256, "lr": .001, "seed": 78001, "bootstrap_seed": 80002,
         "bootstrap_replicates": 6000, "familywise_alpha": .05, "final_comparisons": 3,
         "source_sha256": sources, "input_manifest_sha256": sha(SOURCE / "inputs.jsonl"),
         "normalization_sha256": sha(PREVIOUS / "dataset.json"),
         "probe_sha256": {str(p.relative_to(run)): sha(p) for p in (run / "saved_probes").iterdir()},
         "python": sys.version, "platform": platform.platform(), "cpu_threads": 30,
         "loader_processes": 16, "gpu_allocator_limit_bytes": 16000000000,
         "guard_stop_bytes": 56000000000, "user_limit_bytes": 64000000000,
         "maximum_run_seconds": 5400, "checkpoint_resume_source": str(PREVIOUS / "models/immediate/latest.pt")})


def worker(run, mode, deadline, name=None, steps=None):
    if time.time() >= deadline:
        raise TimeoutError("registered_time_limit")
    args = [sys.executable, str(ROOT / "adhoc/scripts/train_v080_scaling.py"),
            "--run", str(run), "--mode", mode, "--deadline", str(deadline)]
    if name:
        args += ["--condition", name, "--steps", str(steps)]
    log = run / f"worker_{mode}_{name or 'checks'}_{steps or 0}.log"
    with log.open("a") as f:
        # 親の監視対象群を保つ。GPUを使う子を並列に起動しない。
        result = subprocess.run(args, cwd=ROOT, stdout=f, stderr=subprocess.STDOUT)
    save(run / "gpu_state.json", {"active": False, "updated_unix": time.time(), "driver_bytes": 0})
    if result.returncode == 75:
        raise TimeoutError("registered_time_limit")
    if result.returncode:
        raise RuntimeError(f"GPU child failed: {log.name}, exit={result.returncode}")


def export_header(run, name, weights):
    norm = json.loads((run / "dataset.json").read_text())
    def literal(v):
        if isinstance(v, list):
            return "{" + ",".join(map(literal, v)) + "}"
        assert np.isfinite(v)
        return float(np.float32(v)).hex() + "f"
    lines = ["// v080: 固定した最終更新の重み。値が小さい候補を選ぶ。", "namespace v080_rank {"]
    for key in ("mean", "scale"):
        lines.append(f"constexpr float {key}[32]={literal(norm[key])};")
    sizes = (32, *CONDITIONS[name][1], 1)
    for i, (a, b) in enumerate(zip(sizes, sizes[1:])):
        lines.append(f"constexpr float w{i}[{b}][{a}]={literal(weights[f'{2*i}.weight'].tolist())};")
        lines.append(f"constexpr float b{i}[{b}]={literal(weights[f'{2*i}.bias'].tolist())};")
    lines += ["float predict(const array<double,32>& raw) {", "array<float,32> x0;",
              "for(int j=0;j<32;j++)x0[j]=(float(raw[j])-mean[j])/scale[j];"]
    for i, (a, b) in enumerate(zip(sizes, sizes[1:])):
        lines += [f"array<float,{b}> x{i+1};", f"for(int k=0;k<{b};k++){{float v=b{i}[k];",
                  f"for(int j=0;j<{a};j++)v+=w{i}[k][j]*x{i}[j];",
                  f"x{i+1}[k]=" + ("max(0.0f,v);}" if i < len(sizes)-2 else "v;}")]
    lines += [f"return x{len(sizes)-1}[0];", "}", "}"]
    path = run / "models" / name / "model.hpp"
    path.write_text("\n".join(lines) + "\n")
    save(path.with_suffix(".json"), {"mean": norm["mean"], "scale": norm["scale"],
         "weights": {k: v.tolist() for k, v in weights.items()}, "hidden": CONDITIONS[name][1]})
    return path


def cpp_benchmark(run, deadline, condition_names=None):
    import torch
    from train_v080_scaling import model_new
    torch.set_num_threads(30)
    config = json.loads((run / "config.json").read_text())
    for p, digest in config["probe_sha256"].items():
        assert sha(run / p) == digest
    norm = json.loads((run / "dataset.json").read_text())
    mean, scale = np.array(norm["mean"], np.float32), np.array(norm["scale"], np.float32)
    manifest = json.loads((run / "probe_manifest.json").read_text())
    env = os.environ.copy()
    if sys.platform == "darwin":
        env["SDKROOT"] = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
        env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    compiler = os.environ.get("CXX", "g++-15")
    result = {"compiler": subprocess.check_output([compiler, "--version"], text=True).splitlines()[0], "conditions": {}}
    directory = run / "cpp"
    directory.mkdir(exist_ok=True)
    for name in (CONDITIONS if condition_names is None else condition_names):
        checkpoint = run / "models" / name / f"step_{STEPS[-1]}.pt"
        weights = torch.load(checkpoint, map_location="cpu", weights_only=False)["model"]
        header = export_header(run, name, weights)
        cpu = model_new(CONDITIONS[name][1], "cpu")
        cpu.load_state_dict(weights)
        references = {}
        for item in manifest:
            groups = json.loads((run / "saved_probes" / f"{item['stem']}.expected.json").read_text())
            for g in groups:
                x = (np.array(g["features"], np.float32) - mean) / scale
                with torch.no_grad():
                    scores = cpu(torch.from_numpy(x)).squeeze(-1).numpy()
                g["scores"] = scores.tolist()
                g["choice"] = g["ranks"][int(scores.argmin())]
            references[item["stem"]] = groups
        save(directory / f"{name}.reference.json", references)
        result["conditions"][name] = {}
        for mode in ("local", "nonlocal"):
            status(run, "cpp_saved_state_check", condition=name, mode=mode)
            if time.time() >= deadline:
                raise TimeoutError("registered_time_limit")
            binary = directory / f"{name}_{mode}"
            command = [compiler, "-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
                       "-ftrivial-auto-var-init=zero", "-fopenmp", f'-DAHC080_MODEL_HEADER="{header}"']
            command += ["-DLOCAL"] if mode == "local" else ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]
            command += [str(run / "frozen/adhoc/bin/check_v080_nn.cpp"), "-o", str(binary)]
            with (directory / f"{name}_{mode}.build.log").open("w") as log:
                subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT,
                               timeout=max(1, deadline-time.time()), check=True)
            timings = []
            feature_error = score_error = 0.
            for item in manifest:
                stem = item["stem"]
                out = directory / f"{name}_{mode}_{stem}.jsonl"
                if out.exists():
                    # 中断再開でも、同じ保存局面を時間測定のために再実行しない。
                    actual = [json.loads(line) for line in out.read_text().splitlines()]
                else:
                    with (run / "saved_probes" / f"{stem}.input").open() as inp, out.open("x") as dest:
                        with out.with_suffix(".err").open("w") as err:
                            subprocess.run([str(binary)], stdin=inp, stdout=dest, stderr=err,
                                           timeout=max(1, min(30, deadline-time.time())), check=True)
                    actual = [json.loads(line) for line in out.read_text().splitlines()]
                expected = references[stem]
                assert len(actual) == len(expected) == 4
                for got, want in zip(actual, expected):
                    assert got["phase"] == want["phase"] and got["choice"] == want["choice"], (name, mode, stem, "choice")
                    a, b = np.array(got["features"]), np.array(want["features"])
                    np.testing.assert_allclose(a, b, rtol=1e-12, atol=1e-10)
                    feature_error = max(feature_error, float(np.max(np.abs(a-b))))
                    a, b = np.array(got["scores"]), np.array(want["scores"])
                    np.testing.assert_allclose(a, b, rtol=2e-5, atol=2e-4)
                    score_error = max(score_error, float(np.max(np.abs(a-b))))
                    timings.append([got[k] for k in ("selection_ms", "feature_ms", "inference_ms")])
            timing = np.array(timings)
            result["conditions"][name][mode] = {"groups": len(timing), "passed": True,
                "mean_selection_ms": float(timing[:, 0].mean()), "max_selection_ms": float(timing[:, 0].max()),
                "mean_feature_ms": float(timing[:, 1].mean()), "mean_inference_ms": float(timing[:, 2].mean()),
                "max_feature_error": feature_error, "max_score_error": score_error, "binary_sha256": sha(binary)}
            save(run / "cpp_result.json", result)
    result["passed"] = True
    save(run / "cpp_result.json", result)
    return result


def metrics(data, ids, pred):
    from train_v078_targets import pair_stats
    y, mask = data["y"][ids], data["mask"][ids]
    choice = np.where(mask, pred, np.inf).argmin(1)
    selected = y[np.arange(len(y)), choice]
    best = np.where(mask, y, np.inf).min(1)
    shorter = data["measures"][ids, choice, 1]
    available = data["measures"][ids, :, 1].max(1) > 0
    answer = pair_stats(pred, y, mask)
    answer.update(selected_first_T=float(selected.mean()), selected_minus_original=float((selected-y[:, 0]).mean()),
        selected_gap_to_observed_min=float((selected-best).mean()), shorter_selection_rate=float(shorter.mean()),
        shorter_available_rate=float(available.mean()),
        shorter_capture_rate=float(shorter[available].mean()) if available.any() else None,
        selected_first_ms=float(data["measures"][ids, choice, 0].mean()))
    return answer, selected, shorter


def input_average(values, meta):
    unique, inv = np.unique(meta[:, 0], return_inverse=True)
    count = np.bincount(inv)
    result = np.zeros((len(unique), values.shape[1]), np.float64)
    np.add.at(result, inv, values)
    return unique, np.bincount(inv, weights=meta[:, 2]) / count >= 80, result / count[:, None]


def interval(values, high):
    rng = np.random.default_rng(80002)
    boot = np.zeros((6000, values.shape[1]))
    for s in range(0, 6000, 100):
        block = np.zeros((100, values.shape[1]))
        for stratum in (False, True):
            a = values[high == stratum]
            if len(a):
                block += a[rng.integers(len(a), size=(100, len(a)))].sum(1)
        boot[s:s+100] = block / len(values)
    return {"mean": values.mean(0).tolist(),
            "ci95": np.quantile(boot, [.025, .975], axis=0).T.tolist(),
            "ci98_333": np.quantile(boot, [.05/6, 1-.05/6], axis=0).T.tolist()}


def evaluate(run, deadline):
    data, sets = load(run), None
    sets = subsets(data)
    names = list(CONDITIONS)
    report = {"conditions": names, "checkpoints": {}, "gate": {}, "selection_scope": "next_full_search_test_only"}
    final_chosen = {}
    for steps in STEPS:
        stage = {"models": {}, "primary_comparisons": {}}
        choices, short = [], []
        for name in names:
            prediction = np.load(run / "models" / name / f"prediction_{steps}.npy", mmap_mode="r")
            selected_sets = {"training": sets["train"+str(CONDITIONS[name][0])], "primary": sets["primary"]}
            if steps == STEPS[-1]:
                selected_sets["additional"] = sets["additional"]
            stage["models"][name] = {}
            for label, ids in selected_sets.items():
                if time.time() >= deadline:
                    raise TimeoutError("registered_time_limit")
                status(run, "evaluating_saved_candidates", condition=name, steps=steps, subset=label)
                detail, chosen, shortened = metrics(data, ids, prediction[ids])
                stage["models"][name][label] = {"all": detail}
                for title, selected in (("M_lt80", data["meta"][ids, 2] < 80), ("M_ge80", data["meta"][ids, 2] >= 80)):
                    stage["models"][name][label][title] = metrics(data, ids[selected], prediction[ids[selected]])[0]
                if label == "primary":
                    choices.append(chosen)
                    short.append(shortened)
                if steps == STEPS[-1]:
                    final_chosen[name, label] = chosen
        selected = np.array(choices).T
        unique, high, aggregated = input_average(selected, data["meta"][sets["primary"]])
        _, _, short_inputs = input_average(np.array(short).T, data["meta"][sets["primary"]])
        comparisons = interval(aggregated[:, 1:] - aggregated[:, :1], high)
        for i, name in enumerate(names[1:]):
            stage["primary_comparisons"][name] = {k: v[i] for k, v in comparisons.items()}
            stage["primary_comparisons"][name]["shorter_rate_difference"] = float((short_inputs[:, i+1] - short_inputs[:, 0]).mean())
        # 量と容量の効果、および両者を増やしたときの相互作用を同じ入力で記述する。
        effects = np.column_stack((aggregated[:, 1]-aggregated[:, 0], aggregated[:, 3]-aggregated[:, 2],
                                   aggregated[:, 2]-aggregated[:, 0], aggregated[:, 3]-aggregated[:, 1],
                                   aggregated[:, 3]-aggregated[:, 2]-aggregated[:, 1]+aggregated[:, 0]))
        stage["factor_effects"] = dict(zip(("more_data_small", "more_data_large", "larger_model_small_data",
                                             "larger_model_large_data", "interaction"), effects.mean(0).tolist()))
        with (run / f"primary_per_input_{steps}.csv").open("w", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(["index", "M_ge80", *names])
            writer.writerows([int(i), int(h), *row] for i, h, row in zip(unique, high, aggregated))
        report["checkpoints"][str(steps)] = stage
        save(run / "comparison.partial.json", report)
    cpp = cpp_benchmark(run, deadline)
    final = report["checkpoints"][str(STEPS[-1])]
    candidates = []
    for name in names[1:]:
        effect = final["primary_comparisons"][name]
        ratios = {mode: cpp["conditions"][name][mode]["mean_selection_ms"] /
                       cpp["conditions"][names[0]][mode]["mean_selection_ms"] for mode in ("local", "nonlocal")}
        passed = effect["ci98_333"][1] < 0 and effect["shorter_rate_difference"] >= 0 and max(ratios.values()) <= 2
        report["gate"][name] = {"passed": passed, "effect": effect, "selection_time_ratio": ratios,
                                "mechanism_checks_passed": True}
        if passed:
            candidates.append(name)
    candidates.sort(key=lambda name: (final["primary_comparisons"][name]["mean"], cpp["conditions"][name]["local"]["mean_selection_ms"]))
    report["candidates"] = candidates
    # 追加集合の差は診断専用で、主判定の集合は差し替えない。
    extra_ids, extra_high, extra = input_average(np.column_stack([final_chosen[n, "additional"] for n in names]), data["meta"][sets["additional"]])
    report["additional_comparisons"] = interval(extra[:, 1:] - extra[:, :1], extra_high)
    report["finished_at"] = now()
    save(run / "comparison.json", report)
    lines = ["# v080 学習量と容量の4条件比較", "", "固定92,160更新での主検証結果。差は少量小型モデルに対する選択直後手数で、負が改善。", "",
             "| 条件 | 平均差 | 補正済み区間 | 短縮選択率の差 | 選択時間比 LOCAL / 非LOCAL | 次の全探索評価 |",
             "|---|---:|---|---:|---|---|"]
    for name, gate in report["gate"].items():
        e, r = gate["effect"], gate["selection_time_ratio"]
        lines.append(f"| {name} | {e['mean']:.6f} | {e['ci98_333'][0]:.6f} ～ {e['ci98_333'][1]:.6f} | {100*e['shorter_rate_difference']:+.3f}ポイント | {r['local']:.3f} / {r['nonlocal']:.3f} | {'候補' if gate['passed'] else '基準未達'} |")
    lines += ["", "優先候補: " + (", ".join(candidates) if candidates else "該当なし"), "",
              "これは保存候補に対する比較である。探索全体の改善、平均5手の目標、M3での最終採否は未評価。",
              "入力単位・Mの2層で6,000回復元抽出し、最終3比較をBonferroni補正した。詳細と途中更新の結果はcomparison.jsonを参照する。"]
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
        pid = int(lock.read_text())
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            lock.unlink()
        else:
            raise RuntimeError(f"run already active: {pid}")
    fd = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    os.write(fd, str(os.getpid()).encode())
    os.close(fd)
    deadline = time.time() + args.seconds
    try:
        freeze(run)
        if args.calibrate:
            rate = prepare(run, deadline, limit=256)
            save(run / "data_calibration.json", rate)
            worker(run, "calibrate", deadline)
            gpu = json.loads((run / "gpu_calibration.json").read_text())
            train_seconds = 172800 * gpu["architectures"]["16"]["seconds_per_update"] + 184320 * gpu["architectures"]["64_32"]["seconds_per_update"]
            # 小規模な読み込みは起動費用を含む。コンパイル・検査・集計には別に3分を見込む。
            expected = 65280 / rate["inputs_per_second"] + train_seconds + 180
            save(run / "estimate.json", {"expected_seconds": expected, "training_seconds": train_seconds,
                 "data_seconds": 65280/rate["inputs_per_second"], "overhead_allowance_seconds": 180,
                 "calibrated_at": now(), "note": "人工更新と256入力からの概算。実測の進捗により幅がある。"})
            status(run, "ready_to_launch", expected_seconds=expected)
            return
        assert (run / "gpu_calibration.json").exists()
        if (run / "comparison.json").exists():
            raise ValueError("comparison already finished")
        if not (run / "dataset.json").exists():
            prepare(run, deadline)
        else:
            for name, digest in json.loads((run / "dataset.json").read_text())["array_sha256"].items():
                assert sha(run / "data" / f"{name}.npy") == digest
        if not (run / "tiny.json").exists():
            status(run, "tiny_fit_checks")
            worker(run, "tiny", deadline)
        tiny = json.loads((run / "tiny.json").read_text())
        assert set(tiny) == {"16", "64_32"} and all(r["pair_accuracy"] >= .95 for r in tiny.values())
        for steps in STEPS:
            for name in CONDITIONS:
                finished = run / "models" / name / f"finished_{steps}.json"
                if finished.exists():
                    audit = json.loads(finished.read_text())
                    assert sha(finished.parent / f"step_{steps}.pt") == audit["checkpoint_sha256"]
                    assert sha(finished.parent / f"prediction_{steps}.npy") == audit["prediction_sha256"]
                else:
                    status(run, "starting_training", condition=name, target_steps=steps)
                    worker(run, "train", deadline, name, steps)
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
