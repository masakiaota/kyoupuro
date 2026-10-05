#!/usr/bin/env python3
"""固定した候補集合内の順位学習。低い予測値の候補を選択する。"""
import argparse
import json
from pathlib import Path
import time

import numpy as np
import torch
from torch import nn
from torch.nn import functional as F

FEATURES = [
    "N", "K", "M", "floors", "density", "current_T", "best_T", "current_minus_best",
    "progress", "stagnant", "size", "size_over_M", "colors", "max_color_fraction",
    "color_entropy", "removed_commands", "broken_support_jumps", "removed_per_size",
    "broken_per_size", "priority", "nest_distance_mean", "nest_distance_max",
    "nest_distance_min", "pair_distance_mean", "pair_distance_max", "first_time",
    "last_time", "time_span", "flight_fraction", "passengers_per_size", "support_weight", "ride_weight",
]

def save_json(path, value):
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n")
    tmp.replace(path)

def checkpoint(path, value):
    tmp = path.with_suffix(".tmp")
    torch.save(value, tmp)
    tmp.replace(path)

def load_data(run, count):
    # 入力数ごとの配列を保存し、JSONの再読込なしでも後から検査できるようにする。
    shape = (count * 4, 32, 32)
    x = np.zeros(shape, dtype=np.float32)
    y = np.full(shape[:2], 100000, dtype=np.float32)
    mask = np.zeros(shape[:2], dtype=np.bool_)
    ranks = np.full(shape[:2], 100000, dtype=np.int32)
    meta = np.zeros((count * 4, 4), dtype=np.int32)
    manifest = [json.loads(line) for line in (run / "inputs.jsonl").read_text().splitlines()]
    first_completed = 0
    deadline_hits = 0
    teacher_overrun_sum = 0.0
    teacher_overrun_max = 0.0
    for item in manifest[:count]:
        i = item["index"]
        folder = run / "cases" / f"{i:06d}"
        assert (folder / "complete.json").exists(), folder
        for phase in range(4):
            meta[i * 4 + phase] = (i, int(item["role"] == "validation"), item["M"], phase)
        rows = [json.loads(line) for line in (folder / "candidates.jsonl").read_text().splitlines()]
        for row in rows:
            first_completed += int(row.get("first_completed", False))
            deadline_hits += int(row.get("deadline_hit", False))
            overrun = max(0.0, row.get("elapsed", 0.0) - row.get("budget", 0.0))
            teacher_overrun_sum += overrun
            teacher_overrun_max = max(teacher_overrun_max, overrun)
        for phase in range(4):
            selected = sorted((r for r in rows if r["phase"] == phase), key=lambda r: r["rank"])
            for j, row in enumerate(selected):
                g = i * 4 + phase
                x[g, j] = row["features"]
                y[g, j] = row["best_T"]
                mask[g, j] = True
                ranks[g, j] = row["rank"]
    valid = mask.sum(1) >= 2
    if not np.isfinite(x).all():
        raise ValueError("nonfinite feature")
    training = valid & (meta[:, 1] == 0)
    validation = valid & (meta[:, 1] == 1)
    informative = np.where(mask, y, np.inf).min(1) < np.where(mask, y, -np.inf).max(1)
    if not np.any(training & informative) or not np.any(validation & informative):
        raise ValueError("no differing teacher pairs in train or validation")
    samples = x[training][mask[training]]
    mean = samples.mean(0, dtype=np.float64).astype(np.float32)
    scale = samples.std(0, dtype=np.float64).astype(np.float32)
    scale[scale < 1e-6] = 1.0
    del samples
    x = (x - mean) / scale
    x[~mask] = 0
    summary = {
        "inputs": count, "groups": int(valid.sum()), "candidate_rows": int(mask.sum()),
        "missing_groups": int((~valid).sum()), "informative_groups": int((valid & informative).sum()),
        "train_groups": int(training.sum()), "validation_groups": int(validation.sum()),
        "train_informative": int((training & informative).sum()),
        "validation_informative": int((validation & informative).sum()),
        "first_trial_completed": first_completed, "deadline_hits": deadline_hits,
        "teacher_overrun_mean_sec": teacher_overrun_sum / max(1, int(mask.sum())),
        "teacher_overrun_max_sec": teacher_overrun_max,
        "mean": mean.tolist(), "scale": scale.tolist(), "features": FEATURES,
    }
    return x, y, mask, ranks, meta, training, validation, informative, summary

