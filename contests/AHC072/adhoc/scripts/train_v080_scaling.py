#!/usr/bin/env python3
"""固定した更新数でv080の1条件を学習するGPU子プロセス。"""
import argparse
import json
import math
import os
from pathlib import Path
import signal
import threading
import time

import numpy as np
import torch
from torch import nn

from train_v077_rank import checkpoint
from train_v078_targets import pair_loss, pair_stats, self_test
from v080_data import CONDITIONS, PREVIOUS, SEED, load, now, save, sha, status, subsets


def model_new(hidden, device):
    torch.manual_seed(SEED)
    if str(device) == "mps":
        torch.mps.manual_seed(SEED)
    sizes, layers = (32, *hidden, 1), []
    for i, (a, b) in enumerate(zip(sizes, sizes[1:])):
        layers.append(nn.Linear(a, b))
        if i < len(sizes) - 2:
            layers.append(nn.ReLU())
    return nn.Sequential(*layers).to(device)


class GPUReport:
    def __init__(self, path):
        self.path, self.stop = path, threading.Event()
        self.thread = threading.Thread(target=self.loop, daemon=True)
        self.write()
        self.thread.start()

    def write(self):
        save(self.path, {"active": True, "pid": os.getpid(), "updated_unix": time.time(),
                         "driver_bytes": torch.mps.driver_allocated_memory(),
                         "tensor_bytes": torch.mps.current_allocated_memory()})

    def loop(self):
        while not self.stop.wait(1):
            self.write()

    def close(self):
        self.stop.set()
        self.thread.join()
        save(self.path, {"active": False, "pid": os.getpid(), "updated_unix": time.time(), "driver_bytes": 0})


def tensor(a):
    return torch.from_numpy(np.array(a, copy=True)).to("mps")


def fit_steps(model, x, y, mask, steps, lr):
    opt = torch.optim.Adam(model.parameters(), lr=lr)
    for _ in range(steps):
        loss = pair_loss(model(x).squeeze(-1), y, mask)
        opt.zero_grad(set_to_none=True)
        loss.backward()
        opt.step()
    torch.mps.synchronize()


def predict(model, x):
    with torch.no_grad():
        return model(tensor(x)).squeeze(-1).cpu().numpy()


def calibration(run):
    check = self_test(torch.device("mps"))
    result = {"self_test": check, "torch": torch.__version__, "numpy": np.__version__,
              "recommended_max_memory": torch.mps.recommended_max_memory(), "architectures": {}}
    random = np.random.default_rng(SEED)
    raw = random.normal(size=(256, 32, 32)).astype(np.float32)
    x, y, mask = tensor(raw), tensor(np.floor(raw[:, :, 10])), tensor(np.ones((256, 32), bool))
    for hidden in ((16,), (64, 32)):
        model = model_new(hidden, "mps")
        opt = torch.optim.Adam(model.parameters(), lr=.001)
        original = [p.detach().cpu().clone() for p in model.parameters()]
        # 本学習と同じ同期点を置く。人工データだけで処理時間を見積もる。
        for step in range(300):
            if step == 100:
                started = time.monotonic()
            loss = pair_loss(model(x).squeeze(-1), y, mask)
            assert math.isfinite(loss.item())
            opt.zero_grad(set_to_none=True)
            loss.backward()
            if step == 0:
                assert all(torch.isfinite(p.grad).all().item() for p in model.parameters())
            opt.step()
        torch.mps.synchronize()
        seconds_per_update = (time.monotonic() - started) / 200
        assert any(not torch.equal(a, b.detach().cpu()) for a, b in zip(original, model.parameters()))
        cpu = model_new(hidden, "cpu")
        cpu.load_state_dict({k: v.detach().cpu() for k, v in model.state_dict().items()})
        with torch.no_grad():
            ref = cpu(torch.from_numpy(raw[:4])).squeeze(-1).numpy()
        np.testing.assert_allclose(predict(model, raw[:4]), ref, rtol=2e-5, atol=2e-4)
        # 保存再開後の次のAdam更新が、中断しない更新と一致することを人工データで確認する。
        path = run / ("calibration_" + "_".join(map(str, hidden)) + ".pt")
        checkpoint(path, {"model": model.state_dict(), "optimizer": opt.state_dict()})
        resumed = model_new(hidden, "mps")
        loaded = torch.load(path, map_location="cpu", weights_only=False)
        resumed.load_state_dict(loaded["model"])
        resumed_opt = torch.optim.Adam(resumed.parameters(), lr=.001)
        resumed_opt.load_state_dict(loaded["optimizer"])
        for net, optimizer in ((model, opt), (resumed, resumed_opt)):
            loss = pair_loss(net(x).squeeze(-1), y, mask)
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            optimizer.step()
        np.testing.assert_array_equal(predict(model, raw[:4]), predict(resumed, raw[:4]))
        control = model_new(hidden, "mps")
        fit_steps(control, x[:4], x[:4, :, 10], mask[:4], 400, .005)
        accuracy = pair_stats(predict(control, raw[:4]), raw[:4, :, 10], np.ones((4, 32), bool))["pair_accuracy"]
        assert accuracy >= .99, (hidden, "known feature control", accuracy)
        result["architectures"]["_".join(map(str, hidden))] = {
            "parameters": sum(p.numel() for p in model.parameters()), "seconds_per_update": seconds_per_update,
            "cpu_mps_match": True, "checkpoint_next_update_exact": True, "control_pair_accuracy": accuracy}
    save(run / "gpu_calibration.json", result)


