#!/usr/bin/env python3
"""保存教師の関係特徴追加、固定学習、条件付き探索評価を順に実行する。"""
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

from v080_data import ROOT, STEPS, now, save, sha, status, subsets
from v082_data import BASE, NAME, load, prepare
from run_v080_scaling import SOURCES, input_average, interval, metrics
from run_v082_search import prepare_inputs, execute as search_execute

SOURCE_FILES = list(dict.fromkeys([*SOURCES, "src/bin/v076_relative_tuned.cpp",
    "src/bin/v082_nn_relation.cpp", "adhoc/bin/extract_v082_features.cpp", "adhoc/bin/check_v082_nn.cpp",
    "adhoc/scripts/v082_data.py", "adhoc/scripts/train_v082_relation.py",
    "adhoc/scripts/check_v082_relation.py", "adhoc/scripts/run_v082_relation.py",
    "adhoc/scripts/run_v082_search.py", "adhoc/scripts/run_v079_experiment.py",
    "adhoc/scripts/run_v077_overnight.py", "scripts/eval.py", "scripts/build_solver.sh",
    "notes/experiments/v082.md"]))


def freeze(run):
    baseline_config = json.loads((BASE / "config.json").read_text())
    assert json.loads((BASE / "exit.json").read_text())["exit_code"] == 0
    assert sha(ROOT / "adhoc/scripts/memory_guard.py") == baseline_config["source_sha256"]["adhoc/scripts/memory_guard.py"]
    assert json.loads((BASE / "memory_checks/passed.json").read_text())["passed"]
    sources = {p: sha(ROOT / p) for p in SOURCE_FILES}
    path = run / "config.json"
    if path.exists():
        config = json.loads(path.read_text())
        for p, digest in config["source_sha256"].items():
            assert sha(run / "frozen" / p) == digest
            if p != "notes/experiments/v082.md":
                assert sources[p] == digest, ("source_changed", p)
        for p, digest in config["probe_sha256"].items():
            assert sha(run / p) == digest
        return config
    dataset = json.loads((BASE / "dataset.json").read_text())
    for name, digest in dataset["array_sha256"].items():
        assert sha(BASE / "data" / f"{name}.npy") == digest
    baselines = {}
    for steps in STEPS:
        directory = BASE / "models" / NAME
        finished = json.loads((directory / f"finished_{steps}.json").read_text())
        assert sha(directory / f"step_{steps}.pt") == finished["checkpoint_sha256"]
        assert sha(directory / f"prediction_{steps}.npy") == finished["prediction_sha256"]
        baselines[str(steps)] = finished
    shutil.copytree(BASE / "saved_probes", run / "saved_probes")
    shutil.copyfile(BASE / "probe_manifest.json", run / "probe_manifest.json")
    for p in SOURCE_FILES:
        target = run / "frozen" / p
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(ROOT / p, target)
    config = {"experiment": "v082", "created_at": now(), "baseline": str(BASE), "condition": NAME,
        "features": 46, "hidden": [16], "parameters": 769, "seed": 78001, "batch_size": 256, "lr": .001,
        "checkpoints": STEPS, "train_inputs": 49152, "primary_inputs": 2048, "additional_inputs": 14336,
        "source_sha256": sources, "baseline_data_sha256": dataset["array_sha256"],
        "baseline_checkpoints": baselines, "baseline_cpp_sha256": sha(BASE / "cpp_result.json"),
        "probe_sha256": {p: sha(run / p) for p in baseline_config["probe_sha256"]},
        "maximum_seconds": 5400, "memory_stop_bytes": 56000000000, "gpu_cap_bytes": 16000000000,
        "alpha": .05, "bootstrap_replicates": 6000, "bootstrap_seed": 80002,
        "platform": subprocess.check_output(["uname", "-a"], text=True).strip(),
        "chip": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()}
    save(path, config)
    return config


def compile_cpp(source, target, run, deadline, local=True, defines=()):
    env = os.environ.copy()
    if sys.platform == "darwin":
        env["SDKROOT"] = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
        env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    args = [os.environ.get("CXX", "g++-15"), "-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native",
            "-pthread", "-ftrivial-auto-var-init=zero", "-fopenmp"]
    args += ["-DLOCAL"] if local else ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]
    args += list(defines) + [str(source), "-o", str(target)]
    with target.with_suffix(".build.log").open("w") as log:
        subprocess.run(args, env=env, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                       timeout=max(1, deadline-time.time()), check=True)


