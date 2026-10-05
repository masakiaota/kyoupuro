#!/usr/bin/env python3
"""v080の固定条件で、候補間の手数差を重みとする順位損失を学習する。"""
import argparse
import json
import math
import os
from pathlib import Path
import signal
import time

import numpy as np
import torch
from torch.nn import functional as F

from train_v077_rank import checkpoint
from train_v078_targets import pair_loss, pair_stats
from train_v080_scaling import GPUReport, model_new, predict, tensor, train
from v080_data import CONDITIONS, SEED, load, save


CONDITION = "n49152_h16"
HIDDEN = CONDITIONS[CONDITION][1]


def pair_gap_loss(prediction, target, mask):
    gap = target[:, None, :] - target[:, :, None]
    pairs = (target[:, :, None] < target[:, None, :]) & mask[:, :, None] & mask[:, None, :]
    losses = F.softplus(prediction[:, :, None] - prediction[:, None, :])
    weights = torch.where(pairs, gap, 0)
    # 手数差の大きい局面も重くするため、重みの総和ではなく候補対数で割る。
    per_group = (losses * weights).sum((1, 2)) / pairs.sum((1, 2)).clamp_min(1)
    # v080と局面順・更新回数をそろえ、全候補同点の局面も0として平均に含める。
    return per_group.mean()


def check_time(deadline):
    if time.time() >= deadline:
        raise TimeoutError("registered_time_limit_or_signal")


def value_gradient(prediction, target, mask, device, loss_function=pair_gap_loss):
    p = torch.tensor(prediction, dtype=torch.float32, device=device, requires_grad=True)
    y = torch.tensor(target, dtype=torch.float32, device=device)
    m = torch.tensor(mask, dtype=torch.bool, device=device)
    loss = loss_function(p, y, m)
    loss.backward()
    return float(loss.item()), p.grad.detach().cpu().numpy()


