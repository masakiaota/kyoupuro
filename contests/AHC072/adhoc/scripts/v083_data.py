#!/usr/bin/env python3
"""v083の枝の対応検査、固定教師の集計、入力単位の区間推定。"""
import hashlib
import json
import math
from pathlib import Path

import numpy as np

from v080_data import now, save, sha, status

ROOT = Path(__file__).resolve().parents[2]
REPLICAS, STEPS, PHASES, CANDIDATES = 8, 512, 4, 32
ARRAYS = ("raw", "x", "mask", "ranks", "meta", "baseline", "immediate", "future", "first_best",
          "checkpoints", "immediate_advantage", "future_advantage")


def read_jsonl(path):
    return [json.loads(line) for line in path.read_text().splitlines()]


def branch_seed(index, phase, replica, step):
    mask = (1 << 64) - 1
    value = 0x083f3a9c75612be1
    for item in (index, phase, replica, step, 0x1234 if step == 0 else 0x5678):
        value = ((value ^ item) * 1099511628211) & mask
    value = (value + 0x9e3779b97f4a7c15) & mask
    value = ((value ^ (value >> 30)) * 0xbf58476d1ce4e5b9) & mask
    value = ((value ^ (value >> 27)) * 0x94d049bb133111eb) & mask
    return (value ^ (value >> 31)) or 0x083