def metrics(prediction, y, mask, meta):
    prediction = np.where(mask, prediction, np.inf)
    # 元順位順で並べているため、予測値が同じなら現行上位を選ぶ。
    selected = prediction.argmin(1)
    chosen = y[np.arange(len(y)), selected]
    base = y[:, 0]
    oracle = np.where(mask, y, np.inf).min(1)
    ids = meta[:, 0]
    unique, inverse = np.unique(ids, return_inverse=True)
    group_count = np.bincount(inverse)
    def per_input(a):
        return np.bincount(inverse, weights=a) / group_count
    chosen_i, base_i, oracle_i = map(per_input, (chosen, base, oracle))
    matches = 0.0
    pairs = 0
    for start in range(0, len(y), 2048):
        yy, mm, pp = y[start:start+2048], mask[start:start+2048], prediction[start:start+2048]
        different = (yy[:, :, None] < yy[:, None, :]) & mm[:, :, None] & mm[:, None, :]
        with np.errstate(invalid="ignore"):
            matches += ((pp[:, :, None] < pp[:, None, :]) & different).sum()
            matches += 0.5 * ((pp[:, :, None] == pp[:, None, :]) & different).sum()
        pairs += different.sum()
    group_M = np.bincount(inverse, weights=meta[:, 2]) / group_count
    result = {
        "inputs": int(len(unique)), "selected_T": float(chosen_i.mean()),
        "baseline_T": float(base_i.mean()), "oracle_T": float(oracle_i.mean()),
        "delta_vs_baseline": float((chosen_i - base_i).mean()),
        "gap_to_oracle": float((chosen_i - oracle_i).mean()),
        "pair_accuracy": float(matches / pairs) if pairs else None,
        "differing_pairs": int(pairs),
    }
    for name, condition in (("M_ge_80", group_M >= 80), ("M_lt_80", group_M < 80)):
        result[name] = {"inputs": int(condition.sum()), "delta_vs_baseline": float((chosen_i - base_i)[condition].mean())}
    return result

def predict(model, x, indices, device):
    output = []
    with torch.no_grad():
        for start in range(0, len(indices), 1024):
            batch = torch.from_numpy(x[indices[start:start+1024]]).to(device)
            output.append(model(batch).squeeze(-1).cpu().numpy())
    return np.concatenate(output)