def worker(run, mode, deadline, steps=None):
    args = [sys.executable, ROOT / "adhoc/scripts/train_v082_relation.py", "--run", run,
            "--mode", mode, "--deadline", str(deadline)]
    if steps is not None:
        args += ["--steps", str(steps)]
    log_path = run / f"worker_{mode}_{steps or 0}.log"
    with log_path.open("a") as log:
        result = subprocess.run(list(map(str, args)), cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                timeout=max(1, deadline-time.time()+10))
    save(run / "gpu_state.json", {"active": False, "updated_unix": time.time(), "driver_bytes": 0})
    if result.returncode == 75:
        raise TimeoutError("registered_time_limit")
    if result.returncode:
        raise RuntimeError(f"GPU child failed: {log_path.name}, exit={result.returncode}")


def export_model(run):
    import torch
    norm = json.loads((run / "dataset.json").read_text())
    path = run / "models" / NAME
    weights = torch.load(path / f"step_{STEPS[-1]}.pt", map_location="cpu", weights_only=False)["model"]
    def literal(v):
        if isinstance(v, list):
            return "{" + ",".join(map(literal, v)) + "}"
        assert np.isfinite(v)
        return float(np.float32(v)).hex() + "f"
    lines = ["// v082: 関係14特徴を加え、固定92,160更新で得た重み。", "namespace v082_rank {"]
    for key in ("mean", "scale"):
        assert len(norm[key]) == 46
        lines.append(f"constexpr float {key}[46]={literal(norm[key])};")
    sizes = (46, 16, 1)
    for i, (a, b) in enumerate(zip(sizes, sizes[1:])):
        lines += [f"constexpr float w{i}[{b}][{a}]={literal(weights[f'{2*i}.weight'].tolist())};",
                  f"constexpr float b{i}[{b}]={literal(weights[f'{2*i}.bias'].tolist())};"]
    lines += ["float predict(const array<double,46>& raw) {", "array<float,46> x0;",
              "for(int j=0;j<46;j++)x0[j]=(float(raw[j])-mean[j])/scale[j];"]
    for i, (a, b) in enumerate(zip(sizes, sizes[1:])):
        lines += [f"array<float,{b}> x{i+1};", f"for(int k=0;k<{b};k++){{float v=b{i}[k];",
                  f"for(int j=0;j<{a};j++)v+=w{i}[k][j]*x{i}[j];",
                  f"x{i+1}[k]=" + ("max(0.0f,v);}" if i == 0 else "v;}")]
    lines += ["return x2[0];", "}", "}"]
    header = path / "model.hpp"
    header.write_text("\n".join(lines) + "\n")
    save(path / "model.json", {"mean": norm["mean"], "scale": norm["scale"],
                              "weights": {k: v.tolist() for k, v in weights.items()}})
    template = run / "frozen/src/bin/v082_nn_relation.cpp"
    text = template.read_text()
    start, end = "// V082_MODEL_BEGIN", "// V082_MODEL_END"
    assert text.count(start) == text.count(end) == 1
    before, rest = text.split(start)
    _, after = rest.split(end)
    integrated = run / "integrated/src/bin/v082_nn_relation.cpp"
    integrated.parent.mkdir(parents=True, exist_ok=True)
    integrated.write_text(before + start + "\n" + header.read_text().replace("namespace v082_rank", "namespace neural_rank") + end + after)
    checker = run / "integrated/adhoc/bin/check_v082_nn.cpp"
    checker.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(run / "frozen/adhoc/bin/check_v082_nn.cpp", checker)
    save(run / "integration.json", {"template_sha256": sha(template), "integrated_sha256": sha(integrated),
                                    "model_header_sha256": sha(header), "checkpoint_sha256": sha(path / f"step_{STEPS[-1]}.pt")})
    return header, weights