def validate_case(folder, item, replicas=REPLICAS, steps=STEPS, phases=PHASES, candidates=CANDIDATES, repeat=False):
    """欠損や打ち切りを除外して成功扱いにせず、入力全体を失敗させる。"""
    if (folder / "failures.jsonl").exists() and (folder / "failures.jsonl").stat().st_size:
        raise ValueError(f"collector failure: {folder}")
    parent = json.loads((folder / "parent.json").read_text())
    states, rows = read_jsonl(folder / "states.jsonl"), read_jsonl(folder / "candidates.jsonl")
    assert parent["input_index"] == item["index"] and parent["parent_unchanged"]
    assert parent["errors"] == 0 and parent["pool_free"] == 4
    assert 1 <= len(states) == parent["snapshots"] <= phases
    assert parent["requested_phases"] == phases
    assert [s["phase"] for s in states] == list(range(len(states)))
    expected = {}
    for state in states:
        assert state["input_index"] == item["index"] and state["pool_free"] == 4
        assert state["replicas"] == replicas and state["steps"] == steps
        assert state["snapshot_hash"] == state["parent_fingerprint"]
        assert 0 <= state["progress"] <= 1
        cs = state["candidates"]
        assert 1 <= len(cs) <= candidates and len({c["rank"] for c in cs}) == len(cs)
        assert state["baseline_rank"] in {c["rank"] for c in cs}
        for candidate in cs:
            assert len(set(candidate["ids"])) == len(candidate["ids"]) > 0
            for replica in range(replicas):
                expected[state["phase"], candidate["rank"], replica] = (state, candidate)
    seen, duplicates, original = set(), [], {}
    outcomes = {"accepted", "extract_failed", "insert_failed", "duplicate", "rejected"}
    first_seeds, continuation = {}, {}
    for row in rows:
        key = row["phase"], row["rank"], row["replica"]
        assert key in expected and row["input_index"] == item["index"]
        state, candidate = expected[key]
        assert row["snapshot_hash"] == state["snapshot_hash"] and row["candidate_hash"] == candidate["hash"]
        assert row["complete"] and row["first_completed"] and row["first_outcome"] in outcomes
        assert row["pool_free"] == 4 and row["errors"] == row["E"] == 0
        assert row["requested_steps"] == steps and 0 <= row["completed_steps"] <= steps
        if row["absorbed"]:
            assert row["stop_reason"] == "no_candidates"
        else:
            assert row["completed_steps"] == steps and row["stop_reason"] == "completed"
        f = row["features"]
        assert f == candidate["features"] and len(f) == 32 and np.isfinite(f).all()
        assert f[2] == item["M"] and f[5] == len(state["current"]) and f[6] == len(state["best"])
        assert row["best_T"] <= row["first_best_T"] <= f[6]
        assert row["first_best_T"] <= row["first_T"] and row["best_T"] <= row["current_T"]
        if row["first_outcome"] != "accepted":
            assert row["first_T"] == f[5] and row["first_best_T"] == f[6]
        assert 0 <= row["first_wall_seconds"] <= row["wall_seconds"] <= 10.1
        assert row["virtual_ticks"] >= row["first_virtual_ticks"] >= 0
        seed = branch_seed(item["index"], key[0], key[2], 0)
        assert int(row["seed_first"]) == seed
        first_seeds[key[0], key[2]] = seed
        # 同じ継続反復数なら、候補によらず投入した乱数列も同じになる。
        continuation_key = (key[0], key[2], row["completed_steps"])
        if continuation_key in continuation:
            assert continuation[continuation_key] == row["seed_digest"]
        continuation[continuation_key] = row["seed_digest"]
        checkpoints = row["checkpoints"]
        assert [p["step"] for p in checkpoints] == [128, 256, 512]
        best, ticks = row["first_best_T"], row["first_virtual_ticks"]
        for point in checkpoints:
            assert point["best_T"] <= best and point["best_T"] <= point["current_T"]
            assert point["ticks"] >= ticks
            assert math.isfinite(point["temperature"]) and point["temperature"] > 0
            assert 0 <= point["progress"] <= 1
            best, ticks = point["best_T"], point["ticks"]
        assert checkpoints[-1]["best_T"] == row["best_T"]
        assert checkpoints[-1]["current_T"] == row["current_T"]
        assert all(v >= 0 for v in row["stats"].values())
        if row["repeat"]:
            duplicates.append(row)
        else:
            assert key not in seen
            seen.add(key)
            original[key] = row
    assert seen == set(expected)
    assert len(duplicates) == int(repeat)
    for row in duplicates:
        key = row["phase"], row["rank"], row["replica"]
        assert key == (0, states[0]["candidates"][0]["rank"], 0)
        clean = lambda r: {k: v for k, v in r.items() if k not in ("repeat", "wall_seconds", "first_wall_seconds")}
        assert clean(row) == clean(original[key]), "deterministic branch differs"
    assert len(set(first_seeds.values())) == len(first_seeds)
    result = {"passed": True, "input_index": item["index"], "snapshots": len(states),
              "rows": len(rows), "branches": len(seen), "duplicates_exact": len(duplicates),
              "absorbed": sum(r["absorbed"] for r in original.values()), "best_T": parent["best_T"],
              "branch_wall_seconds": sum(r["wall_seconds"] for r in original.values()),
              "max_branch_seconds": max(r["wall_seconds"] for r in original.values()),
              "stats": {k: sum(r["stats"][k] for r in original.values()) for k in rows[0]["stats"]},
              "sha256": {name: sha(folder / name) for name in ("states.jsonl", "candidates.jsonl", "parent.json")}}
    return result, states, list(original.values())


