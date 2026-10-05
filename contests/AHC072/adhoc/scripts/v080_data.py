#!/usr/bin/env python3
"""v080の固定条件と、保存教師を入力単位で検査する読み込み処理。"""
from concurrent.futures import ProcessPoolExecutor, wait, FIRST_COMPLETED
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import time

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "results/nn_rank/v077/20261002T021222_studio"
PREVIOUS = ROOT / "results/nn_rank/v078/20261002T095446_studio"
PROBES = ROOT / "results/nn_rank/v079/20261002T104039_studio"
SEED = 78001
CONDITIONS = {"n6144_h16": (6144, (16,)), "n49152_h16": (49152, (16,)),
              "n6144_h64_32": (6144, (64, 32)), "n49152_h64_32": (49152, (64, 32))}
STEPS = (11520, 92160)


def now():
    return datetime.now(timezone.utc).isoformat()


def save(path, value):
    path = Path(path)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    temporary.replace(path)


def sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def status(run, stage, **fields):
    save(run / "status.json", {"stage": stage, "updated_at": now(), **fields})


def read_case(item):
    """1入力だけを展開する。返す配列は4局面分に限り、メモリを入力数に比例させない。"""
    folder = SOURCE / "cases" / f"{item['index']:06d}"
    complete = json.loads((folder / "complete.json").read_text())
    assert json.loads((folder / "input.json").read_text()) == item
    assert sha(SOURCE / item["path"]) == item["sha256"]
    raw, states_raw = (folder / "candidates.jsonl").read_bytes(), (folder / "states.jsonl").read_bytes()
    assert hashlib.sha256(raw).hexdigest() == complete["candidates_sha256"]
    assert hashlib.sha256(states_raw).hexdigest() == complete["states_sha256"]
    rows = [json.loads(line) for line in raw.splitlines()]
    states_list = [json.loads(line) for line in states_raw.splitlines()]
    states = {r["phase"]: r for r in states_list}
    assert len(rows) == complete["rows"] and len(states) == len(states_list) == complete["snapshots"]
    assert 1 <= len(states) <= 4 and all(0 <= p < 4 for p in states)
    x = np.zeros((4, 32, 32), np.float32)
    y = np.zeros((4, 32), np.float32)
    mask = np.zeros((4, 32), bool)
    # 候補の費用、短縮、有効変更。v078のmeasuresの対応列と照合する。
    measures = np.zeros((4, 32, 3), np.float32)
    ranks = np.full((4, 32), 100000, np.int32)
    meta = np.array([(item["index"], int(item["role"] == "validation"), item["M"], p)
                     for p in range(4)], np.int32)
    allowed = {"accepted", "extract_failed", "insert_failed", "duplicate", "rejected"}
    assert all(r["phase"] in states for r in rows)
    for p, state in states.items():
        group = sorted((r for r in rows if r["phase"] == p), key=lambda r: r["rank"])
        candidates = {c["rank"]: c for c in state["candidates"]}
        assert 1 <= len(group) <= 32 and len(group) == len(candidates)
        assert len({r["rank"] for r in group}) == len(group)
        assert group[0]["rank"] == state["baseline_rank"]
        for j, row in enumerate(group):
            f = row["features"]
            assert len(f) == 32 and np.isfinite(f).all() and f[2] == item["M"]
            assert row["candidate_hash"] == candidates[row["rank"]]["hash"]
            assert row["snapshot_hash"] == state["snapshot_hash"] and row["rng"] == state["rng"]
            assert row["first_completed"] and row["first_outcome"] in allowed
            assert row["pool_free"] == 4 and row["E"] == 0 and row["errors"] == 0
            assert row["best_T"] <= row["first_best_T"] <= f[6]
            assert row["first_best_T"] <= row["first_T"]
            assert f[5] == len(state["current"]) and f[6] == len(state["best"])
            assert row["first_elapsed"] > 0 and math.isfinite(row["first_elapsed"])
            accepted = row["first_outcome"] == "accepted"
            if not accepted:
                assert row["first_T"] == f[5] and row["first_best_T"] == f[6]
            x[p, j], y[p, j], mask[p, j], ranks[p, j] = f, row["first_T"], True, row["rank"]
            measures[p, j] = (row["first_elapsed"] * 1000,
                               accepted and row["first_T"] < f[5],
                               accepted and row["first_T"] <= f[5])
    return item["index"], dict(x=x, y=y, mask=mask, ranks=ranks, measures=measures, meta=meta), complete


def load(run, mode="r"):
    return {name: np.load(run / "data" / f"{name}.npy", mmap_mode=mode)
            for name in ("x", "y", "mask", "ranks", "measures", "meta")}


def subsets(data):
    valid = data["mask"].sum(1) >= 2
    meta = data["meta"]
    small = meta[:, 0] // 8 % 8 == 1
    train = meta[:, 1] == 0
    return {"train6144": np.flatnonzero(valid & train & small),
            "train49152": np.flatnonzero(valid & train),
            "primary": np.flatnonzero(valid & ~train & small),
            "additional": np.flatnonzero(valid & ~train & ~small)}