def train(run, count, deadline):
    torch.set_num_threads(30)
    torch.set_num_interop_threads(1)
    if not torch.backends.mps.is_available():
        raise RuntimeError("MPS is required for this registered run")
    device = torch.device("mps")
    destination = run / "models" / f"n{count:06d}"
    destination.mkdir(parents=True, exist_ok=True)
    x, y, mask, ranks, meta, training, validation, informative, summary = load_data(run, count)
    save_json(destination / "dataset.json", summary)
    save_json(destination / "runtime.json", {"torch": torch.__version__, "numpy": np.__version__, "device": str(device), "seed": 77001})
    train_indices = np.flatnonzero(training & informative)
    val_indices = np.flatnonzero(validation)
    all_metrics = {}
    for name in ("linear", "mlp16"):
        finished_path = destination / f"{name}_finished.json"
        if finished_path.exists():
            all_metrics[name] = json.loads(finished_path.read_text())["validation"]
            continue
        if time.time() >= deadline:
            break
        torch.manual_seed(77001)
        torch.mps.manual_seed(77001)
        rng = np.random.default_rng(77001)
        model = nn.Linear(32, 1) if name == "linear" else nn.Sequential(nn.Linear(32, 16), nn.ReLU(), nn.Linear(16, 1))
        model = model.to(device)
        optimizer = torch.optim.Adam(model.parameters(), lr=0.001)
        best_metric, best_epoch, stale, history = float("inf"), 0, 0, []
        best_state = None
        first_epoch = 1
        latest = destination / f"{name}_latest.pt"
        if latest.exists():
            saved = torch.load(latest, map_location="cpu", weights_only=False)
            model.load_state_dict(saved["model"])
            optimizer.load_state_dict(saved["optimizer"])
            best_metric, best_epoch, stale = saved["best_metric"], saved["best_epoch"], saved["stale"]
            best_state = saved["best_state"]
            first_epoch = saved["epoch"] + 1
            rng.bit_generator.state = saved["numpy_rng"]
            torch.set_rng_state(saved["torch_rng"])
            torch.mps.set_rng_state(saved["mps_rng"])
            history = json.loads((destination / f"{name}_history.json").read_text())
            all_metrics[name] = next(row for row in history if row["epoch"] == best_epoch)
        epoch = first_epoch - 1
        interrupted = False
        for epoch in range(first_epoch, 81):
            if time.time() >= deadline:
                break
            model.train()
            order = rng.permutation(train_indices)
            loss_sum, groups = 0.0, 0
            interrupted = False
            for start in range(0, len(order), 256):
                if time.time() >= deadline:
                    interrupted = True
                    break
                indices = order[start:start+256]
                xx = torch.from_numpy(x[indices]).to(device)
                yy = torch.from_numpy(y[indices]).to(device)
                mm = torch.from_numpy(mask[indices]).to(device)
                output = model(xx).squeeze(-1)
                pairs = (yy[:, :, None] < yy[:, None, :]) & mm[:, :, None] & mm[:, None, :]
                pair_losses = F.softplus(output[:, :, None] - output[:, None, :])
                per_group = (pair_losses * pairs).sum((1, 2)) / pairs.sum((1, 2)).clamp_min(1)
                loss = per_group.mean()
                if not torch.isfinite(loss).item():
                    raise ValueError("nonfinite training loss")
                optimizer.zero_grad(set_to_none=True)
                loss.backward()
                optimizer.step()
                loss_sum += float(loss.item()) * len(indices)
                groups += len(indices)
            model.eval()
            prediction = predict(model, x, val_indices, device)
            measured = metrics(prediction, y[val_indices], mask[val_indices], meta[val_indices])
            row = {"epoch": epoch, "loss": loss_sum / max(1, groups), "partial_epoch": interrupted, **measured}
            history.append(row)
            if measured["selected_T"] < best_metric:
                best_metric, best_epoch, stale = measured["selected_T"], epoch, 0
                best_state = {k: v.detach().cpu().clone() for k, v in model.state_dict().items()}
                all_metrics[name] = row
            else:
                stale += 1
            state = {
                "model": {k: v.detach().cpu() for k, v in model.state_dict().items()},
                "optimizer": optimizer.state_dict(), "epoch": epoch, "best_epoch": best_epoch,
                "best_state": best_state, "best_metric": best_metric, "stale": stale,
                "numpy_rng": rng.bit_generator.state, "torch_rng": torch.get_rng_state(),
                "mps_rng": torch.mps.get_rng_state(), "normalization": summary,
            }
            checkpoint(destination / f"{name}_latest.pt", state)
            save_json(destination / f"{name}_history.json", history)
            print(json.dumps({"stage": "training", "training_dataset_inputs": count, "model": name, **row}), flush=True)
            if stale >= 12 or interrupted:
                break
        if best_state is not None:
            checkpoint(destination / f"{name}_best.pt", {"model": best_state, "epoch": best_epoch, "normalization": summary})
            exported = {"model": name, "features": FEATURES, "mean": summary["mean"], "scale": summary["scale"],
                        "weights": {k: v.tolist() for k, v in best_state.items()}, "best_epoch": best_epoch,
                        "selection": "argmin; ties use original rank", "validation": all_metrics[name]}
            save_json(destination / f"{name}_weights.json", exported)
            if stale >= 12 or (epoch >= 80 and not interrupted):
                save_json(finished_path, {"validation": all_metrics[name], "last_epoch": epoch})
        del model, optimizer
        torch.mps.empty_cache()
    save_json(destination / "summary.json", {"inputs": count, "models": all_metrics, "completed_at": time.time()})
    return all_metrics

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--count", type=int, required=True)
    parser.add_argument("--deadline", type=float, required=True)
    args = parser.parse_args()
    train(args.run.resolve(), args.count, args.deadline)
