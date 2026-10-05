#!/usr/bin/env python3
"""v079の固定済み照合→機構確認→200入力比較を一度だけ実行する。"""
import argparse
import csv
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time
import traceback

from run_v077_overnight import input_properties, prior_inputs

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "results/nn_rank/v077/20261002T021222_studio"
TRAIN = ROOT / "results/nn_rank/v078/20261002T095446_studio"
BINS = ("v076_relative_tuned", "v079_nn_immediate")
ERRORS = ("baseline_recovery", "final_recovery", "construction_errors", "lns_errors", "lns_invalid_candidates")


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(value, indent=2) + "\n")
    temp.replace(path)


def status(run, stage, **values):
    save(run / "status.json", {"stage": stage, "updated_at": datetime.now(timezone.utc).isoformat(), **values})
    print(stage, flush=True)


def command(args, log, input_path=None, output_path=None):
    # 子も同じ群に置き、外側のmemory_guardから一括停止できるようにする。
    with log.open("w") as err:
        inp = input_path.open("rb") if input_path else subprocess.DEVNULL
        out = output_path.open("wb") if output_path else err
        try:
            started = time.monotonic()
            subprocess.run(list(map(str, args)), cwd=ROOT, stdin=inp, stdout=out, stderr=err, check=True)
            return (time.monotonic() - started) * 1000
        finally:
            if input_path:
                inp.close()
            if output_path:
                out.close()