def tiny(run):
    data = load(run)
    description = json.loads((run / "dataset.json").read_text())
    ids = np.array([r[0] * 4 + r[3] for r in description["tiny_groups"]])
    x, y, mask = tensor(data["x"][ids]), tensor(data["y"][ids]), tensor(data["mask"][ids])
    result = {}
    for hidden in ((16,), (64, 32)):
        model = model_new(hidden, "mps")
        fit_steps(model, x, y, mask, 2000, .005)
        measured = pair_stats(predict(model, data["x"][ids]), data["y"][ids], data["mask"][ids])
        result["_".join(map(str, hidden))] = measured
        save(run / "tiny.json", result)
        assert measured["pair_accuracy"] >= .95, (hidden, "tiny fit below registered threshold", measured)


def train(run, name, target_steps, deadline, loss_function=pair_loss):
    data = load(run)
    ids = subsets(data)["train" + str(CONDITIONS[name][0])]
    hidden = CONDITIONS[name][1]
    model = model_new(hidden, "mps")
    opt = torch.optim.Adam(model.parameters(), lr=.001)
    directory = run / "models" / name
    directory.mkdir(parents=True, exist_ok=True)
    latest = directory / "latest.pt"
    epoch = offset = steps = 0
    history = []
    if latest.exists():
        saved = torch.load(latest, map_location="cpu", weights_only=False)
    elif name == "n6144_h16":
        original = PREVIOUS / "models/immediate/latest.pt"
        assert sha(original) == "93a9a319564a93161c874fdd08838c0f98441824b2c0793d2c64270a4ebb61fa"
        saved = torch.load(original, map_location="cpu", weights_only=False)
        assert saved["epoch"] == 120 and saved["offset"] == 0 and saved["steps"] == 11520
    else:
        saved = None
    if saved is not None:
        model.load_state_dict(saved["model"])
        opt.load_state_dict(saved["optimizer"])
        epoch, offset, steps = saved["epoch"], saved["offset"], saved["steps"]
        history = saved["history"]
    xx, yy, mm = tensor(data["x"][ids]), tensor(data["y"][ids]), tensor(data["mask"][ids])
    initial = {k: v.detach().cpu().clone() for k, v in model.state_dict().items()}
    initial_steps, started = steps, time.monotonic()
    interrupted = False

    def stop(signum, frame):
        nonlocal interrupted
        interrupted = True

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)

    def persist(path=latest):
        checkpoint(path, {"model": {k: v.detach().cpu() for k, v in model.state_dict().items()},
                          "optimizer": opt.state_dict(), "epoch": epoch, "offset": offset, "steps": steps,
                          "history": history, "seed": SEED, "hidden": hidden,
                          "torch_rng": torch.get_rng_state(), "mps_rng": torch.mps.get_rng_state()})

    order = np.random.default_rng(SEED + epoch).permutation(len(ids))
    chunk_loss = chunk_steps = 0
    while steps < target_steps:
        if interrupted or time.time() >= deadline:
            persist()
            raise TimeoutError("registered_time_limit_or_signal")
        pieces, required = [], 256
        while required:
            count = min(required, len(order) - offset)
            pieces.append(order[offset:offset + count])
            offset += count
            required -= count
            if offset == len(order):
                epoch, offset = epoch + 1, 0
                order = np.random.default_rng(SEED + epoch).permutation(len(ids))
        batch = tensor(np.concatenate(pieces))
        loss = loss_function(model(xx[batch]).squeeze(-1), yy[batch], mm[batch])
        value = float(loss.item())
        assert math.isfinite(value)
        opt.zero_grad(set_to_none=True)
        loss.backward()
        if steps == initial_steps or steps % 1024 == 0:
            assert all(torch.isfinite(p.grad).all().item() for p in model.parameters())
        opt.step()
        steps += 1
        chunk_loss += value
        chunk_steps += 1
        if steps % 1024 == 0 or steps == target_steps:
            history.append({"steps": steps, "epoch": epoch, "offset": offset,
                            "all_group_training_loss": chunk_loss / chunk_steps})
            chunk_loss = chunk_steps = 0
            persist()
            rate = (steps - initial_steps) / (time.monotonic() - started)
            status(run, "training", condition=name, steps=steps, target_steps=target_steps,
                   epoch=epoch, updates_per_second=rate, condition_remaining_seconds=(target_steps - steps) / rate)
    assert steps == target_steps
    if steps > initial_steps:
        assert any(not torch.equal(v.detach().cpu(), initial[k]) for k, v in model.state_dict().items())
    milestone = directory / f"step_{steps}.pt"
    persist(milestone)
    persist()
    reloaded = model_new(hidden, "mps")
    reloaded.load_state_dict(torch.load(milestone, map_location="cpu", weights_only=False)["model"])
    np.testing.assert_array_equal(predict(model, data["x"][ids[:4]]), predict(reloaded, data["x"][ids[:4]]))
    cpu = model_new(hidden, "cpu")
    cpu.load_state_dict({k: v.detach().cpu() for k, v in model.state_dict().items()})
    # 全分割から固定間隔でCPU/MPSを照合する。最終的なC++参照はCPUを使う。
    check_ids = np.arange(0, len(data["x"]), 1024)
    with torch.no_grad():
        ref = cpu(torch.from_numpy(np.array(data["x"][check_ids]))).squeeze(-1).numpy()
    gpu = predict(model, data["x"][check_ids])
    np.testing.assert_allclose(gpu, ref, rtol=2e-5, atol=2e-4)
    mask = data["mask"][check_ids]
    np.testing.assert_array_equal(np.where(mask, gpu, np.inf).argmin(1), np.where(mask, ref, np.inf).argmin(1))
    output = directory / f"prediction_{steps}.npy"
    temp = output.with_suffix(".tmp.npy")
    predictions = np.lib.format.open_memmap(temp, mode="w+", dtype=np.float32, shape=data["mask"].shape)
    status(run, "predicting_saved_groups", condition=name, steps=steps)
    for s in range(0, len(data["x"]), 1024):
        if interrupted or time.time() >= deadline:
            raise TimeoutError("registered_time_limit_or_signal")
        predictions[s:s + 1024] = predict(model, data["x"][s:s + 1024])
    assert np.isfinite(predictions).all()
    predictions.flush()
    del predictions
    temp.replace(output)
    save(directory / f"finished_{steps}.json", {"finished_at": now(), "steps": steps,
         "checkpoint_reload_exact": True, "cpu_mps_match": True, "checked_groups": len(check_ids),
         "max_cpu_mps_error": float(np.max(np.abs(ref - gpu))), "checkpoint_sha256": sha(milestone),
         "prediction_sha256": sha(output), "elapsed_sec": time.monotonic() - started})


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--mode", choices=("calibrate", "tiny", "train"), required=True)
    parser.add_argument("--condition", choices=CONDITIONS)
    parser.add_argument("--steps", type=int)
    parser.add_argument("--deadline", type=float, required=True)
    args = parser.parse_args()
    torch.set_num_threads(30)
    torch.set_num_interop_threads(1)
    if not torch.backends.mps.is_available():
        raise RuntimeError("MPS required; no automatic CPU substitution")
    torch.mps.set_per_process_memory_fraction(16e9 / torch.mps.recommended_max_memory())
    report = GPUReport(args.run / "gpu_state.json")
    try:
        if args.mode == "calibrate":
            calibration(args.run)
        elif args.mode == "tiny":
            tiny(args.run)
        else:
            train(args.run, args.condition, args.steps, args.deadline)
    except TimeoutError:
        raise SystemExit(75)
    finally:
        report.close()


if __name__ == "__main__":
    main()
