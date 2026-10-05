#!/usr/bin/env python3
"""同一局面・正規化・初期値・更新回数で即時教師と長期教師を比較する。"""
import argparse
import json
import math
import os
from pathlib import Path
import signal
import time

import numpy as np
import torch

from train_v077_rank import checkpoint
from train_v078_targets import pair_loss, pair_stats, self_test
from train_v080_scaling import GPUReport, fit_steps, model_new, predict, tensor
from v083_data import load, save, sha, status

SEED, UPDATES = 78001, 11520


def mechanism(run, data):
    result = {"self_test": self_test(torch.device("mps")), "torch": torch.__version__, "tiny": {}}
    raw = np.random.default_rng(SEED).normal(size=(4, 32, 32)).astype(np.float32)
    xx, yy, mm = tensor(raw), tensor(raw[:, :, 10]), tensor(np.ones((4, 32), bool))
    model = model_new((16,), "mps")
    fit_steps(model, xx, yy, mm, 400, .005)
    result["artificial_accuracy"] = pair_stats(predict(model, raw), raw[:, :, 10], np.ones((4, 32), bool))["pair_accuracy"]
    assert result["artificial_accuracy"] >= .99
    # Adamの状態を含め、保存再開後の次の更新を同じ人工バッチで照合する。
    model = model_new((16,), "mps")
    optimizer = torch.optim.Adam(model.parameters(), lr=.001)
    for _ in range(3):
        optimizer.zero_grad(set_to_none=True)
        pair_loss(model(xx).squeeze(-1), yy, mm).backward()
        optimizer.step()
    path = run / "resume_check.pt"
    checkpoint(path, {"model": model.state_dict(), "optimizer": optimizer.state_dict()})
    resumed = model_new((16,), "mps")
    state = torch.load(path, map_location="cpu", weights_only=False)
    resumed.load_state_dict(state["model"])
    resumed_optimizer = torch.optim.Adam(resumed.parameters(), lr=.001)
    resumed_optimizer.load_state_dict(state["optimizer"])
    for net, opt in ((model, optimizer), (resumed, resumed_optimizer)):
        opt.zero_grad(set_to_none=True)
        pair_loss(net(xx).squeeze(-1), yy, mm).backward()
        opt.step()
    np.testing.assert_array_equal(predict(model, raw), predict(resumed, raw))
    result["checkpoint_next_update_exact"] = True
    valid = data["mask"].any(1) & (data["meta"][:, 1] == 0)
    # 両教師が非定数な4局面を入力順で固定する。各M層から2局面。
    informative = valid.copy()
    for name in ("immediate", "future"):
        y = data[name].mean(2)
        informative &= np.where(data["mask"], y, -np.inf).max(1) > np.where(data["mask"], y, np.inf).min(1)
    ids = np.concatenate([np.flatnonzero(informative & ((data["meta"][:, 2] >= 80) == high))[:2] for high in (False, True)])
    assert len(ids) == 4
    for name in ("immediate", "future"):
        model = model_new((16,), "mps")
        x, y, m = data["x"][ids], data[name][ids].mean(2), data["mask"][ids]
        fit_steps(model, tensor(x), tensor(y), tensor(m), 2000, .005)
        measured = pair_stats(predict(model, x), y, m)
        result["tiny"][name] = {"groups": ids.tolist(), **measured}
        assert measured["pair_accuracy"] >= .95, ("tiny_fit", name, measured)
    result["passed"] = True
    save(run / "training_checks.json", result)


