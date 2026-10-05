#!/usr/bin/env python3
"""v082の事前登録を通過した最終重みだけで、Studioの探索全体を比較する。"""
import csv
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

from v080_data import ROOT, SOURCE, now, save, sha, status
from run_v077_overnight import input_properties, prior_inputs
from run_v079_experiment import replay, local_log

BINS = ("v076_relative_tuned", "v079_nn_immediate", "v082_nn_relation")


def command(args, log, deadline, input_path=None, output_path=None):
    if time.time() >= deadline:
        raise TimeoutError("registered_time_limit")
    with log.open("w") as err:
        inp = input_path.open("rb") if input_path else subprocess.DEVNULL
        out = output_path.open("wb") if output_path else err
        try:
            started = time.monotonic()
            subprocess.run(list(map(str, args)), cwd=ROOT, stdin=inp, stdout=out, stderr=err,
                           timeout=max(1, deadline-time.time()), check=True)
            return (time.monotonic()-started)*1000
        finally:
            if input_path:
                inp.close()
            if output_path:
                out.close()


def prepare_inputs(run, deadline):
    folder = run / "search"
    folder.mkdir(exist_ok=True)
    manifest_path = folder / "input_manifest.json"
    if manifest_path.exists():
        manifest = json.loads(manifest_path.read_text())
        assert len(manifest) == 200
        for item in manifest:
            assert sha(folder / "inputs" / item["case"]) == item["sha256"]
        return
    status(run, "excluding_known_search_inputs")
    seeds, hashes = prior_inputs(folder)
    for name in ("inputs.jsonl", "generated.jsonl"):
        for line in (SOURCE / name).read_text().splitlines():
            item = json.loads(line)
            seeds.add(item["seed"])
            hashes.add(item["sha256"])
    save(folder / "exclusions.json", {"seeds": sorted(seeds), "hashes": sorted(hashes)})
    (folder / "inputs").mkdir(exist_ok=True)
    manifest, index = [], 930000000
    while len(manifest) < 200:
        batch = []
        while len(batch) < 200-len(manifest):
            if index not in seeds:
                batch.append(index)
            index += 1
        seed_file = folder / f"seeds_{batch[0]}.txt"
        seed_file.write_text("".join(f"{seed}\n" for seed in batch))
        generated = folder / "generated" / str(batch[0])
        generated.mkdir(parents=True)
        command([ROOT / "tools/target/release/gen", seed_file, "--dir", generated],
                folder / f"gen_{batch[0]}.log", deadline)
        for i, seed in enumerate(batch):
            source = generated / f"{i:04d}.txt"
            digest = sha(source)
            seeds.add(seed)
            if digest in hashes:
                continue
            hashes.add(digest)
            props = input_properties(source.read_bytes())
            assert props is not None
            name = f"{len(manifest):04d}.txt"
            shutil.copyfile(source, folder / "inputs" / name)
            manifest.append({"case": name, "seed": seed, "sha256": digest, **props})
    assert len({item["sha256"] for item in manifest}) == 200
    save(manifest_path, manifest)