def assemble(run, items, output, deadline):
    import time
    output.mkdir(exist_ok=True)
    count = len(items) * PHASES
    shape = (count, CANDIDATES)
    values = {"raw": np.zeros((*shape, 32), np.float64), "mask": np.zeros(shape, bool),
              "ranks": np.full(shape, 100000, np.int32), "baseline": np.full(count, -1, np.int32),
              "meta": np.zeros((count, 4), np.int32),
              "checkpoints": np.zeros((*shape, REPLICAS, 3), np.int32)}
    for name in ("immediate", "future", "first_best"):
        values[name] = np.zeros((*shape, REPLICAS), np.float32)
    audit = hashlib.sha256()
    for pos, item in enumerate(items):
        if time.time() >= deadline:
            raise TimeoutError("registered_time_limit")
        folder = run / "cases" / f"{item['index']:06d}"
        complete = json.loads((folder / "complete.json").read_text())
        assert complete["input_sha256"] == item["sha256"]
        for name, digest in complete["sha256"].items():
            assert sha(folder / name) == digest
        # 採取直後に全枝を検査済み。ここでは保存ハッシュからその検査対象との同一性を確認する。
        states, rows = read_jsonl(folder / "states.jsonl"), read_jsonl(folder / "candidates.jsonl")
        audit.update(json.dumps(complete["sha256"], sort_keys=True).encode())
        slots = {}
        for phase in range(PHASES):
            values["meta"][pos*PHASES+phase] = (item["index"], item["role"] == "validation", item["M"], phase)
        for state in states:
            g = pos*PHASES+state["phase"]
            for c, candidate in enumerate(sorted(state["candidates"], key=lambda x: x["rank"])):
                slots[state["phase"], candidate["rank"]] = (g, c)
                values["raw"][g, c] = candidate["features"]
                values["mask"][g, c] = True
                values["ranks"][g, c] = candidate["rank"]
                if candidate["rank"] == state["baseline_rank"]:
                    values["baseline"][g] = c
        for row in rows:
            assert not row["repeat"]
            g, c = slots[row["phase"], row["rank"]]
            r = row["replica"]
            values["immediate"][g, c, r] = row["first_T"]
            values["future"][g, c, r] = row["best_T"]
            values["first_best"][g, c, r] = row["first_best_T"]
            values["checkpoints"][g, c, r] = [p["best_T"] for p in row["checkpoints"]]
    train_mask = values["mask"] & (values["meta"][:, 1] == 0)[:, None]
    valid = values["mask"].any(1)
    assert np.all(values["baseline"][valid] >= 0)
    raw32 = values["raw"].astype(np.float32)
    reference = raw32[train_mask].astype(np.float64)
    mean, scale = reference.mean(0).astype(np.float32), reference.std(0).astype(np.float32)
    scale[scale < 1e-6] = 1
    values["x"] = (raw32 - mean) / scale
    values["x"][~values["mask"]] = 0
    for name in ("immediate", "future"):
        target = values[name].mean(2)
        base = target[np.arange(count), np.maximum(0, values["baseline"])]
        values[name+"_advantage"] = np.where(values["mask"], target-base[:, None], 0)
    for name, value in values.items():
        assert np.isfinite(value).all()
        path = output / f"{name}.npy"
        temp = output / f"{name}.tmp.npy"
        np.save(temp, value)
        temp.replace(path)
    description = {"inputs": len(items), "groups": int(valid.sum()), "candidates": int(values["mask"].sum()),
                   "mean": mean.tolist(), "scale": scale.tolist(), "source_audit_sha256": audit.hexdigest(),
                   "normalization_training_inputs": sum(x["role"] == "train" for x in items),
                   "files": {name: sha(output / f"{name}.npy") for name in values}}
    save(output / "dataset.json", description)
    return values


def load(directory):
    return {name: np.load(directory / f"{name}.npy", mmap_mode="r") for name in ARRAYS}


def input_mean(values, meta):
    ids = np.unique(meta[:, 0])
    return (np.array([values[meta[:, 0] == i].mean() for i in ids], np.float64),
            np.array([meta[meta[:, 0] == i, 2][0] >= 80 for i in ids]), ids)


def interval(values, high):
    values, high = np.asarray(values, np.float64), np.asarray(high, bool)
    assert len(values) and np.isfinite(values).all()
    rng = np.random.default_rng(83002)
    boot = np.zeros(6000)
    for subgroup in (values[~high], values[high]):
        if len(subgroup):
            # 最大2,048入力なので一度の復元抽出でも約100 MBに収まる。
            boot += subgroup[rng.integers(len(subgroup), size=(6000, len(subgroup)))].sum(1) / len(values)
    return {"inputs": len(values), "high_M_inputs": int(high.sum()), "mean": float(values.mean()),
            "ci95": np.quantile(boot, [.025, .975]).tolist(),
            "ci97_5": np.quantile(boot, [.0125, .9875]).tolist()}


def comparison(difference, meta):
    values, high, ids = input_mean(difference, meta)
    result = interval(values, high)
    result["by_M"] = {label: {"inputs": int(m.sum()), "mean": float(values[m].mean()) if m.any() else None}
                      for label, m in (("lt80", ~high), ("ge80", high))}
    result["input_differences"] = [[int(i), float(v)] for i, v in zip(ids, values)]
    return result