def prepare(run, deadline, limit=None):
    directory = run / "data"
    directory.mkdir(exist_ok=True)
    items = [json.loads(line) for line in (SOURCE / "inputs.jsonl").read_text().splitlines()]
    assert len(items) == 65536 and [r["index"] for r in items] == list(range(65536))
    assert len({r["sha256"] for r in items}) == len(items)
    assert sum(r["role"] == "train" for r in items) == 49152
    previous = json.loads((PREVIOUS / "dataset.json").read_text())
    mean, scale = np.array(previous["mean"], np.float32), np.array(previous["scale"], np.float32)
    specs = {"x": ((262144, 32, 32), np.float32), "y": ((262144, 32), np.float32),
             "mask": ((262144, 32), bool), "ranks": ((262144, 32), np.int32),
             "measures": ((262144, 32, 3), np.float32), "meta": ((262144, 4), np.int32),
             "done": ((65536,), bool)}
    arrays = {}
    for name, (shape, dtype) in specs.items():
        path = directory / f"{name}.npy"
        if path.exists():
            arrays[name] = np.load(path, mmap_mode="r+")
            assert arrays[name].shape == shape and arrays[name].dtype == dtype
        else:
            arrays[name] = np.lib.format.open_memmap(path, mode="w+", dtype=dtype, shape=shape)
    done = arrays.pop("done")
    stop = len(items) if limit is None else limit
    remaining = iter(r for r in items[:stop] if not done[r["index"]])
    pending, unflushed = {}, []
    started, initial = time.monotonic(), int(done.sum())

    def flush():
        # 配列の永続化を先に済ませ、完了印だけが先に残る中断を避ける。
        for a in arrays.values():
            a.flush()
        done[unflushed] = True
        done.flush()
        unflushed.clear()

    def submit(pool):
        item = next(remaining, None)
        if item is not None:
            pending[pool.submit(read_case, item)] = item["index"]

    with ProcessPoolExecutor(max_workers=16) as pool:
        for _ in range(32):
            submit(pool)
        try:
            while pending:
                if time.time() >= deadline:
                    raise TimeoutError("registered_time_limit")
                ready, _ = wait(pending, timeout=1, return_when=FIRST_COMPLETED)
                for future in ready:
                    index = pending.pop(future)
                    i, result, _ = future.result()
                    assert i == index
                    result["x"] = (result["x"] - mean) / scale
                    result["x"][~result["mask"]] = 0
                    for name, a in arrays.items():
                        a[index * 4:index * 4 + 4] = result[name]
                    unflushed.append(index)
                    submit(pool)
                if len(unflushed) >= 256:
                    flush()
                    status(run, "preparing_data", completed_inputs=int(done.sum()), total_inputs=65536,
                           inputs_per_second=(int(done.sum()) - initial) / (time.monotonic() - started))
        finally:
            flush()
            for future in pending:
                future.cancel()
    rate = (int(done.sum()) - initial) / max(time.monotonic() - started, 1e-6)
    if limit is not None:
        return {"inputs": int(done.sum()), "inputs_per_second": rate, "seconds": time.monotonic() - started}
    assert done.all()
    sets = subsets(arrays)
    assert len(np.unique(arrays["meta"][sets["train6144"], 0])) == 6144
    assert len(np.unique(arrays["meta"][sets["train49152"], 0])) == 49152
    assert len(np.unique(arrays["meta"][sets["primary"], 0])) == 2048
    assert len(np.unique(arrays["meta"][sets["additional"], 0])) == 14336
    # 旧モデルの継続に必要な入力順・正規化・教師を全件照合する。
    assert sha(PREVIOUS / "dataset.npz") == previous["cache_sha256"]
    with np.load(PREVIOUS / "dataset.npz") as z:
        ids = z["meta"][:, 0] * 4 + z["meta"][:, 3]
        for name in ("x", "mask", "ranks", "meta"):
            np.testing.assert_array_equal(arrays[name][ids], z[name])
        np.testing.assert_array_equal(arrays["y"][ids], z["targets"][1])
        np.testing.assert_array_equal(arrays["measures"][ids], z["measures"][:, :, [3, 5, 4]])
    description = {"source": str(SOURCE), "manifest_sha256": sha(SOURCE / "inputs.jsonl"),
                   "mean": mean.tolist(), "scale": scale.tolist(), "features": previous["features"],
                   "tiny_groups": previous["tiny_groups"], "v078_exact_match": True,
                   "groups_by_subset": {k: len(v) for k, v in sets.items()},
                   "missing_groups": arrays["meta"][arrays["mask"].sum(1) < 2].tolist(),
                   "array_sha256": {k: sha(directory / f"{k}.npy") for k in arrays}, "finished_at": now()}
    save(run / "dataset.json", description)
    return description