def execute(run, deadline):
    folder = run / "search"
    assert json.loads((run / "comparison.json").read_text())["gate"]["passed"]
    if (folder / "comparison.json").exists():
        return json.loads((folder / "comparison.json").read_text())
    # 各版は1回だけ評価する。中断した評価を自動で再実行して隠さない。
    with (folder / "started.json").open("x") as f:
        json.dump({"started_at": now(), "versions": BINS, "jobs": 20}, f)
    integration = json.loads((run / "integration.json").read_text())
    integrated = run / "integrated/src/bin/v082_nn_relation.cpp"
    assert sha(integrated) == integration["integrated_sha256"]
    root_source = ROOT / "src/bin/v082_nn_relation.cpp"
    assert sha(root_source) == integration["template_sha256"]
    shutil.copyfile(integrated, root_source)
    save(folder / "solver_sha256.json", {name: sha(ROOT / f"src/bin/{name}.cpp") for name in BINS})
    status(run, "search_mechanism_check")
    mechanism = folder / "mechanism"
    mechanism.mkdir()
    rows = []
    for mode in ("local", "nonlocal"):
        args = [ROOT / "scripts/build_solver.sh"]
        if mode == "nonlocal":
            args.append("--no-local")
        args.append("v082_nn_relation")
        command(args, mechanism / f"build_{mode}.log", deadline)
        binary = mechanism / f"v082_nn_relation_{mode}"
        shutil.copyfile(ROOT / "target/release/v082_nn_relation", binary)
        binary.chmod(0o755)
        for item in json.loads((run / "probe_manifest.json").read_text())[:2]:
            inp = ROOT / item["source"]
            out = mechanism / f"{item['stem']}_{mode}.txt"
            err = out.with_suffix(".err")
            elapsed = command([binary], err, deadline, inp, out)
            T = replay(inp, out)
            scored = subprocess.check_output([str(ROOT / "tools/target/release/vis"), str(inp), str(out)],
                                             text=True, cwd=ROOT).strip()
            assert scored == f"Score = {T}"
            details = local_log(err, T, True) if mode == "local" else {}
            rows.append({"index": item["index"], "M": item["M"], "mode": mode, "T": T,
                         "elapsed_ms": elapsed, **details})
    save(folder / "mechanism_result.json", {"passed": True, "runs": rows})
    records = {}
    source_hashes = json.loads((folder / "solver_sha256.json").read_text())
    for name in BINS:
        assert sha(ROOT / f"src/bin/{name}.cpp") == source_hashes[name]
        status(run, "evaluating_full_search", solver=name, cases=200, jobs=20)
        scratch = ROOT / "results/out" / name
        if scratch.exists():
            archive = folder / "previous_out" / name
            archive.parent.mkdir(exist_ok=True)
            assert not archive.exists()
            shutil.move(str(scratch), archive)
        label = f"{run.name}_v082_{name}"
        command([sys.executable, ROOT / "scripts/eval.py", name, folder / "inputs", "-j", "20", "--label", label],
                folder / f"eval_{name}.log", deadline)
        output = folder / "outputs" / name
        output.parent.mkdir(exist_ok=True)
        shutil.copytree(scratch, output)
        found = []
        with (ROOT / "results/eval_records.jsonl").open() as stream:
            for line in stream:
                item = json.loads(line)
                if item["label"] == label:
                    found.append(item)
        assert len(found) == len({r["case_name"] for r in found}) == 200
        assert len({r["run_id"] for r in found}) == 1
        records[name] = {r["case_name"]: r for r in found}
        save(folder / f"records_{name}.json", found)
        shutil.copyfile(ROOT / "target/release" / name, folder / f"{name}_evaluated")
    cases = []
    for item in json.loads((folder / "input_manifest.json").read_text()):
        case = {**item, "versions": {}}
        for name in BINS:
            record = records[name][item["case"]]
            assert record["status"] == "ok" and record["local"] and record["bin"] == name
            assert (ROOT / record["input_dir"]).resolve() == folder / "inputs"
            output = folder / "outputs" / name / item["case"]
            T = replay(folder / "inputs" / item["case"], output)
            assert T == record["score"] and T > 0
            details = local_log(output.with_suffix(".txt.err"), T, name != BINS[0])
            case["versions"][name] = {"T": T, "elapsed_ms": record["elapsed"], **details}
        reference = min(v["T"] for v in case["versions"].values())
        for value in case["versions"].values():
            T = value["T"]
            value["relative"] = (2*10**9*reference+T)//(2*T)
        cases.append(case)
    summary = {}
    for name in BINS:
        values = [c["versions"][name] for c in cases]
        summary[name] = {"total_T": sum(v["T"] for v in values),
                         "max_elapsed_ms": max(v["elapsed_ms"] for v in values),
                         "mean_times_ms": {k: sum(v["times_ms"].get(k, 0) for v in values)/200
                                           for k in {k for v in values for k in v["times_ms"]}}}
    comparisons = {}
    for base in BINS[:2]:
        differences = [c["versions"][BINS[2]]["T"]-c["versions"][base]["T"] for c in cases]
        relative = sum(c["versions"][BINS[2]]["relative"]-c["versions"][base]["relative"] for c in cases)/200
        groups = {}
        for label, high in (("M_lt80", False), ("M_ge80", True)):
            selected = [d for d, c in zip(differences, cases) if (c["M"] >= 80) == high]
            groups[label] = {"cases": len(selected), "mean_delta_T": sum(selected)/len(selected)}
        comparisons[base] = {"delta_T": sum(differences), "mean_delta_T": sum(differences)/200,
                             "relative_mean_delta": relative,
                             "wins": sum(d < 0 for d in differences), "draws": differences.count(0),
                             "losses": sum(d > 0 for d in differences), "groups": groups}
    effect = comparisons[BINS[1]]
    result = {"finished_at": now(), "cases": 200, "versions": summary, "comparisons": comparisons,
              "all_legal_E0": True, "all_existing_errors_zero": True, "mechanism_passed": True,
              "adopt_for_m3_test": effect["delta_T"] < 0 and effect["relative_mean_delta"] > 0
                 and all(v["max_elapsed_ms"] <= 2000 for v in summary.values()),
              "mean_5_move_goal_reached": comparisons[BINS[0]]["mean_delta_T"] <= -5}
    save(folder / "case_results.json", cases)
    save(folder / "comparison.json", result)
    with (folder / "case_comparison.csv").open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["case", "M", *BINS, "delta_v079", "delta_v076"])
        for c in cases:
            v = [c["versions"][name]["T"] for name in BINS]
            writer.writerow([c["case"], c["M"], *v, v[2]-v[1], v[2]-v[0]])
    return result