def cpp_check(run, deadline):
    import torch
    from train_v082_relation import model_new
    torch.set_num_threads(30)
    header, weights = export_model(run)
    net = model_new("cpu")
    net.load_state_dict(weights)
    norm = json.loads((run / "dataset.json").read_text())
    mean, scale = (np.array(norm[k], np.float32) for k in ("mean", "scale"))
    extra = np.load(run / "data/extra_raw.npy", mmap_mode="r")
    manifest = json.loads((run / "probe_manifest.json").read_text())
    references = {}
    for item in manifest:
        groups = json.loads((run / "saved_probes" / f"{item['stem']}.expected.json").read_text())
        for g in groups:
            raw = np.concatenate((np.array(g["features"], np.float64), extra[item["index"]*4+g["phase"], :len(g["ranks"])]), axis=1)
            with torch.no_grad():
                scores = net(torch.from_numpy((raw.astype(np.float32)-mean)/scale)).squeeze(-1).numpy()
            g.update(features=raw.tolist(), scores=scores.tolist(), choice=g["ranks"][int(scores.argmin())])
        references[item["stem"]] = groups
    folder = run / "cpp"
    folder.mkdir(exist_ok=True)
    save(folder / "reference.json", references)
    result = {"compiler": subprocess.check_output([os.environ.get("CXX", "g++-15"), "--version"], text=True).splitlines()[0], "modes": {}}
    for mode in ("local", "nonlocal"):
        status(run, "cpp_saved_state_check", mode=mode)
        binary = folder / f"check_{mode}"
        compile_cpp(run / "integrated/adhoc/bin/check_v082_nn.cpp", binary, run, deadline, mode == "local",
                    [f'-DAHC082_MODEL_HEADER="{header}"', "-DAHC082_CHECK_EMBEDDED"])
        timings, feature_error, score_error = [], 0., 0.
        for item in manifest:
            stem = item["stem"]
            out = folder / f"{mode}_{stem}.jsonl"
            if not out.exists():
                with (run / "saved_probes" / f"{stem}.input").open() as inp, out.open("x") as dest, out.with_suffix(".err").open("w") as err:
                    subprocess.run([str(binary)], stdin=inp, stdout=dest, stderr=err,
                                   timeout=max(1, min(30, deadline-time.time())), check=True)
            actual = [json.loads(line) for line in out.read_text().splitlines()]
            assert len(actual) == len(references[stem]) == 4
            for got, want in zip(actual, references[stem]):
                assert got["phase"] == want["phase"] and got["choice"] == want["choice"]
                assert got["ranks"] == want["ranks"]
                a, b = np.array(got["features"]), np.array(want["features"])
                np.testing.assert_allclose(a, b, rtol=1e-12, atol=1e-10)
                feature_error = max(feature_error, float(np.abs(a-b).max()))
                a, b = np.array(got["scores"]), np.array(want["scores"])
                np.testing.assert_allclose(a, b, rtol=2e-5, atol=2e-4)
                score_error = max(score_error, float(np.abs(a-b).max()))
                timings.append([got[k] for k in ("selection_ms", "feature_ms", "inference_ms")])
        a = np.array(timings)
        result["modes"][mode] = {"passed": True, "groups": len(a), "mean_selection_ms": float(a[:, 0].mean()),
                                 "max_selection_ms": float(a[:, 0].max()), "mean_feature_ms": float(a[:, 1].mean()),
                                 "mean_inference_ms": float(a[:, 2].mean()), "max_feature_error": feature_error,
                                 "max_score_error": score_error, "binary_sha256": sha(binary)}
        save(run / "cpp_result.json", result)
    result["passed"] = True
    save(run / "cpp_result.json", result)
    return result