def self_test(device):
    # 先頭局面の3対の手数差は1, 3, 2。残り2局面は同点と全候補欠測である。
    p = np.zeros((3, 4), np.float32)
    y = np.array([[1, 2, 4, -999], [3, 3, 3, -999], [1, 2, 3, 4]], np.float32)
    mask = np.array([[True, True, True, False], [True, True, True, False],
                     [False, False, False, False]])
    loss, grad = value_gradient(p, y, mask, device)
    expected_loss = 2 * math.log(2) / 3
    expected_grad = np.zeros_like(p)
    expected_grad[0] = (2 / 9, 1 / 18, -5 / 18, 0)
    np.testing.assert_allclose(loss, expected_loss, rtol=1e-6, atol=1e-7)
    np.testing.assert_allclose(grad, expected_grad, rtol=1e-6, atol=1e-7)
    assert grad[0, 0] > 0 and grad[0, 2] < 0
    assert not grad[1:].any() and not grad[~mask].any()
    after_loss, _ = value_gradient(p - .1 * grad, y, mask, device)
    assert after_loss < loss
    absent_loss, absent_grad = value_gradient(p, y, np.zeros_like(mask), device)
    assert absent_loss == 0 and not absent_grad.any()
    # 欠測候補の値は、損失にも有効候補の勾配にも影響しない。
    absent_p, absent_y = p.copy(), y.copy()
    absent_p[~mask], absent_y[~mask] = 123, 10000
    changed_loss, changed_grad = value_gradient(absent_p, absent_y, mask, device)
    np.testing.assert_allclose(changed_loss, loss, rtol=1e-6, atol=1e-7)
    np.testing.assert_array_equal(changed_grad, grad)

    one_loss, one_grad = value_gradient([[.25, -.5]], [[10, 11]], [[True, True]], device)
    three_loss, three_grad = value_gradient([[.25, -.5]], [[10, 13]], [[True, True]], device)
    np.testing.assert_allclose(three_loss, 3 * one_loss, rtol=1e-6, atol=1e-7)
    np.testing.assert_allclose(three_grad, 3 * one_grad, rtol=1e-6, atol=1e-7)

    unit_p = np.array([[.25, -.75, 1.5, 9], [1, -1.25, .5, .25]], np.float32)
    unit_y = np.array([[10, 11, 10, -999], [3, 3, 4, 3]], np.float32)
    unit_mask = np.array([[True, True, True, False], [True, True, True, True]])
    unit_loss, unit_grad = value_gradient(unit_p, unit_y, unit_mask, device)
    old_loss, old_grad = value_gradient(unit_p, unit_y, unit_mask, device, pair_loss)
    np.testing.assert_allclose(unit_loss, old_loss, rtol=1e-6, atol=1e-7)
    np.testing.assert_allclose(unit_grad, old_grad, rtol=1e-6, atol=1e-7)
    # 予測と教師のそれぞれに局面単位の定数を加えても、差は変わらない。
    shift = np.array([[16], [-8]], np.float32)
    shifted_p_loss, shifted_p_grad = value_gradient(unit_p + shift, unit_y, unit_mask, device)
    shifted_y_loss, shifted_y_grad = value_gradient(unit_p, unit_y + shift, unit_mask, device)
    for shifted_loss, shifted_grad in ((shifted_p_loss, shifted_p_grad),
                                       (shifted_y_loss, shifted_y_grad)):
        np.testing.assert_allclose(shifted_loss, unit_loss, rtol=1e-6, atol=1e-7)
        np.testing.assert_allclose(shifted_grad, unit_grad, rtol=1e-6, atol=1e-7)
    return {"passed": True, "hand_calculated_loss": loss, "expected_loss": expected_loss,
            "max_gradient_error": float(np.max(np.abs(grad - expected_grad))),
            "gradient_direction": True, "gradient_step_reduces_loss": True,
            "ties_and_missing_candidates": True, "missing_values_do_not_affect_loss": True,
            "gap_three_times_loss_and_gradient": True, "gap_loss_ratio": three_loss / one_loss,
            "unit_gap_matches_pair_loss": True, "unit_gap_loss_error": abs(unit_loss - old_loss),
            "unit_gap_max_gradient_error": float(np.max(np.abs(unit_grad - old_grad))),
            "prediction_shift_invariant": True, "teacher_shift_invariant": True}


def fit_gap_steps(model, x, y, mask, steps, lr, deadline):
    opt = torch.optim.Adam(model.parameters(), lr=lr)
    for step in range(steps):
        check_time(deadline)
        loss = pair_gap_loss(model(x).squeeze(-1), y, mask)
        assert math.isfinite(loss.item())
        opt.zero_grad(set_to_none=True)
        loss.backward()
        if step == 0:
            assert all(torch.isfinite(p.grad).all().item() for p in model.parameters())
        opt.step()
    torch.mps.synchronize()


def check_next_update(run, model, opt, x, y, mask, deadline):
    check_time(deadline)
    path = run / "calibration_16.pt"
    checkpoint(path, {"model": model.state_dict(), "optimizer": opt.state_dict()})
    resumed = model_new(HIDDEN, "mps")
    loaded = torch.load(path, map_location="cpu", weights_only=False)
    resumed.load_state_dict(loaded["model"])
    resumed_opt = torch.optim.Adam(resumed.parameters(), lr=.001)
    resumed_opt.load_state_dict(loaded["optimizer"])
    for net, optimizer in ((model, opt), (resumed, resumed_opt)):
        check_time(deadline)
        loss = pair_gap_loss(net(x).squeeze(-1), y, mask)
        assert math.isfinite(loss.item())
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        optimizer.step()
    torch.mps.synchronize()
    for name, value in model.state_dict().items():
        assert torch.equal(value.detach().cpu(), resumed.state_dict()[name].detach().cpu()), name
    original_state, resumed_state = opt.state_dict(), resumed_opt.state_dict()
    assert original_state["param_groups"] == resumed_state["param_groups"]
    assert original_state["state"].keys() == resumed_state["state"].keys()
    for key, state in original_state["state"].items():
        other = resumed_state["state"][key]
        assert state.keys() == other.keys()
        for name, value in state.items():
            assert torch.equal(value.detach().cpu(), other[name].detach().cpu()), (key, name)
    return {"passed": True, "model_exact": True, "adam_state_exact": True,
            "checkpoint": path.name}