def train(run, data, name, deadline):
    directory = run / "models" / name
    directory.mkdir(parents=True, exist_ok=True)
    latest = directory / "latest.pt"
    ids = np.flatnonzero(data["mask"].any(1) & (data["meta"][:, 1] == 0))
    assert len(ids)
    model = model_new((16,), "mps")
    optimizer = torch.optim.Adam(model.parameters(), lr=.001)
    epoch = offset = steps = 0
    history = []
    dataset_sha256 = sha(run / "data/dataset.json")
    initial_path = directory / "initial.pt"
    if not initial_path.exists():
        checkpoint(initial_path, {"model": {k: v.detach().cpu() for k, v in model.state_dict().items()}})
    if latest.exists():
        state = torch.load(latest, map_location="cpu", weights_only=False)
        assert state["dataset_sha256"] == dataset_sha256 and state["target"] == name
        model.load_state_dict(state["model"])
        optimizer.load_state_dict(state["optimizer"])
        epoch, offset, steps, history = (state[k] for k in ("epoch", "offset", "steps", "history"))
    xx, yy, mm = tensor(data["x"][ids]), tensor(data[name][ids].mean(2)), tensor(data["mask"][ids])
    initial, started = steps, time.monotonic()
    interrupted = False

    def stop(*_):
        nonlocal interrupted
        interrupted = True

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)

    def persist():
        checkpoint(latest, {"model": {k: v.detach().cpu() for k, v in model.state_dict().items()},
                           "optimizer": optimizer.state_dict(), "epoch": epoch, "offset": offset, "steps": steps,
                           "history": history, "seed": SEED, "hidden": [16], "target": name,
                           "dataset_sha256": dataset_sha256})

    order = np.random.default_rng(SEED+epoch).permutation(len(ids))
    loss_sum = chunk = 0
    while steps < UPDATES:
        if interrupted or time.time() >= deadline:
            persist()
            raise TimeoutError("registered_time_limit_or_signal")
        pieces, required = [], 256
        while required:
            count = min(required, len(order)-offset)
            pieces.append(order[offset:offset+count])
            offset += count
            required -= count
            if offset == len(order):
                epoch, offset = epoch+1, 0
                order = np.random.default_rng(SEED+epoch).permutation(len(ids))
        batch = tensor(np.concatenate(pieces))
        loss = pair_loss(model(xx[batch]).squeeze(-1), yy[batch], mm[batch])
        value = float(loss.item())
        assert math.isfinite(value)
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        if steps == initial or steps % 1024 == 0:
            assert all(torch.isfinite(p.grad).all().item() for p in model.parameters())
        optimizer.step()
        steps += 1
        chunk += 1
        loss_sum += value
        if steps % 1024 == 0 or steps == UPDATES:
            history.append({"steps": steps, "epoch": epoch, "offset": offset, "all_group_loss": loss_sum/chunk})
            loss_sum = chunk = 0
            persist()
            rate = (steps-initial)/max(1e-9, time.monotonic()-started)
            status(run, "training", condition=name, steps=steps, target_steps=UPDATES, updates_per_second=rate,
                   condition_remaining_seconds=(UPDATES-steps)/rate)
    assert steps == UPDATES
    persist()
    # 検証入力も含む固定間隔の局面でCPUとMPSの数値・選択を照合する。
    check_ids = np.unique(np.concatenate((
        np.arange(0, len(data["x"]), max(1, len(data["x"])//128)),
        np.flatnonzero(data["mask"].any(1) & (data["meta"][:, 1] == 1))[:32])))
    cpu = model_new((16,), "cpu")
    cpu.load_state_dict({k: v.detach().cpu() for k, v in model.state_dict().items()})
    with torch.no_grad():
        reference = cpu(torch.from_numpy(np.array(data["x"][check_ids]))).squeeze(-1).numpy()
    actual = predict(model, data["x"][check_ids])
    np.testing.assert_allclose(actual, reference, rtol=2e-5, atol=2e-4)
    mask = data["mask"][check_ids]
    np.testing.assert_array_equal(np.where(mask, actual, np.inf).argmin(1), np.where(mask, reference, np.inf).argmin(1))
    pred = np.zeros(data["mask"].shape, np.float32)
    for start in range(0, len(pred), 1024):
        if interrupted or time.time() >= deadline:
            raise TimeoutError("registered_time_limit_or_signal")
        pred[start:start+1024] = predict(model, data["x"][start:start+1024])
    assert np.isfinite(pred).all()
    temporary = directory / "prediction.tmp.npy"
    np.save(temporary, pred)
    temporary.replace(directory / "prediction.npy")
    save(directory / "result.json", {"steps": steps, "epoch": epoch, "offset": offset,
         "cpu_mps_passed": True, "max_score_error": float(np.abs(actual-reference).max()),
         "checkpoint_sha256": sha(latest), "prediction_sha256": sha(directory / "prediction.npy")})


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--deadline", type=float, required=True)
    args = parser.parse_args()
    torch.set_num_threads(30)
    torch.set_num_interop_threads(1)
    if not torch.backends.mps.is_available():
        raise RuntimeError("MPS is required")
    torch.mps.set_per_process_memory_fraction(16e9/torch.mps.recommended_max_memory())
    report = GPUReport(args.run / "gpu_state.json")
    try:
        data = load(args.run / "data")
        metadata = json.loads((args.run / "data/dataset.json").read_text())
        for name, digest in metadata["files"].items():
            assert sha(args.run / "data" / f"{name}.npy") == digest
        if not (args.run / "training_checks.json").exists():
            mechanism(args.run, data)
        assert json.loads((args.run / "training_checks.json").read_text())["passed"]
        for name in ("immediate", "future"):
            train(args.run, data, name, args.deadline)
        a, b = [torch.load(args.run / "models" / name / "initial.pt", map_location="cpu", weights_only=False)["model"]
                for name in ("immediate", "future")]
        assert all(torch.equal(a[k], b[k]) for k in a)
    except TimeoutError:
        raise SystemExit(75)
    finally:
        report.close()


if __name__ == "__main__":
    main()