def prepare(run):
    import numpy as np
    import torch
    torch.set_num_threads(1)
    torch.set_num_interop_threads(1)
    assert not (run / "prepared.json").exists()
    status(run, "preparing_saved_probes")
    model = json.loads((run / "model.json").read_text())
    net = torch.nn.Sequential(torch.nn.Linear(32, 16), torch.nn.ReLU(), torch.nn.Linear(16, 1))
    net.load_state_dict({k: torch.tensor(v, dtype=torch.float32) for k, v in model["weights"].items()})
    net.eval()
    mean, scale = (np.asarray(model[k], dtype=np.float32) for k in ("mean", "scale"))
    items = sorted((x for x in json.loads((TRAIN / "inputs.json").read_text()) if x["role"] == "train"), key=lambda x: x["index"])[:32]
    assert len(items) == 32 and items[0]["M"] < 80 <= items[1]["M"]
    (run / "probes").mkdir()
    probes = []
    for item in items:
        case = DATA / "cases" / f"{item['index']:06d}"
        source = DATA / item["path"]
        assert sha(source) == item["sha256"]
        complete = json.loads((case / "complete.json").read_text())
        for name in ("states", "candidates"):
            assert sha(case / (name + ".jsonl")) == complete[name + "_sha256"]
        states = sorted(map(json.loads, (case / "states.jsonl").read_text().splitlines()), key=lambda x: x["phase"])
        rows = list(map(json.loads, (case / "candidates.jsonl").read_text().splitlines()))
        assert [x["phase"] for x in states] == list(range(4))
        lines = source.read_text().splitlines() + [str(len(states))]
        expected = []
        for state in states:
            group = sorted((x for x in rows if x["phase"] == state["phase"]), key=lambda x: x["rank"])
            candidates = {x["rank"]: x for x in state["candidates"]}
            raw = np.asarray([x["features"] for x in group], dtype=np.float32)
            assert 0 < len(group) <= 32 and len(group) == len(candidates)
            with torch.no_grad():
                predictions = net(torch.from_numpy((raw - mean) / scale)).flatten().tolist()
            expected.append({"phase": state["phase"], "ranks": [x["rank"] for x in group],
                             "features": [x["features"] for x in group], "scores": predictions,
                             "choice": group[min(range(len(group)), key=predictions.__getitem__)]["rank"]})
            lines.append(f"{state['phase']} {state['stagnant']} {state['progress']:.17g} {int(raw[0,30])} {int(raw[0,31])}")
            for key in ("current", "best"):
                lines.append(str(len(state[key])))
                lines.extend(" ".join(map(str, move)) for move in state[key])
            lines.append(str(len(group)))
            for row in group:
                candidate = candidates[row["rank"]]
                assert candidate["hash"] == row["candidate_hash"] and row["snapshot_hash"] == state["snapshot_hash"]
                lines.append(f"{row['rank']} {candidate['hash']} {row['features'][19]:.17g} {len(candidate['ids'])} " + " ".join(map(str, candidate["ids"])))
        stem = f"{item['index']:06d}"
        (run / "probes" / (stem + ".input")).write_text("\n".join(lines) + "\n")
        save(run / "probes" / (stem + ".expected.json"), expected)
        probes.append({**item, "stem": stem, "source": str(source.relative_to(ROOT))})
    save(run / "probe_manifest.json", probes)

    status(run, "excluding_known_inputs")
    seeds, hashes = prior_inputs(run)
    for name in ("inputs.jsonl", "generated.jsonl"):
        with (DATA / name).open() as stream:
            for line in stream:
                item = json.loads(line)
                seeds.add(item["seed"]); hashes.add(item["sha256"])
    save(run / "exclusions.json", {"seeds": sorted(seeds), "hashes": sorted(hashes),
                                   "extra_manifests": [str(DATA / n) for n in ("inputs.jsonl", "generated.jsonl")]})
    status(run, "generating_200_inputs")
    (run / "inputs").mkdir()
    generated, index, manifest = run / "generated", 920000000, []
    while len(manifest) < 200:
        batch = []
        while len(batch) < 200 - len(manifest):
            if index not in seeds:
                batch.append(index)
            index += 1
        seed_file = run / f"seeds_{batch[0]}.txt"
        seed_file.write_text("".join(f"{seed}\n" for seed in batch))
        folder = generated / str(batch[0])
        folder.mkdir(parents=True)
        command([ROOT / "tools/target/release/gen", seed_file, "--dir", folder], run / f"gen_{batch[0]}.log")
        for i, seed in enumerate(batch):
            source = folder / f"{i:04d}.txt"
            digest = sha(source)
            seeds.add(seed)
            if digest in hashes:
                continue
            hashes.add(digest)
            name = f"{len(manifest):04d}.txt"
            props = input_properties(source.read_bytes())
            assert props is not None
            shutil.copy2(source, run / "inputs" / name)
            manifest.append({"case": name, "seed": seed, "sha256": digest, **props})
    save(run / "input_manifest.json", manifest)
    assert len(manifest) == len({x["sha256"] for x in manifest}) == 200
    source_paths = [ROOT / f"src/bin/{name}.cpp" for name in BINS] + [
        ROOT / x for x in ("adhoc/bin/check_v079_nn.cpp", "adhoc/scripts/run_v079_experiment.py",
            "adhoc/scripts/memory_guard.py", "adhoc/scripts/build_v079_nn.py", "adhoc/scripts/v079_nn_members.cpp.txt",
            "adhoc/scripts/v077_teacher_support.cpp.txt", "adhoc/scripts/run_v077_overnight.py",
            "scripts/eval.py", "scripts/build_solver.sh")]
    for path in source_paths:
        destination = run / "frozen" / "sources" / path.relative_to(ROOT)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, destination)
    frozen = [p for p in (run / "frozen").rglob("*") if p.is_file()]
    immutable = [run / "model.json", run / "source_audit.json", run / "input_manifest.json", run / "probe_manifest.json"]
    immutable += list((run / "probes").iterdir()) + list((run / "inputs").iterdir()) + frozen + source_paths
    save(run / "prepared.json", {"sha256": {str(p.relative_to(ROOT)): sha(p) for p in immutable},
                                "inputs": 200, "saved_inputs": 32, "saved_groups": 128,
                                "jobs": 20, "training_device": "none", "prediction_reference": "torch_cpu",
                                "platform": subprocess.check_output(["uname", "-a"], text=True).strip(),
                                "chip": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip(),
                                "compiler": subprocess.check_output(["g++-15", "--version"], text=True).splitlines()[0]})
    status(run, "prepared")


def verify_frozen(run):
    for path, digest in json.loads((run / "prepared.json").read_text())["sha256"].items():
        assert sha(ROOT / path) == digest, f"changed frozen file: {path}"