def calibration(run, deadline):
    result = {"passed": False, "seconds_per_update": None, "condition": CONDITION,
              "hidden": list(HIDDEN), "torch": torch.__version__, "numpy": np.__version__,
              "recommended_max_memory": torch.mps.recommended_max_memory(), "checks": {}}
    output = run / "gpu_calibration.json"
    save(output, result)
    try:
        check_time(deadline)
        result["checks"]["loss_self_test"] = self_test("mps")
        random = np.random.default_rng(SEED)
        raw = random.normal(size=(256, 32, 32)).astype(np.float32)
        target = np.floor(raw[:, :, 10])
        valid = np.ones((256, 32), bool)
        x, y, mask = tensor(raw), tensor(target), tensor(valid)
        model = model_new(HIDDEN, "mps")
        result["parameters"] = sum(p.numel() for p in model.parameters())
        assert result["parameters"] == 545
        opt = torch.optim.Adam(model.parameters(), lr=.001)
        original = [p.detach().cpu().clone() for p in model.parameters()]
        # 最初の100更新で初回処理を済ませ、本学習と同じloss.item()の同期を計測する。
        for step in range(300):
            check_time(deadline)
            if step == 100:
                torch.mps.synchronize()
                started = time.monotonic()
            loss = pair_gap_loss(model(x).squeeze(-1), y, mask)
            assert math.isfinite(loss.item())
            opt.zero_grad(set_to_none=True)
            loss.backward()
            if step == 0:
                assert all(torch.isfinite(p.grad).all().item() for p in model.parameters())
                result["checks"]["finite_gradients"] = True
            opt.step()
        torch.mps.synchronize()
        result["seconds_per_update"] = (time.monotonic() - started) / 200
        result["benchmark"] = {"batch_size": 256, "warmup_updates": 100,
                               "measured_updates": 200, "learning_rate": .001}
        assert any(not torch.equal(a, b.detach().cpu()) for a, b in zip(original, model.parameters()))
        result["checks"]["weights_updated"] = True
        save(output, result)

        check_time(deadline)
        cpu = model_new(HIDDEN, "cpu")
        cpu.load_state_dict({k: v.detach().cpu() for k, v in model.state_dict().items()})
        with torch.no_grad():
            reference = cpu(torch.from_numpy(raw[:4])).squeeze(-1).numpy()
        gpu = predict(model, raw[:4])
        np.testing.assert_allclose(gpu, reference, rtol=2e-5, atol=2e-4)
        result["checks"]["cpu_mps_prediction"] = {
            "passed": True, "max_error": float(np.max(np.abs(gpu - reference)))}
        loss_mask = valid[:4].copy()
        loss_mask[0, -3:] = False
        cpu_loss, cpu_grad = value_gradient(raw[:4, :, 10], target[:4], loss_mask, "cpu")
        gpu_loss, gpu_grad = value_gradient(raw[:4, :, 10], target[:4], loss_mask, "mps")
        np.testing.assert_allclose(gpu_loss, cpu_loss, rtol=2e-5, atol=2e-6)
        np.testing.assert_allclose(gpu_grad, cpu_grad, rtol=2e-5, atol=2e-6)
        result["checks"]["cpu_mps_loss_and_gradient"] = {
            "passed": True, "loss_error": abs(gpu_loss - cpu_loss),
            "max_gradient_error": float(np.max(np.abs(gpu_grad - cpu_grad)))}
        result["checks"]["checkpoint_next_update"] = check_next_update(run, model, opt, x, y, mask, deadline)
        save(output, result)

        control = model_new(HIDDEN, "mps")
        # 小さい手数差の勾配を弱める損失なので、人工順位への当てはまりには2,000更新を与える。
        fit_gap_steps(control, x[:4], x[:4, :, 10], mask[:4], 2000, .005, deadline)
        control_stats = pair_stats(predict(control, raw[:4]), raw[:4, :, 10], valid[:4])
        accuracy = control_stats["pair_accuracy"]
        result["checks"]["known_feature_control"] = {
            "passed": accuracy is not None and accuracy >= .99, "pair_accuracy": accuracy,
            "threshold": .99, "updates": 2000, "learning_rate": .005}
        assert accuracy is not None and accuracy >= .99, ("known feature control", accuracy)
        check_time(deadline)
        result["passed"] = True
        save(output, result)
    except Exception as error:
        result["error"] = f"{type(error).__name__}: {error}"
        save(output, result)
        raise


