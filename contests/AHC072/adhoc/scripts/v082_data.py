#!/usr/bin/env python3
"""保存したv077局面へ関係特徴を加える。旧特徴と教師はv080のまま保持する。"""
from concurrent.futures import ProcessPoolExecutor, wait, FIRST_COMPLETED
import json
from pathlib import Path
import subprocess
import time

import numpy as np

from v080_data import ROOT, SOURCE, SEED, STEPS, now, save, sha, status, subsets
from v080_data import read_case as read_original_case

BASE = ROOT / "results/nn_rank/v080/20261002T113935_studio"
NAME = "n49152_h16"
EXTRA_FEATURES = (
    "external_support_flights", "lost_support_times_external_passengers_per_size",
    "external_support_deficit_per_size", "unsupported_external_passengers_per_size",
    "selected_passengers_times_external_support_per_size", "mixed_selected_passengers_per_size",
    "external_copassengers_per_size", "distinct_external_copassengers_over_M",
    "transport_distance_per_size", "first_boarding_time_mean", "last_boarding_time_mean",
    "maximum_boarding_gap", "remaining_support_slack_weighted_mean", "transport_interval_overlap_mean",
)
FEATURES = (
    "N", "K", "M", "floors", "density", "current_T", "best_T", "current_minus_best",
    "progress", "stagnant", "size", "size_over_M", "colors", "max_color_fraction",
    "color_entropy", "removed_commands", "broken_support_jumps", "removed_per_size",
    "broken_per_size", "priority", "nest_distance_mean", "nest_distance_max", "nest_distance_min",
    "pair_distance_mean", "pair_distance_max", "first_time", "last_time", "time_span",
    "flight_fraction", "passengers_per_size", "support_weight", "ride_weight", *EXTRA_FEATURES,
)
INPUTS, GROUPS, WIDTH = 65536, 262144, 46
SHARED = ("y", "mask", "ranks", "measures", "meta")


def load(run, mode="r"):
    return {name: np.load(run / "data" / f"{name}.npy", mmap_mode=mode)
            for name in ("x", *SHARED)}


def probe_input(item, states, rows):
    """C++抽出器へ、試行前の操作列と保存候補だけを渡す。"""
    lines = (SOURCE / item["path"]).read_text().splitlines() + [str(len(states))]
    for state in sorted(states, key=lambda r: r["phase"]):
        group = sorted((r for r in rows if r["phase"] == state["phase"]), key=lambda r: r["rank"])
        candidates = {c["rank"]: c for c in state["candidates"]}
        sw, rw = group[0]["features"][30:32]
        assert int(sw) == sw and int(rw) == rw
        assert all(r["features"][30:32] == [sw, rw] for r in group)
        lines.append(f"{state['phase']} {state['stagnant']} {state['progress']:.17g} {int(sw)} {int(rw)}")
        for key in ("current", "best"):
            lines.append(str(len(state[key])))
            lines.extend(" ".join(map(str, move)) for move in state[key])
        lines.append(str(len(group)))
        for row in group:
            candidate = candidates[row["rank"]]
            assert candidate["hash"] == row["candidate_hash"]
            lines.append(f"{row['rank']} {candidate['hash']} {row['features'][19]:.17g} "
                         f"{len(candidate['ids'])} " + " ".join(map(str, candidate["ids"])))
    return "\n".join(lines) + "\n"


def read_case(item, extractor, deadline):
    # v080と同じ検査を通し、入力・phase・rank・hash・教師の対応も維持する。
    index, original, complete = read_original_case(item)
    folder = SOURCE / "cases" / f"{index:06d}"
    states = [json.loads(line) for line in (folder / "states.jsonl").read_text().splitlines()]
    rows = [json.loads(line) for line in (folder / "candidates.jsonl").read_text().splitlines()]
    assert len(states) == complete["snapshots"] and len(rows) == complete["rows"]
    remaining = deadline - time.time()
    if remaining <= 0:
        raise TimeoutError("registered_time_limit")
    result = subprocess.run([str(extractor)], input=probe_input(item, states, rows), text=True,
                            capture_output=True, check=True, timeout=min(60, remaining), cwd=ROOT)
    extracted = [json.loads(line) for line in result.stdout.splitlines()]
    assert len(extracted) == len(states) and len({r["phase"] for r in extracted}) == len(states)
    assert {r["phase"] for r in extracted} == {r["phase"] for r in states}
    extra = np.zeros((4, 32, len(EXTRA_FEATURES)), np.float64)
    for group in extracted:
        phase = group["phase"]
        valid = original["mask"][phase]
        assert group["ranks"] == original["ranks"][phase, valid].tolist()
        raw = np.asarray(group["features"], dtype=np.float64)
        assert raw.shape == (int(valid.sum()), WIDTH) and np.isfinite(raw).all()
        # 元の算術結果を、学習が実際に使うfloat32でも完全照合する。
        np.testing.assert_array_equal(raw[:, :32].astype(np.float32), original["x"][phase, valid])
        saved_raw = np.array([r["features"] for r in sorted(
            (r for r in rows if r["phase"] == phase), key=lambda r: r["rank"])], np.float64)
        np.testing.assert_allclose(raw[:, :32], saved_raw, rtol=1e-12, atol=1e-10)
        extra[phase, valid] = raw[:, 32:]
    return index, extra, {k: original[k] for k in SHARED}


