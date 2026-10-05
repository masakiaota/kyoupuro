#!/usr/bin/env python3
"""既存の教師と対応させ、個体特徴と候補の所属だけを追加する。"""
from concurrent.futures import ProcessPoolExecutor, wait, FIRST_COMPLETED
import json
from pathlib import Path
import subprocess
import time

import numpy as np

from v080_data import ROOT, SOURCE, SEED, STEPS, now, save, sha, status, subsets, read_case as read_original_case
from v082_data import probe_input

BASE = ROOT / "results/nn_rank/v080/20261002T113935_studio"
NAME = "n49152_set"
INPUTS, GROUPS, TOKENS, MEMBERS, WIDTH = 65536, 262144, 256, 12, 20
SHARED = ("x", "y", "mask", "ranks", "measures", "meta")
ITEM_FEATURES = ("floor_distance_over_N", "manhattan_over_N", "start_degree_over_4", "home_degree_over_4",
                 "moves_over_T", "travel_over_N", "first_time_over_T", "last_time_over_T", "max_gap_over_T",
                 "mean_copassengers_over_7", "max_copassengers_over_7", "solo_fraction", "same_color_copassenger_fraction",
                 "mixed_flight_fraction", "long_jump_fraction", "support_count_over_T", "supported_passengers_over_T",
                 "min_support_slack_over_8", "mean_lower_height_over_8", "mean_bundle_rank_over_7")


def load(run):
    return {name: np.load(run / "data" / f"{name}.npy", mmap_mode="r")
            for name in (*SHARED, "items", "item_ids", "members")}


def read_case(item, extractor, deadline):
    index, original, complete = read_original_case(item)
    folder = SOURCE / "cases" / f"{index:06d}"
    states = [json.loads(line) for line in (folder / "states.jsonl").read_text().splitlines()]
    rows = [json.loads(line) for line in (folder / "candidates.jsonl").read_text().splitlines()]
    assert len(states) == complete["snapshots"] and len(rows) == complete["rows"]
    remaining = deadline-time.time()
    if remaining <= 0:
        raise TimeoutError("registered_time_limit")
    result = subprocess.run([str(extractor)], input=probe_input(item, states, rows), text=True,
                            capture_output=True, check=True, timeout=min(60, remaining), cwd=ROOT)
    extracted = [json.loads(line) for line in result.stdout.splitlines()]
    assert {s["phase"] for s in extracted} == {s["phase"] for s in states} and len(extracted) == len(states)
    raw = np.zeros((4, TOKENS, WIDTH), np.float32)
    item_ids = np.full((4, TOKENS), -1, np.int16)
    members = np.full((4, 32, MEMBERS), -1, np.int16)
    for group in extracted:
        phase = group["phase"]
        valid = original["mask"][phase]
        assert group["ranks"] == original["ranks"][phase, valid].tolist()
        np.testing.assert_array_equal(np.asarray(group["features"], np.float32), original["x"][phase, valid])
        saved = sorted((r for r in rows if r["phase"] == phase), key=lambda r: r["rank"])
        np.testing.assert_allclose(group["features"], [r["features"] for r in saved], rtol=1e-12, atol=1e-10)
        ids = group["item_ids"]
        assert len(ids) == item["M"] <= TOKENS and ids == sorted(set(ids))
        assert len(group["candidate_ids"]) == len(group["ranks"])
        features = np.asarray(group["item_features"], np.float64)
        assert features.shape == (len(ids), WIDTH) and np.isfinite(features).all()
        raw[phase, :len(ids)] = features
        item_ids[phase, :len(ids)] = ids
        positions = {value: k for k, value in enumerate(ids)}
        state = next(s for s in states if s["phase"] == phase)
        expected = {c["rank"]: c["ids"] for c in state["candidates"]}
        for j, (rank, selected) in enumerate(zip(group["ranks"], group["candidate_ids"])):
            assert sorted(selected) == sorted(expected[rank]) and len(selected) == len(set(selected))
            assert 1 <= len(selected) <= MEMBERS
            members[phase, j, :len(selected)] = [positions[value] for value in sorted(selected)]
    return index, raw, item_ids, members, {k: original[k] for k in SHARED if k != "x"}


