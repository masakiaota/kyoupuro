#!/usr/bin/env python3
"""追加14特徴を使う46→16→1モデルを、v080と同じ更新順で学習する。"""
import argparse
import json
import math
from pathlib import Path
import signal
import time

import numpy as np
import torch
from torch import nn

from train_v077_rank import checkpoint
from train_v078_targets import pair_loss, pair_stats
from train_v080_scaling import GPUReport, tensor, predict
from train_v080_scaling import model_new as original_model_new
from v082_data import NAME, WIDTH, SEED, STEPS, load, now, save, sha, status, subsets

HIDDEN = (16,)


def model_new(device):
    # 入力数を変えた通常の初期化は後段の乱数まで変わるため、32入力版から全層を移す。
    original = original_model_new(HIDDEN, "cpu")
    expanded = nn.Sequential(nn.Linear(WIDTH, 16), nn.ReLU(), nn.Linear(16, 1))
    with torch.no_grad():
        expanded[0].weight[:, :32].copy_(original[0].weight)
        expanded[0].weight[:, 32:].zero_()
        expanded[0].bias.copy_(original[0].bias)
        expanded[2].weight.copy_(original[2].weight)
        expanded[2].bias.copy_(original[2].bias)
    if str(device) == "mps":
        torch.mps.manual_seed(SEED)
    return expanded.to(device)


def check_time(deadline):
    if time.time() >= deadline:
        raise TimeoutError("registered_time_limit_or_signal")


def array_predict(model, x, device):
    with torch.no_grad():
        return model(torch.from_numpy(np.array(x, copy=True)).to(device)).squeeze(-1).cpu().numpy()


def initial_check(device):
    old = original_model_new(HIDDEN, "cpu")
    new = model_new(device)
    state = {k: v.detach().cpu() for k, v in new.state_dict().items()}
    assert torch.equal(state["0.weight"][:, :32], old[0].weight)
    assert not state["0.weight"][:, 32:].any()
    for key in ("0.bias", "2.weight", "2.bias"):
        assert torch.equal(state[key], old.state_dict()[key])
    assert sum(p.numel() for p in new.parameters()) == 769
    raw = np.random.default_rng(SEED).normal(size=(4, 32, WIDTH)).astype(np.float32)
    reference = array_predict(old, raw[:, :, :32], "cpu")
    actual = array_predict(new, raw, device)
    np.testing.assert_allclose(actual, reference, rtol=2e-5, atol=2e-6)
    return {"passed": True, "old_weights_exact": True, "new_columns_zero": True,
            "parameters": 769, "initial_prediction_max_error": float(np.max(np.abs(actual - reference)))}


def loss_check(device):
    p = torch.zeros((3, 3), dtype=torch.float32, device=device, requires_grad=True)
    y = torch.tensor([[1., 2., -999.], [3., 3., 3.], [1., 2., 3.]], device=device)
    m = torch.tensor([[True, True, False], [True, True, True], [False, False, False]], device=device)
    loss = pair_loss(p, y, m)
    loss.backward()
    expected = np.zeros((3, 3), np.float32)
    expected[0] = (1 / 6, -1 / 6, 0)
    np.testing.assert_allclose(float(loss.item()), math.log(2) / 3, rtol=1e-6, atol=1e-7)
    np.testing.assert_allclose(p.grad.detach().cpu().numpy(), expected, rtol=1e-6, atol=1e-7)
    assert pair_loss(p.detach() - .1 * p.grad, y, m).item() < loss.item()
    assert pair_loss(p, y, torch.zeros_like(m)).item() == 0
    return {"passed": True, "gradient_direction": True, "ties_and_missing": True}


def fit_steps(model, x, y, mask, steps, lr, deadline):
    opt = torch.optim.Adam(model.parameters(), lr=lr)
    for step in range(steps):
        check_time(deadline)
        loss = pair_loss(model(x).squeeze(-1), y, mask)
        assert math.isfinite(loss.item())
        opt.zero_grad(set_to_none=True)
        loss.backward()
        if step == 0:
            assert all(torch.isfinite(p.grad).all().item() for p in model.parameters())
        opt.step()
    return opt


def resume_check(run, model, opt, x, y, mask, device):
    path = run / "calibration_16.pt"
    checkpoint(path, {"model": {k: v.detach().cpu() for k, v in model.state_dict().items()},
                      "optimizer": opt.state_dict()})
    resumed = model_new(device)
    saved = torch.load(path, map_location="cpu", weights_only=False)
    resumed.load_state_dict(saved["model"])
    other = torch.optim.Adam(resumed.parameters(), lr=.001)
    other.load_state_dict(saved["optimizer"])
    for net, optimizer in ((model, opt), (resumed, other)):
        loss = pair_loss(net(x).squeeze(-1), y, mask)
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        optimizer.step()
    for key, value in model.state_dict().items():
        assert torch.equal(value.detach().cpu(), resumed.state_dict()[key].detach().cpu()), key
    first, second = opt.state_dict(), other.state_dict()
    assert first["param_groups"] == second["param_groups"]
    assert first["state"].keys() == second["state"].keys()
    for key, values in first["state"].items():
        for name, value in values.items():
            assert torch.equal(value.detach().cpu(), second["state"][key][name].detach().cpu()), (key, name)
    return {"passed": True, "model_exact": True, "adam_state_exact": True, "checkpoint": path.name}