def saved_probes(run):
    result = {"groups": 0, "max_feature_error": 0., "max_score_error": 0., "timings_ms": {}}
    for mode in ("local", "nonlocal"):
        timings = []
        for item in json.loads((run / "probe_manifest.json").read_text()):
            stem = item["stem"]
            output = run / "probes" / f"{stem}.{mode}.jsonl"
            command([run / "frozen" / f"check_v079_nn_{mode}"], output.with_suffix(".err"),
                    run / "probes" / f"{stem}.input", output)
            actual = list(map(json.loads, output.read_text().splitlines()))
            expected = json.loads((run / "probes" / f"{stem}.expected.json").read_text())
            assert len(actual) == len(expected) == 4
            for got, want in zip(actual, expected):
                assert got["phase"] == want["phase"] and got["choice"] == want["choice"], (stem, mode, "choice", got["phase"])
                assert len(got["features"]) == len(want["features"]) and len(got["scores"]) == len(want["scores"])
                for a, b in zip(got["features"], want["features"]):
                    assert len(a) == len(b) == 32
                    for j, (x, y) in enumerate(zip(a, b)):
                        assert math.isfinite(x) and abs(x-y) <= 1e-10 + 1e-12*abs(y), (stem, mode, "feature", j, x, y)
                        result["max_feature_error"] = max(result["max_feature_error"], abs(x-y))
                for x, y in zip(got["scores"], want["scores"]):
                    assert math.isfinite(x) and abs(x-y) <= 2e-4 + 2e-5*abs(y), (stem, mode, "score", x, y)
                    result["max_score_error"] = max(result["max_score_error"], abs(x-y))
                timings.append(got["selection_ms"])
                result["groups"] += 1
        result["timings_ms"][mode] = {"mean": sum(timings)/len(timings), "max": max(timings)}
    result["passed"] = True
    save(run / "probe_result.json", result)


def replay(input_path, output_path):
    rows = input_path.read_text().splitlines()
    n, _ = map(int, rows[0].split())
    cells = "".join(rows[1:n+1])
    towers = [[ord(c)-ord("a")] if "a" <= c <= "z" else [] for c in cells]
    homes = {p: ord(c)-ord("A") for p, c in enumerate(cells) if "A" <= c <= "Z"}
    moves = output_path.read_text().splitlines()
    assert len(moves) <= 100000
    directions = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}
    for line in moves:
        row, col, keep, direction, length = line.split()
        row, col, keep, length = map(int, (row, col, keep, length))
        assert 0 <= row < n and 0 <= col < n and direction in directions
        p = row*n+col
        assert 0 <= keep < len(towers[p]) and 1 <= length <= keep+1
        dr, dc = directions[direction]
        for step in range(1, length+1):
            r, c = row+dr*step, col+dc*step
            assert 0 <= r < n and 0 <= c < n and cells[r*n+c] != "#"
        q = (row+dr*length)*n+col+dc*length
        moving = towers[p][keep:]
        assert len(towers[q])+len(moving) <= 8
        towers[p] = towers[p][:keep]
        towers[q].extend(reversed(moving))
        for at in (p, q):
            while towers[at] and towers[at][-1] == homes.get(at, -1):
                towers[at].pop()
    assert not any(towers), str(output_path)
    return len(moves)


def local_log(path, T, require_nn=False):
    text = path.read_text()
    counts = {k: int(v) for k, v in re.findall(r"\[summary.count\] ([^=]+)=(-?\d+)", text)}
    times = {k: float(v) for k, v in re.findall(r"\[summary.time_ms\] ([^=]+)=([\d.]+)", text)}
    assert "diagnostic:" not in text and all(counts[k] == 0 for k in ERRORS)
    assert T == counts["T"] == counts["final_ops"] == counts["validated_moves"] and counts["E"] == 0
    assert counts["state_pool_free_at_end"] == counts["state_slots"] == 4
    assert times["search_limit"] == 1544.0
    if require_nn:
        assert counts["nn_calls"] > 0 and counts["nn_changed_choices"] > 0
        assert counts["nn_calls"] <= counts["nn_candidates_scored"] <= 32*counts["nn_calls"]
    return {"counts": counts, "times_ms": times}