def quality(data):
    valid = data["mask"].any(1)
    mask, meta = data["mask"][valid], data["meta"][valid]
    first, future, initial_best = (data[name][valid] for name in ("immediate", "future", "first_best"))
    base, group = data["baseline"][valid], np.arange(int(valid.sum()))
    report = {"groups": len(group), "bootstrap_seed": 83002, "bootstrap_replicates": 6000, "directions": {}}
    for label, select, test in (("primary", slice(0, 4), slice(4, 8)), ("reverse_diagnostic", slice(4, 8), slice(0, 4))):
        train_long, train_now = future[:, :, select].mean(2), first[:, :, select].mean(2)
        selected = np.where(mask, train_long, np.inf).argmin(1)
        immediate = np.where(mask, train_now, np.inf).argmin(1)
        measured = future[:, :, test].mean(2)
        result = {"vs_v079": comparison(measured[group, selected]-measured[group, base], meta),
                  "vs_immediate": comparison(measured[group, selected]-measured[group, immediate], meta),
                  "changed_vs_v079_fraction": float((selected != base).mean()),
                  "changed_vs_immediate_fraction": float((selected != immediate).mean())}
        # 基準候補と初回最良手数の4条件ベクトルが一致する候補に限る診断。
        equal = mask & (initial_best[:, :, select] == initial_best[group, base, select][:, None, :]).all(2)
        cohort = equal.sum(1) >= 2
        chosen_equal = np.where(equal, train_long, np.inf).argmin(1)
        result["same_first_best_as_v079"] = {"groups": int(cohort.sum()), "definition": "first-best vector equals v079 in selection half; at least two candidates"}
        if cohort.any():
            result["same_first_best_as_v079"]["vs_v079"] = comparison((measured[group, chosen_equal]-measured[group, base])[cohort], meta[cohort])
        result["intermediate_diagnostic"] = {}
        for j, steps in enumerate((128, 256, 512)):
            y = data["checkpoints"][valid, :, test, j].mean(2)
            result["intermediate_diagnostic"][str(steps)] = comparison(y[group, selected]-y[group, base], meta)
        report["directions"][label] = result
    primary = report["directions"]["primary"]
    report["gate"] = all(primary[name]["mean"] <= -.10 and primary[name]["ci97_5"][1] < 0
                         for name in ("vs_v079", "vs_immediate"))
    return report


def evaluate_models(run):
    from train_v078_targets import pair_stats
    data = load(run / "data")
    result = {"models": {}, "comparisons": {}}
    choices = {}
    valid = data["mask"].any(1)
    for split, split_mask in (("train", data["meta"][:, 1] == 0), ("validation", data["meta"][:, 1] == 1)):
        ids = np.flatnonzero(valid & split_mask)
        mask, meta = data["mask"][ids], data["meta"][ids]
        group, baseline = np.arange(len(ids)), data["baseline"][ids]
        immediate, future = data["immediate"][ids].mean(2), data["future"][ids].mean(2)
        for name in ("immediate", "future"):
            predictions = np.load(run / "models" / name / "prediction.npy", mmap_mode="r")[ids]
            selected = np.where(mask, predictions, np.inf).argmin(1)
            choices[name] = selected
            result["models"][f"{split}_{name}"] = {
                "groups": len(ids), "future_vs_v079": comparison(future[group, selected]-future[group, baseline], meta),
                "immediate_vs_v079": comparison(immediate[group, selected]-immediate[group, baseline], meta),
                "future_pair": pair_stats(predictions, future, mask), "immediate_pair": pair_stats(predictions, immediate, mask),
                "future_oracle_gap": float((future[group, selected]-np.where(mask, future, np.inf).min(1)).mean())}
        result["comparisons"][split] = comparison(future[group, choices["future"]]-future[group, choices["immediate"]], meta)
    result["gate"] = (result["comparisons"]["validation"]["ci95"][1] < 0 and
                      result["models"]["validation_future"]["future_vs_v079"]["mean"] < 0)
    result["scope"] = "saved candidate choices only; full real-time solver evaluation is a separate experiment"
    return result