def evaluate(run, deadline):
    data, report = load(run), {"baseline": str(BASE), "checkpoints": {}}
    sets = subsets(data)
    for steps in STEPS:
        groups = {"training": sets["train49152"], "primary": sets["primary"]}
        if steps == STEPS[-1]:
            groups["additional"] = sets["additional"]
        result, chosen, shorter = {"models": {}, "comparisons": {}}, {}, {}
        for label, directory in (("base32", BASE), ("relation46", run)):
            prediction = np.load(directory / "models" / NAME / f"prediction_{steps}.npy", mmap_mode="r")
            result["models"][label] = {}
            for split, ids in groups.items():
                if time.time() >= deadline:
                    raise TimeoutError("registered_time_limit")
                status(run, "evaluating_saved_candidates", model=label, steps=steps, subset=split)
                stats, chosen[label, split], shorter[label, split] = metrics(data, ids, prediction[ids])
                result["models"][label][split] = {"all": stats}
                for title, selection in (("M_lt80", data["meta"][ids, 2] < 80), ("M_ge80", data["meta"][ids, 2] >= 80)):
                    result["models"][label][split][title] = metrics(data, ids[selection], prediction[ids[selection]])[0]
        for split, ids in groups.items():
            if split == "training":
                continue
            values = np.column_stack((chosen["base32", split], chosen["relation46", split]))
            unique, high, means = input_average(values, data["meta"][ids])
            stats = interval(means[:, 1:]-means[:, :1], high)
            _, _, rates = input_average(np.column_stack((shorter["base32", split], shorter["relation46", split])), data["meta"][ids])
            result["comparisons"][split] = {"mean": stats["mean"][0], "ci95": stats["ci95"][0],
                                           "shorter_rate_difference": float((rates[:, 1]-rates[:, 0]).mean())}
            with (run / f"{split}_per_input_{steps}.csv").open("w", newline="") as f:
                writer = csv.writer(f)
                writer.writerow(["index", "M_ge80", "base32_first_T", "relation46_first_T", "difference"])
                writer.writerows([int(i), int(h), a, b, b-a] for i, h, (a, b) in zip(unique, high, means))
        report["checkpoints"][str(steps)] = result
        save(run / "comparison.partial.json", report)
    cpp = cpp_check(run, deadline)
    baseline_cpp = json.loads((BASE / "cpp_result.json").read_text())
    assert cpp["compiler"] == baseline_cpp["compiler"]
    ratios = {mode: cpp["modes"][mode]["mean_selection_ms"] / baseline_cpp["conditions"][NAME][mode]["mean_selection_ms"]
              for mode in ("local", "nonlocal")}
    effect = report["checkpoints"][str(STEPS[-1])]["comparisons"]["primary"]
    report["gate"] = {"passed": effect["ci95"][1] < 0 and effect["shorter_rate_difference"] >= 0 and max(ratios.values()) <= 2,
                      "effect": effect, "selection_time_ratio": ratios, "mechanism_checks_passed": True}
    report["finished_at"] = now()
    save(run / "comparison.json", report)
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", required=True, type=Path)
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
    with lock.open("x") as f:
        f.write(str(os.getpid()))
    deadline = time.time()+args.seconds
    try:
        freeze(run)
        if (run / "exit.json").exists() and json.loads((run / "exit.json").read_text())["exit_code"] == 0:
            raise ValueError("experiment already finished")
        extractor = run / "extract_v082_features"
        if not extractor.exists():
            compile_cpp(run / "frozen/adhoc/bin/extract_v082_features.cpp", extractor, run, deadline)
        if args.calibrate:
            stats = prepare(run, extractor, deadline, workers=20, limit=512)
            save(run / "prepare_calibration.json", stats)
            worker(run, "calibrate", deadline)
            prepare_inputs(run, deadline)
            save(run / "estimate.json", {"data": stats, "overhead_allowance_seconds": 180, "calibrated_at": now()})
            status(run, "ready_to_launch")
            return
        prepare(run, extractor, deadline, workers=20)
        from check_v082_relation import check_run
        checked = check_run(run)
        save(run / "relation_check.json", checked)
        worker(run, "tiny", deadline)
        for steps in STEPS:
            marker = run / "models" / NAME / f"finished_{steps}.json"
            if marker.exists():
                finished = json.loads(marker.read_text())
                assert sha(marker.parent / f"step_{steps}.pt") == finished["checkpoint_sha256"]
                assert sha(marker.parent / f"prediction_{steps}.npy") == finished["prediction_sha256"]
            else:
                worker(run, "train", deadline, steps)
        result = evaluate(run, deadline)
        if result["gate"]["passed"]:
            result["full_search"] = search_execute(run, deadline)
        else:
            result["full_search"] = {"executed": False, "reason": "candidate_gate_failed"}
        save(run / "result.json", result)
        effect = result["gate"]["effect"]
        (run / "REPORT.md").write_text("# v082 関係特徴の追加\n\n"
            f"- 保存候補の次段階条件: {'通過' if result['gate']['passed'] else '未達'}。\n"
            f"- 主検証の平均差: {effect['mean']:+.6f}手。95%区間: {effect['ci95']}。\n"
            f"- 短縮選択率差: {100*effect['shorter_rate_difference']:+.3f}ポイント。\n"
            f"- C++選択時間比: {result['gate']['selection_time_ratio']}。\n"
            "- 詳細はresult.json、comparison.json、cpp_result.json、search/comparison.jsonを参照する。\n")
        status(run, "finished", report=str(run / "REPORT.md"))
        save(run / "exit.json", {"exit_code": 0, "reason": "registered_experiment_finished", "finished_at": now()})
    except (TimeoutError, subprocess.TimeoutExpired):
        status(run, "paused", reason="registered_time_limit", resumable=True)
        save(run / "exit.json", {"exit_code": 75, "reason": "registered_time_limit", "finished_at": now()})
        raise SystemExit(75)
    except BaseException as error:
        status(run, "failed", error=repr(error))
        save(run / "exit.json", {"exit_code": 1, "reason": type(error).__name__, "error": str(error), "finished_at": now()})
        traceback.print_exc()
        raise SystemExit(1)
    finally:
        lock.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