def live_mechanism(run):
    folder = run / "mechanism"
    folder.mkdir()
    result = []
    for item in json.loads((run / "probe_manifest.json").read_text())[:2]:
        for mode in ("local", "nonlocal"):
            output = folder / f"{item['stem']}.{mode}.txt"
            err = output.with_suffix(".txt.err")
            inp = ROOT / item["source"]
            elapsed = command([run / "frozen" / f"v079_nn_immediate_{mode}"], err, inp, output)
            T = replay(inp, output)
            scored = subprocess.check_output([str(ROOT / "tools/target/release/vis"), str(inp), str(output)], cwd=ROOT, text=True).strip()
            assert scored == f"Score = {T}", scored
            checked = local_log(err, T, True) if mode == "local" else {}
            result.append({"index": item["index"], "M": item["M"], "mode": mode, "T": T, "elapsed_ms": elapsed, **checked})
    save(run / "mechanism_result.json", {"passed": True, "runs": result})


def evaluate(run, name):
    verify_frozen(run)
    output = ROOT / "results/out" / name
    previous = run / "previous_out" / name
    previous.parent.mkdir(exist_ok=True)
    if output.exists():
        assert not previous.exists()
        shutil.move(str(output), previous)
    label = f"{run.name}_v079_{name}"
    command([sys.executable, ROOT / "scripts/eval.py", name, run / "inputs", "-j", "20", "--label", label], run / f"eval_{name}.log")
    archived = run / "outputs" / name
    archived.parent.mkdir(exist_ok=True)
    shutil.copytree(output, archived)
    records = []
    with (ROOT / "results/eval_records.jsonl").open() as stream:
        for line in stream:
            row = json.loads(line)
            if row["label"] == label:
                records.append(row)
    assert len(records) == len({x["case_name"] for x in records}) == 200
    assert len({x["run_id"] for x in records}) == 1
    save(run / f"records_{name}.json", records)
    shutil.copy2(ROOT / "target/release" / name, run / "frozen" / (name + "_evaluated"))
    save(run / f"evaluated_binary_{name}.json", {"sha256": sha(ROOT / "target/release" / name)})
    return {r["case_name"]: r for r in records}