def tiny(run, deadline):
    result = {"passed": False, "condition": CONDITION, "updates": 2000,
              "learning_rate": .005, "threshold": .95}
    output = run / "tiny.json"
    save(output, result)
    try:
        check_time(deadline)
        data = load(run)
        description = json.loads((run / "dataset.json").read_text())
        groups = np.array(description["tiny_groups"], dtype=np.int64)
        assert groups.shape == (4, 4)
        ids = groups[:, 0] * 4 + groups[:, 3]
        assert len(np.unique(ids)) == 4
        np.testing.assert_array_equal(data["meta"][ids], groups)
        x, y, mask = tensor(data["x"][ids]), tensor(data["y"][ids]), tensor(data["mask"][ids])
        model = model_new(HIDDEN, "mps")
        fit_gap_steps(model, x, y, mask, 2000, .005, deadline)
        measured = pair_stats(predict(model, data["x"][ids]), data["y"][ids], data["mask"][ids])
        result.update(measured)
        with torch.no_grad():
            result["weighted_all_group_loss"] = float(pair_gap_loss(model(x).squeeze(-1), y, mask).item())
        result["tiny_groups"] = groups.tolist()
        assert measured["pair_accuracy"] is not None and measured["pair_accuracy"] >= .95, (
            "tiny fit below registered threshold", measured)
        check_time(deadline)
        result["passed"] = True
        save(output, result)
    except Exception as error:
        result["error"] = f"{type(error).__name__}: {error}"
        save(output, result)
        raise


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--mode", choices=("calibrate", "tiny", "train"), required=True)
    parser.add_argument("--steps", type=int)
    parser.add_argument("--deadline", type=float, required=True)
    args = parser.parse_args()
    if args.mode == "train" and (args.steps is None or args.steps <= 0):
        parser.error("--steps must be positive in train mode")
    if not math.isfinite(args.deadline):
        parser.error("--deadline must be a finite Unix timestamp")
    torch.set_num_threads(30)
    torch.set_num_interop_threads(1)
    if os.environ.get("PYTORCH_ENABLE_MPS_FALLBACK", "0") not in ("", "0"):
        raise RuntimeError("MPS CPU fallback must be disabled")
    if not torch.backends.mps.is_available():
        raise RuntimeError("MPS required; no automatic CPU substitution")
    torch.mps.set_per_process_memory_fraction(16e9 / torch.mps.recommended_max_memory())
    report = GPUReport(args.run / "gpu_state.json")

    def stop(signum, frame):
        raise TimeoutError("registered_time_limit_or_signal")

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    try:
        check_time(args.deadline)
        if args.mode == "calibrate":
            calibration(args.run, args.deadline)
        elif args.mode == "tiny":
            tiny(args.run, args.deadline)
        else:
            train(args.run, CONDITION, args.steps, args.deadline, loss_function=pair_gap_loss)
    except TimeoutError:
        raise SystemExit(75)
    finally:
        report.close()


if __name__ == "__main__":
    main()