def synthetic_check(run, device="cpu", deadline=None):
    """元32列を0にし、追加列だけで既知の順位を学べるか確認する。"""
    deadline = time.time() + 120 if deadline is None else deadline
    initial, loss_result = initial_check(device), loss_check(device)
    raw = np.random.default_rng(SEED + 82).normal(size=(4, 32, WIDTH)).astype(np.float32)
    raw[:, :, :32] = 0
    targets, valid = np.floor(3 * raw[:, :, 32]), np.ones((4, 32), bool)
    x = torch.from_numpy(raw).to(device)
    y, mask = torch.from_numpy(targets).to(device), torch.from_numpy(valid).to(device)
    model = model_new(device)
    loss = pair_loss(model(x).squeeze(-1), y, mask)
    loss.backward()
    gradient = model[0].weight.grad[:, 32:].detach().cpu().numpy()
    assert np.isfinite(gradient).all() and np.any(gradient != 0)
    # 機構試験の学習率・更新数はv080の実4局面試験と同じにする。
    fit_steps(model, x, y, mask, 2000, .005, deadline)
    weights = model[0].weight[:, 32:].detach().cpu().numpy()
    assert np.isfinite(weights).all() and np.any(weights != 0)
    stats = pair_stats(array_predict(model, raw, device), targets, valid)
    assert stats["pair_accuracy"] >= .99, stats
    return {"passed": True, "initialization": initial, "loss": loss_result,
            "extra_gradient_nonzero": True, "extra_weights_updated": True,
            "max_extra_gradient": float(np.max(np.abs(gradient))),
            "known_extra_feature_control": {**stats, "updates": 2000, "lr": .005, "threshold": .99}}


def calibration(run, deadline):
    result = {"passed": False, "condition": NAME, "hidden": list(HIDDEN), "input_features": WIDTH,
              "torch": torch.__version__, "numpy": np.__version__,
              "recommended_max_memory": torch.mps.recommended_max_memory(), "checks": {}}
    output = run / "gpu_calibration.json"
    save(output, result)
    check_time(deadline)
    result["checks"]["synthetic_extra_feature"] = synthetic_check(run, "mps", deadline)
    raw = np.random.default_rng(SEED).normal(size=(256, 32, WIDTH)).astype(np.float32)
    targets, valid = np.floor(raw[:, :, 32]), np.ones((256, 32), bool)
    x, y, mask = tensor(raw), tensor(targets), tensor(valid)
    model = model_new("mps")
    original = {k: v.detach().cpu().clone() for k, v in model.state_dict().items()}
    opt = torch.optim.Adam(model.parameters(), lr=.001)
    for step in range(300):
        check_time(deadline)
        if step == 100:
            torch.mps.synchronize()
            started = time.monotonic()
        loss = pair_loss(model(x).squeeze(-1), y, mask)
        assert math.isfinite(loss.item())
        opt.zero_grad(set_to_none=True)
        loss.backward()
        if step == 0:
            assert all(torch.isfinite(p.grad).all().item() for p in model.parameters())
            assert model[0].weight.grad[:, 32:].abs().max().item() > 0
        opt.step()
    torch.mps.synchronize()
    result["seconds_per_update"] = (time.monotonic() - started) / 200
    result["parameters"] = 769
    result["benchmark"] = {"batch_size": 256, "warmup_updates": 100, "measured_updates": 200, "lr": .001}
    assert any(not torch.equal(value.detach().cpu(), original[key]) for key, value in model.state_dict().items())
    assert model[0].weight[:, 32:].abs().max().item() > 0
    cpu = model_new("cpu")
    cpu.load_state_dict({k: v.detach().cpu() for k, v in model.state_dict().items()})
    reference, actual = array_predict(cpu, raw[:4], "cpu"), predict(model, raw[:4])
    np.testing.assert_allclose(actual, reference, rtol=2e-5, atol=2e-4)
    np.testing.assert_array_equal(actual.argmin(1), reference.argmin(1))
    result["checks"]["cpu_mps_prediction"] = {"passed": True, "max_error": float(np.max(np.abs(actual-reference)))}
    cpu_p = torch.tensor(reference, requires_grad=True)
    gpu_p = torch.tensor(reference, device="mps", requires_grad=True)
    cpu_loss = pair_loss(cpu_p, torch.from_numpy(targets[:4]), torch.from_numpy(valid[:4]))
    gpu_loss = pair_loss(gpu_p, y[:4], mask[:4])
    cpu_loss.backward()
    gpu_loss.backward()
    np.testing.assert_allclose(gpu_loss.item(), cpu_loss.item(), rtol=2e-5, atol=2e-6)
    np.testing.assert_allclose(gpu_p.grad.cpu().numpy(), cpu_p.grad.numpy(), rtol=2e-5, atol=2e-6)
    result["checks"]["cpu_mps_loss_and_gradient"] = {"passed": True}
    check_time(deadline)
    result["checks"]["checkpoint_next_update"] = resume_check(run, model, opt, x, y, mask, "mps")
    result["passed"] = True
    save(output, result)