def comparison(run, all_records):
    cases = []
    for item in json.loads((run / "input_manifest.json").read_text()):
        case = {**item, "versions": {}}
        for name in BINS:
            record = all_records[name][item["case"]]
            assert record["status"] == "ok" and record["local"] and record["bin"] == name
            assert (ROOT / record["input_dir"]).resolve() == run / "inputs"
            output = run / "outputs" / name / item["case"]
            T = replay(run / "inputs" / item["case"], output)
            assert T == record["score"] and T > 0
            checked = local_log(output.with_suffix(".txt.err"), T)
            case["versions"][name] = {"T": T, "elapsed_ms": record["elapsed"], **checked}
        base, new = (case["versions"][x]["T"] for x in BINS)
        ref = min(base, new)
        case.update(delta_T=new-base, relative_base=(2*10**9*ref+base)//(2*base), relative_new=(2*10**9*ref+new)//(2*new))
        cases.append(case)
    summary = {}
    for name in BINS:
        rows = [x["versions"][name] for x in cases]
        keys = {k for r in rows for k in r["counts"]}
        summary[name] = {"total_T": sum(r["T"] for r in rows), "max_elapsed_ms": max(r["elapsed_ms"] for r in rows),
                         "mean_elapsed_ms": sum(r["elapsed_ms"] for r in rows)/200,
                         "counts": {k: sum(r["counts"].get(k, 0) for r in rows) for k in keys},
                         "mean_times_ms": {k: sum(r["times_ms"].get(k, 0) for r in rows)/200 for k in {k for r in rows for k in r["times_ms"]}}}
    delta = sum(c["delta_T"] for c in cases)
    rel_delta = sum(c["relative_new"]-c["relative_base"] for c in cases)/200
    result = {"cases": 200, "versions": summary, "delta_T": delta, "mean_delta_T": delta/200,
              "relative_mean_delta": rel_delta, "relative_delta_points_of_100": rel_delta/1e7,
              "wins": sum(c["delta_T"] < 0 for c in cases), "draws": sum(c["delta_T"] == 0 for c in cases),
              "losses": sum(c["delta_T"] > 0 for c in cases),
              "groups": {label: {"cases": len(rows), "delta_T": sum(c["delta_T"] for c in rows),
                                   "mean_delta_T": sum(c["delta_T"] for c in rows)/len(rows)}
                         for label, rows in (("M_lt_80", [c for c in cases if c["M"] < 80]), ("M_ge_80", [c for c in cases if c["M"] >= 80]))},
              "all_legal_E0": True, "all_existing_errors_zero": True,
              "mechanism_passed": json.loads((run / "mechanism_result.json").read_text())["passed"] and json.loads((run / "probe_result.json").read_text())["passed"]}
    result["adopt_for_m3_test"] = bool(result["mechanism_passed"] and delta < 0 and rel_delta > 0 and all(summary[n]["max_elapsed_ms"] <= 2000 for n in BINS))
    save(run / "case_results.json", cases)
    save(run / "comparison.json", result)
    with (run / "case_comparison.csv").open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(("case", "M", "v076_T", "v079_T", "delta_T", "relative_delta", "nn_calls", "nn_changed_choices", "nn_selection_ms"))
        for c in cases:
            base, new = (c["versions"][n] for n in BINS)
            writer.writerow((c["case"], c["M"], base["T"], new["T"], c["delta_T"], c["relative_new"]-c["relative_base"], new["counts"]["nn_calls"], new["counts"]["nn_changed_choices"], new["times_ms"]["nn_selection"]))
    (run / "REPORT.md").write_text(f"# v079 Studio比較\n\n- 200入力、LOCAL・20並列、v076とv079各1回。\n- 合計手数差: {delta:+d}、平均差: {delta/200:+.5f}。\n- 擬似相対差（100点換算）: {rel_delta/1e7:+.6f}。\n- 勝／分／負: {result['wins']}／{result['draws']}／{result['losses']}。\n- M3評価へ進める条件: {'通過' if result['adopt_for_m3_test'] else '不通過'}。\n- 全入力の合法性・E=0・既存エラー0・盤面領域返却を確認した。\n- 詳細はcomparison.json、case_comparison.csv、probe_result.json、mechanism_result.json、memory_execute/exit.jsonに保存する。\n\n単回の実時間比較である。M3での最終採否は未評価であり、結果に基づくコードの改善は行わない。\n")
    return result


def execute(run):
    # 再起動時の無断再評価を防ぐ。再開には保存結果を確認した別の判断が必要。
    with (run / "execution_started.json").open("x") as stream:
        json.dump({"started_at": datetime.now(timezone.utc).isoformat()}, stream)
    verify_frozen(run)
    status(run, "checking_saved_probes")
    saved_probes(run)
    status(run, "checking_live_mechanism")
    live_mechanism(run)
    records = {}
    for name in BINS:
        status(run, "evaluating", solver=name, cases=200, jobs=20)
        records[name] = evaluate(run, name)
    status(run, "analyzing")
    verify_frozen(run)
    result = comparison(run, records)
    status(run, "finished", adopt_for_m3_test=result["adopt_for_m3_test"], delta_T=result["delta_T"])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("stage", choices=("prepare", "execute"))
    parser.add_argument("--run", type=Path, required=True)
    args = parser.parse_args()
    run = args.run.resolve()
    def interrupted(signum, frame):
        raise InterruptedError(f"signal {signum}")
    signal.signal(signal.SIGTERM, interrupted)
    try:
        {"prepare": prepare, "execute": execute}[args.stage](run)
    except BaseException as error:
        status(run, "failed", operation=args.stage, error=repr(error))
        (run / f"{args.stage}_failure.txt").write_text(traceback.format_exc())
        raise


if __name__ == "__main__":
    main()