def prepare(run, extractor, deadline, limit=None):
    directory = run / "data"
    directory.mkdir(exist_ok=True)
    base = json.loads((BASE / "dataset.json").read_text())
    description_path = run / "dataset.json"
    if description_path.exists():
        description = json.loads(description_path.read_text())
        assert description["extractor_sha256"] == sha(extractor)
        for name, digest in description["array_sha256"].items():
            assert sha(directory / f"{name}.npy") == digest
        return description
    provenance = {"baseline_dataset_sha256": sha(BASE / "dataset.json"), "input_manifest_sha256": sha(SOURCE / "inputs.jsonl"),
                  "extractor_sha256": sha(extractor), "item_features": list(ITEM_FEATURES)}
    if (run / "data_provenance.json").exists():
        assert json.loads((run / "data_provenance.json").read_text()) == provenance
    else:
        for name, digest in base["array_sha256"].items():
            assert sha(BASE / "data" / f"{name}.npy") == digest
        save(run / "data_provenance.json", provenance)
    for name in SHARED:
        path = directory / f"{name}.npy"
        if path.exists():
            assert path.resolve() == (BASE / "data" / path.name).resolve()
        else:
            path.symlink_to(BASE / "data" / path.name)
    shared = {name: np.load(directory / f"{name}.npy", mmap_mode="r") for name in SHARED}
    assert int(shared["meta"][:, 2].max()) <= TOKENS
    specs = {"raw_items": ((GROUPS, TOKENS, WIDTH), np.float32),
             "item_ids": ((GROUPS, TOKENS), np.int16), "members": ((GROUPS, 32, MEMBERS), np.int16),
             "done": ((INPUTS,), bool)}
    arrays = {}
    for name, (shape, dtype) in specs.items():
        path = directory / f"{name}.npy"
        if path.exists():
            array = np.load(path, mmap_mode="r+")
            assert array.shape == shape and array.dtype == dtype
        else:
            array = np.lib.format.open_memmap(path, mode="w+", dtype=dtype, shape=shape)
            if name == "done":
                array[:] = False
                array.flush()
        arrays[name] = array
    items = [json.loads(line) for line in (SOURCE / "inputs.jsonl").read_text().splitlines()]
    assert len(items) == INPUTS and [x["index"] for x in items] == list(range(INPUTS))
    done = arrays["done"]
    stop = INPUTS if limit is None else limit
    iterator = iter(item for item in items[:stop] if not done[item["index"]])
    pending, unflushed = {}, []
    initial, started = int(done.sum()), time.monotonic()

    def flush():
        for name in ("raw_items", "item_ids", "members"):
            arrays[name].flush()
        done[unflushed] = True
        done.flush()
        unflushed.clear()

    def submit(pool):
        item = next(iterator, None)
        if item is not None:
            pending[pool.submit(read_case, item, extractor, deadline)] = item["index"]

    with ProcessPoolExecutor(max_workers=20) as pool:
        for _ in range(40):
            submit(pool)
        try:
            while pending:
                if time.time() >= deadline:
                    raise TimeoutError("registered_time_limit")
                finished, _ = wait(pending, timeout=1, return_when=FIRST_COMPLETED)
                for future in finished:
                    expected = pending.pop(future)
                    index, raw, ids, members, original = future.result()
                    assert index == expected
                    region = slice(index*4, index*4+4)
                    for name, value in original.items():
                        np.testing.assert_array_equal(shared[name][region], value)
                    arrays["raw_items"][region], arrays["item_ids"][region], arrays["members"][region] = raw, ids, members
                    unflushed.append(index)
                    submit(pool)
                if len(unflushed) >= 256:
                    flush()
                    status(run, "extracting_individual_features", completed_inputs=int(done.sum()), total_inputs=INPUTS,
                           inputs_per_second=(int(done.sum())-initial)/max(1e-6, time.monotonic()-started))
        finally:
            flush()
            for future in pending:
                future.cancel()
    elapsed = time.monotonic()-started
    if limit is not None:
        result = {"inputs": int(done.sum()), "new_inputs": int(done.sum())-initial, "elapsed_sec": elapsed,
                  "inputs_per_second": (int(done.sum())-initial)/max(1e-6, elapsed)}
        save(run / "prepare_calibration.json", result)
        return result
    assert done.all()
    assert {k: len(v) for k, v in subsets(shared).items()} == base["groups_by_subset"]
    raw, ids = arrays["raw_items"], arrays["item_ids"]
    training = (shared["meta"][:, 1] == 0) & shared["mask"].any(1)
    count, total = 0, np.zeros(WIDTH, np.float64)
    status(run, "normalizing_individual_features")
    for start in range(0, GROUPS, 1024):
        region = slice(start, start+1024)
        selected = (ids[region] >= 0) & training[region, None]
        samples = raw[region][selected]
        total += samples.sum(0, dtype=np.float64)
        count += len(samples)
    assert count > 0
    precise_mean, squares = total/count, np.zeros(WIDTH, np.float64)
    for start in range(0, GROUPS, 1024):
        if time.time() >= deadline:
            raise TimeoutError("registered_time_limit")
        region = slice(start, start+1024)
        selected = (ids[region] >= 0) & training[region, None]
        deviations = raw[region][selected].astype(np.float64)-precise_mean
        squares += np.square(deviations).sum(0)
    mean, scale = precise_mean.astype(np.float32), np.sqrt(squares/count).astype(np.float32)
    scale[scale < 1e-6] = 1
    path = directory / "items.npy"
    normalized = np.lib.format.open_memmap(path, mode="w+", dtype=np.float32, shape=raw.shape)
    for start in range(0, GROUPS, 1024):
        if time.time() >= deadline:
            normalized.flush()
            raise TimeoutError("registered_time_limit")
        region = slice(start, start+1024)
        value = (raw[region]-mean)/scale
        value[ids[region] < 0] = 0
        assert np.isfinite(value).all()
        normalized[region] = value
    normalized.flush()
    description = {**base, "experiment": "v084", **provenance, "item_mean": mean.tolist(), "item_scale": scale.tolist(),
                   "training_item_observations": count, "tokens": TOKENS, "members": MEMBERS,
                   "original_32_exact_match": True, "source_hashes_checked": True,
                   "array_sha256": {**{name: base["array_sha256"][name] for name in SHARED},
                                    **{name: sha(directory / f"{name}.npy") for name in ("items", "raw_items", "item_ids", "members")}},
                   "finished_at": now()}
    save(description_path, description)
    return description