def tiny(run, deadline):
    data = load(run)
    description = json.loads((run / "dataset.json").read_text())
    ids = np.array([r[0] * 4 + r[3] for r in description["tiny_groups"]])
    assert len(ids) == 4
    x, y, mask = tensor(data["x"][ids]), tensor(data["y"][ids]), tensor(data["mask"][ids])
    model = model_new("mps")
    fit_steps(model, x, y, mask, 2000, .005, deadline)
    measured = pair_stats(predict(model, data["x"][ids]), data["y"][ids], data["mask"][ids])
    save(run / "tiny.json", {**measured, "passed": measured["pair_accuracy"] >= .95,
                            "threshold": .95, "groups_meta": description["tiny_groups"]})
    assert measured["pair_accuracy"] >= .95, measured


def train(run, target_steps, deadline):
    assert target_steps in STEPS
    name = NAME
    data = load(run)
    ids = subsets(data)["train49152"]
    model = model_new("mps")
    opt = torch.optim.Adam(model.parameters(), lr=.001)
    directory = run / "models" / name
    directory.mkdir(parents=True, exist_ok=True)
    latest = directory / "latest.pt"
    epoch = offset = steps = 0
    history = []
    if latest.exists():
        saved = torch.load(latest, map_location="cpu", weights_only=False)
    else:
        saved = None
    if saved is not None:
        assert saved["seed"] == SEED and saved["input_features"] == WIDTH
        assert saved["dataset_sha256"] == sha(run / "dataset.json")
        model.load_state_dict(saved["model"])
        opt.load_state_dict(saved["optimizer"])
        epoch, offset, steps = saved["epoch"], saved["offset"], saved["steps"]
        history = saved["history"]
    assert steps <= target_steps and steps * 256 == epoch * len(ids) + offset
    extra_gradient_max = saved["extra_gradient_max"] if saved is not None else 0.
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
                          "history": history, "seed": SEED, "hidden": HIDDEN,
                          "input_features": WIDTH, "dataset_sha256": sha(run / "dataset.json"),
                          "extra_gradient_max": extra_gradient_max,
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
        loss = pair_loss(model(xx[batch]).squeeze(-1), yy[batch], mm[batch])
        value = float(loss.item())
        assert math.isfinite(value)
        opt.zero_grad(set_to_none=True)
        loss.backward()
        if steps == initial_steps or steps % 1024 == 0:
            assert all(torch.isfinite(p.grad).all().item() for p in model.parameters())
            extra_gradient_max = max(extra_gradient_max, model[0].weight.grad[:, 32:].abs().max().item())
            assert extra_gradient_max > 0
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
    assert model[0].weight[:, 32:].abs().max().item() > 0
    if steps > initial_steps:
        assert any(not torch.equal(v.detach().cpu(), initial[k]) for k, v in model.state_dict().items())
    milestone = directory / f"step_{steps}.pt"
    persist(milestone)
    persist()
    reloaded = model_new("mps")
    reloaded.load_state_dict(torch.load(milestone, map_location="cpu", weights_only=False)["model"])
    np.testing.assert_array_equal(predict(model, data["x"][ids[:4]]), predict(reloaded, data["x"][ids[:4]]))
    cpu = model_new("cpu")
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
         "checkpoint_reload_exact": True, "cpu_mps_match": True, "extra_weights_updated": True, "checked_groups": len(check_ids),
         "extra_gradient_max": extra_gradient_max,
         "max_cpu_mps_error": float(np.max(np.abs(ref - gpu))), "checkpoint_sha256": sha(milestone),
         "prediction_sha256": sha(output), "elapsed_sec": time.monotonic() - started})


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--mode", choices=("calibrate", "tiny", "train"), required=True)
    parser.add_argument("--steps", type=int, choices=STEPS)
    parser.add_argument("--deadline", type=float, required=True)
    args = parser.parse_args()
    if args.mode == "train" and args.steps is None:
        parser.error("--steps required for train")
    torch.set_num_threads(30)
    torch.set_num_interop_threads(1)
    if not torch.backends.mps.is_available():
        raise RuntimeError("MPS required; no automatic CPU substitution")
    torch.mps.set_per_process_memory_fraction(16e9 / torch.mps.recommended_max_memory())
    report = GPUReport(args.run / "gpu_state.json")
    try:
        if args.mode == "calibrate":
            calibration(args.run, args.deadline)
        elif args.mode == "tiny":
            tiny(args.run, args.deadline)
        else:
            train(args.run, args.steps, args.deadline)
    except TimeoutError:
        raise SystemExit(75)
    finally:
        report.close()


if __name__ == "__main__":
    main()