def checked_provenance(run, extractor):
    base = json.loads((BASE / "dataset.json").read_text())
    assert tuple(base["features"]) == FEATURES[:32]
    path = run / "data_provenance.json"
    expected = {"baseline": str(BASE), "baseline_dataset_sha256": sha(BASE / "dataset.json"),
                "input_manifest_sha256": sha(SOURCE / "inputs.jsonl"),
                "extractor_sha256": sha(extractor), "features": FEATURES,
                "baseline_array_sha256": base["array_sha256"]}
    # JSONへ保存されたtupleはlistになるため、比較形式をそろえる。
    expected = json.loads(json.dumps(expected))
    if path.exists():
        assert json.loads(path.read_text()) == expected
    else:
        for name, digest in base["array_sha256"].items():
            assert sha(BASE / "data" / f"{name}.npy") == digest
        save(path, expected)
    return base


def prepare(run, extractor, deadline, workers=20, limit=None):
    """limitで校正した入力の完了印を、その後の全件処理で再利用する。"""
    run, extractor = Path(run), Path(extractor).resolve()
    assert 1 <= workers <= 20 and (limit is None or 0 < limit <= INPUTS)
    directory = run / "data"
    directory.mkdir(parents=True, exist_ok=True)
    base = checked_provenance(run, extractor)
    for name in SHARED:
        target, source = directory / f"{name}.npy", BASE / "data" / f"{name}.npy"
        if target.exists():
            assert target.resolve() == source.resolve()
        else:
            target.symlink_to(source)
    if (run / "dataset.json").exists():
        description = json.loads((run / "dataset.json").read_text())
        for name, digest in description["array_sha256"].items():
            assert sha(directory / f"{name}.npy") == digest
        assert sha(directory / "extra_raw.npy") == description["extra_raw_sha256"]
        return description
    items = [json.loads(line) for line in (SOURCE / "inputs.jsonl").read_text().splitlines()]
    assert len(items) == INPUTS and [r["index"] for r in items] == list(range(INPUTS))
    assert sum(r["role"] == "train" for r in items) == 49152
    # 生値は独立照合の1e-10精度を守る。学習と正規化は下でfloat32へ丸める。
    specs = {"extra_raw": ((GROUPS, 32, len(EXTRA_FEATURES)), np.float64), "done": ((INPUTS,), bool)}
    arrays = {}
    for name, (shape, dtype) in specs.items():
        path = directory / f"{name}.npy"
        if path.exists():
            arrays[name] = np.load(path, mmap_mode="r+")
            assert arrays[name].shape == shape and arrays[name].dtype == dtype
        else:
            arrays[name] = np.lib.format.open_memmap(path, mode="w+", dtype=dtype, shape=shape)
            if name == "done":
                arrays[name][:] = False
                arrays[name].flush()
    raw, done = arrays["extra_raw"], arrays["done"]
    shared = {name: np.load(directory / f"{name}.npy", mmap_mode="r") for name in SHARED}
    stop = INPUTS if limit is None else limit
    pending, unflushed = {}, []
    remaining = iter(item for item in items[:stop] if not done[item["index"]])
    started, initial = time.monotonic(), int(done.sum())

    def flush():
        raw.flush()
        done[unflushed] = True
        done.flush()
        unflushed.clear()

    def submit(pool):
        item = next(remaining, None)
        if item is not None:
            pending[pool.submit(read_case, item, extractor, deadline)] = item["index"]

    with ProcessPoolExecutor(max_workers=workers) as pool:
        for _ in range(workers * 2):
            submit(pool)
        try:
            while pending:
                if time.time() >= deadline:
                    raise TimeoutError("registered_time_limit")
                ready, _ = wait(pending, timeout=1, return_when=FIRST_COMPLETED)
                for future in ready:
                    index = pending.pop(future)
                    actual_index, extra, original = future.result()
                    assert index == actual_index
                    region = slice(index * 4, index * 4 + 4)
                    for name in SHARED:
                        np.testing.assert_array_equal(shared[name][region], original[name])
                    raw[region] = extra
                    unflushed.append(index)
                    submit(pool)
                if len(unflushed) >= 256:
                    flush()
                    status(run, "preparing_relation_features", completed_inputs=int(done.sum()), total_inputs=INPUTS,
                           inputs_per_second=(int(done.sum()) - initial) / max(time.monotonic() - started, 1e-6))
        finally:
            flush()
            for future in pending:
                future.cancel()
    elapsed = time.monotonic() - started
    if limit is not None:
        partial = {"inputs": int(done.sum()), "new_inputs": int(done.sum()) - initial,
                   "newly_processed": int(done.sum()) - initial, "elapsed_sec": elapsed,
                   "inputs_per_second": (int(done.sum()) - initial) / max(elapsed, 1e-6),
                   "seconds": elapsed, "limit": limit, "finished_at": now()}
        save(run / "prepare_partial.json", partial)
        return partial
    assert done.all()
    sets = subsets(shared)
    for name, count in (("train49152", 49152), ("primary", 2048), ("additional", 14336)):
        assert len(np.unique(shared["meta"][sets[name], 0])) == count
    assert {k: len(v) for k, v in sets.items()} == base["groups_by_subset"]
    training = (shared["meta"][:, 1] == 0) & (shared["mask"].sum(1) >= 2)
    total, count = np.zeros(len(EXTRA_FEATURES), np.float64), 0
    for start in range(0, GROUPS, 1024):
        if time.time() >= deadline:
            raise TimeoutError("registered_time_limit")
        region = slice(start, start + 1024)
        selected = shared["mask"][region] & training[region, None]
        samples = np.asarray(raw[region], dtype=np.float32)[selected]
        assert np.isfinite(samples).all()
        total += samples.sum(0, dtype=np.float64)
        count += len(samples)
    assert count > 0
    precise_mean = total / count
    squares = np.zeros(len(EXTRA_FEATURES), np.float64)
    for start in range(0, GROUPS, 1024):
        if time.time() >= deadline:
            raise TimeoutError("registered_time_limit")
        region = slice(start, start + 1024)
        selected = shared["mask"][region] & training[region, None]
        deviations = np.asarray(raw[region], dtype=np.float32)[selected].astype(np.float64) - precise_mean
        squares += np.square(deviations).sum(0)
    mean, scale = precise_mean.astype(np.float32), np.sqrt(squares / count).astype(np.float32)
    scale[scale < 1e-6] = 1
    path = directory / "x.npy"
    x = np.lib.format.open_memmap(path, mode="w+", dtype=np.float32, shape=(GROUPS, 32, WIDTH))
    old_x = np.load(BASE / "data/x.npy", mmap_mode="r")
    for start in range(0, GROUPS, 1024):
        if time.time() >= deadline:
            x.flush()
            raise TimeoutError("registered_time_limit")
        region = slice(start, start + 1024)
        # 生配列を保持し、正規化途中の再開でも二重に正規化しない。
        x[region, :, :32] = old_x[region]
        extra = (np.asarray(raw[region], dtype=np.float32) - mean) / scale
        extra[~shared["mask"][region]] = 0
        assert np.isfinite(extra).all()
        x[region, :, 32:] = extra
        np.testing.assert_array_equal(x[region, :, :32], old_x[region])
    x.flush()
    description = {**base, "experiment": "v082", "baseline": str(BASE), "features": list(FEATURES),
                   "mean": base["mean"] + mean.tolist(), "scale": base["scale"] + scale.tolist(),
                   "extra_features": list(EXTRA_FEATURES), "extra_normalization_train_candidates": count,
                   "original_32_exact_match": True, "source_hashes_checked": True,
                   "array_sha256": {"x": sha(path), **{k: base["array_sha256"][k] for k in SHARED}},
                   "extra_raw_sha256": sha(directory / "extra_raw.npy"), "finished_at": now()}
    save(run / "dataset.json", description)
    return description
