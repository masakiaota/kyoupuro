#!/usr/bin/env python3
"""v077の保存教師を4種類に分ける、事前登録済みの学習診断。solverは実行しない。"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import sys
import time
import traceback

import numpy as np
import torch
from torch import nn
from torch.nn import functional as F

from train_v077_rank import FEATURES, checkpoint, save_json


TARGETS = ("rollout", "immediate", "changed", "cost")
MEASURES = ("rollout_T", "first_T", "first_best_T", "first_ms", "changed_nonworse",
            "shorter", "neutral_changed", "accepted_uphill", "reinsertion_completed",
            "extract_failed", "insert_failed", "duplicate", "rejected")
SEED = 78001


def utc_now():
    return datetime.now(timezone.utc).isoformat()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def model_new(device):
    torch.manual_seed(SEED)
    torch.mps.manual_seed(SEED)
    return nn.Sequential(nn.Linear(32, 16), nn.ReLU(), nn.Linear(16, 1)).to(device)


def pair_loss(prediction, target, mask):
    pairs = (target[:, :, None] < target[:, None, :]) & mask[:, :, None] & mask[:, None, :]
    losses = F.softplus(prediction[:, :, None] - prediction[:, None, :])
    per_group = (losses * pairs).sum((1, 2)) / pairs.sum((1, 2)).clamp_min(1)
    # 全教師が同じ局面順・更新回数になるよう、優劣なしの局面も0として含める。
    return per_group.mean()


def pair_stats(prediction, target, mask):
    correct = pairs_total = informative = 0
    loss_sum = 0.0
    for start in range(0, len(target), 512):
        p, y, m = prediction[start:start + 512], target[start:start + 512], mask[start:start + 512]
        pairs = (y[:, :, None] < y[:, None, :]) & m[:, :, None] & m[:, None, :]
        differences = p[:, :, None] - p[:, None, :]
        count = pairs.sum((1, 2))
        good = count > 0
        losses = (np.logaddexp(0.0, differences) * pairs).sum((1, 2)) / np.maximum(count, 1)
        loss_sum += float(losses[good].sum())
        informative += int(good.sum())
        pairs_total += int(count.sum())
        correct += float(((differences < 0) & pairs).sum())
        correct += 0.5 * float(((differences == 0) & pairs).sum())
    return {"pair_accuracy": correct / pairs_total if pairs_total else None,
            "differing_pairs": pairs_total, "informative_groups": informative,
            "groups": len(target), "tied_group_fraction": 1 - informative / len(target),
            "informative_loss": loss_sum / informative if informative else None,
            "all_group_loss": loss_sum / len(target)}


def prediction(model, x, device):
    model.eval()
    with torch.no_grad():
        pieces = [model(torch.from_numpy(x[s:s + 1024]).to(device)).squeeze(-1).cpu().numpy()
                  for s in range(0, len(x), 1024)]
    result = np.concatenate(pieces)
    if not np.isfinite(result).all():
        raise ValueError("nonfinite prediction")
    return result


def self_test(device):
    # 正しい候補の出力が下がる勾配、同点・埋め草の除外を独立した小例で確認する。
    p = torch.zeros((2, 3), device=device, requires_grad=True)
    y = torch.tensor([[1., 2., -999.], [3., 3., 3.]], device=device)
    m = torch.tensor([[True, True, False], [True, True, True]], device=device)
    loss = pair_loss(p, y, m)
    loss.backward()
    assert abs(float(loss.item()) - math.log(2) / 2) < 1e-6
    grad = p.grad.cpu().numpy()
    assert grad[0, 0] > 0 and grad[0, 1] < 0 and grad[0, 2] == 0 and not grad[1].any()
    stats = pair_stats(p.detach().cpu().numpy(), y.cpu().numpy(), m.cpu().numpy())
    assert stats["pair_accuracy"] == .5 and stats["differing_pairs"] == 1
    after = p.detach() - .1 * p.grad
    assert pair_loss(after, y, m).item() < loss.item()
    assert pair_loss(after, y, torch.zeros_like(m)).item() == 0
    a = model_new(device)
    b = model_new("cpu")
    b.load_state_dict({k: v.detach().cpu() for k, v in a.state_dict().items()})
    xx = np.random.default_rng(SEED).normal(size=(3, 32, 32)).astype(np.float32)
    np.testing.assert_allclose(prediction(a, xx, device), prediction(b, xx, "cpu"), rtol=2e-5, atol=2e-6)
    assert sum(p.numel() for p in a.parameters()) == 545
    return {"passed": True, "gradient_direction": True, "tie_and_padding": True,
            "cpu_mps_inference": True, "parameters": 545}


class Run:
    def __init__(self, output, seconds):
        self.output = output
        self.deadline = time.monotonic() + seconds

    def status(self, stage, **fields):
        save_json(self.output / "status.json", {"stage": stage, "pid": os.getpid(),
                  "updated_at": utc_now(), "seconds_remaining": max(0, self.deadline - time.monotonic()), **fields})

    def check_time(self):
        if time.monotonic() >= self.deadline:
            raise TimeoutError("registered_time_limit")


def load_dataset(source, job):
    destination = job.output
    cache = destination / "dataset.npz"
    if cache.exists():
        description = json.loads((destination / "dataset.json").read_text())
        assert digest(cache.read_bytes()) == description["cache_sha256"]
        with np.load(cache) as z:
            return {k: z[k] for k in z.files}, description
    manifest_bytes = (source / "inputs.jsonl").read_bytes()
    items = [json.loads(line) for line in manifest_bytes.splitlines()]
    chosen = [r for r in items if r["index"] // 8 % 8 == 1]
    assert len(chosen) == 8192
    assert len({r["index"] for r in chosen}) == len(chosen)
    assert len({r["sha256"] for r in chosen}) == len(chosen)
    assert sum(r["role"] == "train" for r in chosen) == 6144
    for role, count in (("train", 3072), ("validation", 1024)):
        assert sum(r["role"] == role and r["M"] >= 80 for r in chosen) == count
        assert sum(r["role"] == role and r["M"] < 80 for r in chosen) == count
    save_json(destination / "inputs.json", chosen)
    shape = (len(chosen) * 4, 32)
    x = np.zeros((*shape, 32), np.float32)
    targets = np.zeros((4, *shape), np.float32)
    measures = np.zeros((*shape, len(MEASURES)), np.float32)
    mask = np.zeros(shape, bool)
    ranks = np.full(shape, 100000, np.int32)
    meta = np.zeros((shape[0], 4), np.int32)
    audit_hash = hashlib.sha256()
    allowed = {"accepted", "extract_failed", "insert_failed", "duplicate", "rejected"}
    for position, item in enumerate(chosen):
        job.check_time()
        folder = source / "cases" / f"{item['index']:06d}"
        complete = json.loads((folder / "complete.json").read_text())
        assert json.loads((folder / "input.json").read_text()) == item
        assert digest((source / item["path"]).read_bytes()) == item["sha256"]
        raw = (folder / "candidates.jsonl").read_bytes()
        states_raw = (folder / "states.jsonl").read_bytes()
        assert digest(raw) == complete["candidates_sha256"]
        assert digest(states_raw) == complete["states_sha256"]
        audit_hash.update(f"{item['index']}:{digest(raw)}:{digest(states_raw)}\n".encode())
        rows = [json.loads(line) for line in raw.splitlines()]
        assert len(rows) == complete["rows"]
        states = {r["phase"]: r for r in map(json.loads, states_raw.splitlines())}
        for phase in range(4):
            g = position * 4 + phase
            meta[g] = (item["index"], int(item["role"] == "validation"), item["M"], phase)
            group = sorted((r for r in rows if r["phase"] == phase), key=lambda r: r["rank"])
            assert len(group) <= 32
            assert len({r["rank"] for r in group}) == len(group)
            if group:
                assert group[0]["rank"] == states[phase]["baseline_rank"]
            for j, row in enumerate(group):
                f = row["features"]
                assert len(f) == 32 and f[2] == item["M"]
                assert row["snapshot_hash"] == states[phase]["snapshot_hash"]
                assert row["rng"] == states[phase]["rng"]
                assert row["first_completed"] and row["first_outcome"] in allowed
                assert row["pool_free"] == 4 and row["E"] == 0 and row["errors"] == 0
                assert row["best_T"] <= row["first_best_T"] <= f[6]
                assert row["first_best_T"] <= row["first_T"]
                assert row["first_elapsed"] > 0 and math.isfinite(row["first_elapsed"])
                assert f[5] == len(states[phase]["current"]) and f[6] == len(states[phase]["best"])
                outcome = row["first_outcome"]
                accepted = outcome == "accepted"
                if not accepted:
                    assert row["first_T"] == f[5] and row["first_best_T"] == f[6]
                changed = accepted and row["first_T"] <= f[5]
                x[g, j], mask[g, j], ranks[g, j] = f, True, row["rank"]
                targets[:, g, j] = (row["best_T"], row["first_T"], int(not changed),
                                    math.floor(4 * math.log2(row["first_elapsed"] / 1e-6)))
                measures[g, j] = (row["best_T"], row["first_T"], row["first_best_T"],
                    row["first_elapsed"] * 1000, changed, accepted and row["first_T"] < f[5],
                    accepted and row["first_T"] == f[5], accepted and row["first_T"] > f[5],
                    outcome in ("accepted", "rejected", "duplicate"), outcome == "extract_failed",
                    outcome == "insert_failed", outcome == "duplicate", outcome == "rejected")
        if (position + 1) % 256 == 0:
            job.status("loading", completed_inputs=position + 1, total_inputs=len(chosen))
    valid = mask.sum(1) >= 2
    missing_groups = meta[~valid].tolist()
    x, targets, measures, mask, ranks, meta = x[valid], targets[:, valid], measures[valid], mask[valid], ranks[valid], meta[valid]
    assert np.isfinite(x).all() and np.isfinite(targets).all() and np.isfinite(measures).all()
    training = meta[:, 1] == 0
    samples = x[training][mask[training]]
    mean = samples.mean(0, dtype=np.float64).astype(np.float32)
    scale = samples.std(0, dtype=np.float64).astype(np.float32)
    scale[scale < 1e-6] = 1
    del samples
    x = (x - mean) / scale
    x[~mask] = 0
    informative = (np.where(mask[None], targets, np.inf).min(2) <
                   np.where(mask[None], targets, -np.inf).max(2))
    tiny = []
    for high in (False, True):
        for late in (False, True):
            eligible = training & ((meta[:, 2] >= 80) == high) & ((meta[:, 3] >= 2) == late) & informative.all(0)
            if not eligible.any():
                raise ValueError("no preregistered tiny-fit group")
            tiny.append(int(np.flatnonzero(eligible)[0]))
    data = dict(x=x, targets=targets, measures=measures, mask=mask, ranks=ranks, meta=meta,
                mean=mean, scale=scale, tiny=np.array(tiny, np.int32))
    temporary = destination / "dataset.tmp.npz"
    np.savez(temporary, **data)
    description = {"source": str(source), "manifest_sha256": digest(manifest_bytes),
        "source_cases_digest": audit_hash.hexdigest(), "cache_sha256": digest(temporary.read_bytes()),
        "selection": "index // 8 % 8 == 1", "features": FEATURES, "targets": TARGETS, "measures": MEASURES,
        "inputs": len(chosen), "train_inputs": 6144, "validation_inputs": 2048,
        "groups": len(meta), "candidate_rows": int(mask.sum()), "missing_groups": missing_groups,
        "train_informative_groups": dict(zip(TARGETS, map(int, informative[:, training].sum(1)))),
        "validation_informative_groups": dict(zip(TARGETS, map(int, informative[:, ~training].sum(1)))),
        "tiny_groups": meta[tiny].tolist(), "mean": mean.tolist(), "scale": scale.tolist()}
    save_json(destination / "dataset.json", description)
    temporary.replace(cache)
    return data, description


def train_one(job, name, x, y, mask, device, epochs, batch_size, lr, save_every):
    directory = job.output / "models" / name
    directory.mkdir(parents=True, exist_ok=True)
    model = model_new(device)
    opt = torch.optim.Adam(model.parameters(), lr=lr)
    latest = directory / "latest.pt"
    epoch = offset = steps = 0
    loss_total = 0.0
    history = []
    if latest.exists():
        saved = torch.load(latest, map_location="cpu", weights_only=False)
        model.load_state_dict(saved["model"])
        opt.load_state_dict(saved["optimizer"])
        epoch, offset, steps = saved["epoch"], saved["offset"], saved["steps"]
        loss_total, history = saved["loss_total"], saved["history"]
        torch.set_rng_state(saved["torch_rng"])
        torch.mps.set_rng_state(saved["mps_rng"])
    initial = {k: v.detach().cpu().clone() for k, v in model_new(device).state_dict().items()}
    xx = torch.from_numpy(x).to(device)
    yy = torch.from_numpy(y).to(device)
    mm = torch.from_numpy(mask).to(device)

    def save():
        checkpoint(latest, {"model": {k: v.detach().cpu() for k, v in model.state_dict().items()},
            "optimizer": opt.state_dict(), "epoch": epoch, "offset": offset, "steps": steps,
            "loss_total": loss_total, "history": history, "torch_rng": torch.get_rng_state(),
            "mps_rng": torch.mps.get_rng_state(), "seed": SEED})
        save_json(directory / "history.json", history)

    model.train()
    while epoch < epochs:
        # epochと固定seedだけから巡回順を作り、教師間と中断再開で順序をそろえる。
        order = np.random.default_rng(SEED + epoch).permutation(len(x))
        while offset < len(order):
            if time.monotonic() >= job.deadline:
                save()
                raise TimeoutError("registered_time_limit")
            stop = min(offset + batch_size, len(order))
            ids = torch.from_numpy(order[offset:stop].copy()).to(device)
            output = model(xx[ids]).squeeze(-1)
            loss = pair_loss(output, yy[ids], mm[ids])
            value = float(loss.item())
            if not math.isfinite(value):
                raise ValueError("nonfinite loss")
            opt.zero_grad(set_to_none=True)
            loss.backward()
            if steps == 0:
                assert all(torch.isfinite(p.grad).all().item() for p in model.parameters() if p.grad is not None)
            opt.step()
            loss_total += value * (stop - offset)
            steps += 1
            offset = stop
        epoch += 1
        history.append({"epoch": epoch, "all_group_training_loss": loss_total / len(x)})
        offset, loss_total = 0, 0.0
        if epoch % save_every == 0 or epoch == epochs:
            save()
            job.status("training", model=name, epoch=epoch, total_epochs=epochs, steps=steps)
            print(json.dumps({"model": name, **history[-1]}), flush=True)
    assert any(not torch.equal(v.detach().cpu(), initial[k]) for k, v in model.state_dict().items())
    # 完了済みの保存物から推論し、破損や保存対象の取り違えを検出する。
    loaded = model_new(device)
    loaded.load_state_dict(torch.load(latest, map_location="cpu", weights_only=False)["model"])
    np.testing.assert_array_equal(prediction(model, x[:4], device), prediction(loaded, x[:4], device))
    save_json(directory / "finished.json", {"finished_at": utc_now(), "epochs": epoch, "steps": steps,
              "final_training_loss": history[-1]["all_group_training_loss"], "checkpoint_reload": True})
    return loaded


def tiny_diagnostics(data, job, device):
    idx = data["tiny"]
    x, mask = data["x"][idx], data["mask"][idx]
    control_y = x[:, :, 10]
    control = train_one(job, "control_size", x, control_y, mask, device, 400, len(idx), .005, 100)
    control_stats = pair_stats(prediction(control, x, device), control_y, mask)
    save_json(job.output / "control.json", control_stats)
    if control_stats["pair_accuracy"] is None or control_stats["pair_accuracy"] < .99:
        raise ValueError("known-feature positive control failed; comparison not started")
    result = {}
    for t, name in enumerate(TARGETS):
        y = data["targets"][t, idx]
        model = train_one(job, "tiny_" + name, x, y, mask, device, 2000, len(idx), .005, 250)
        pred = prediction(model, x, device)
        measured = pair_stats(pred, y, mask)
        choice = np.where(mask, pred, np.inf).argmin(1)
        measured["selected_gap_to_observed_min"] = float(np.mean(y[np.arange(len(y)), choice] - np.where(mask, y, np.inf).min(1)))
        # 完全に同じ特徴への矛盾した教師は、どの決定的モデルでも分けられない。
        identical = (x[:, :, None, :] == x[:, None, :, :]).all(3)
        different = (y[:, :, None] < y[:, None, :]) & mask[:, :, None] & mask[:, None, :]
        measured["identical_feature_differing_pairs"] = int((identical & different).sum())
        measured["fit_95_percent"] = measured["pair_accuracy"] >= .95
        result[name] = measured
        save_json(job.output / "tiny.json", result)
    return result


def aggregate_inputs(values, meta):
    unique, inverse = np.unique(meta[:, 0], return_inverse=True)
    result = np.zeros((len(unique), values.shape[1]), np.float64)
    np.add.at(result, inverse, values)
    result /= np.bincount(inverse)[:, None]
    high = np.bincount(inverse, weights=meta[:, 2]) / np.bincount(inverse) >= 80
    return unique, high, result


def bootstrap_interval(differences, high):
    # 局面や候補を独立とみなさず、入力ごとにまとめてからMの層内で復元抽出する。
    rng = np.random.default_rng(78002)
    group_ids = [np.flatnonzero(high == flag) for flag in (False, True)]
    replicates = []
    for _ in range(20):
        ids = np.concatenate([rng.choice(ids, (100, len(ids)), replace=True) for ids in group_ids], axis=1)
        replicates.append(differences[ids].mean(1))
    draws = np.concatenate(replicates)
    return np.percentile(draws, [2.5, 97.5], axis=0).T


def evaluate(data, predictions, job):
    meta, mask, y, measures = data["meta"], data["mask"], data["targets"], data["measures"]
    baseline = np.zeros(len(meta), np.int64)
    selected = {name: np.where(mask, pred, np.inf).argmin(1) for name, pred in predictions.items()}
    selected["baseline"] = baseline
    report = {"models": {}, "comparison": {}, "preregistered_diagnostic_pass": []}
    validation_arrays = {}
    for split, condition in (("train", meta[:, 1] == 0), ("validation", meta[:, 1] == 1)):
        job.check_time()
        ids = np.flatnonzero(condition)
        job.status("evaluation", split=split)
        for name, choice in selected.items():
            job.check_time()
            values = measures[ids, choice[ids]]
            unique, high, by_input = aggregate_inputs(values, meta[ids])
            own_y = y[:, ids, choice[ids]].T
            _, _, target_by_input = aggregate_inputs(own_y, meta[ids])
            summary = {"inputs": len(unique), "selected": dict(zip(MEASURES, map(float, by_input.mean(0))))}
            for group, keep in (("M_lt_80", ~high), ("M_ge_80", high)):
                summary[group] = {"inputs": int(keep.sum()), "selected": dict(zip(MEASURES, map(float, by_input[keep].mean(0))))}
            pred = data["ranks"][ids].astype(np.float32) if name == "baseline" else predictions[name][ids]
            summary["targets"] = {}
            for t, target in enumerate(TARGETS):
                stat = pair_stats(pred, y[t, ids], mask[ids])
                stat["selected_mean"] = float(target_by_input[:, t].mean())
                oracle = np.where(mask[ids], y[t, ids], np.inf).min(1)
                _, _, oracle_i = aggregate_inputs(oracle[:, None], meta[ids])
                stat["observed_min_mean"] = float(oracle_i.mean())
                stat["gap_to_observed_min"] = float((target_by_input[:, t] - oracle_i[:, 0]).mean())
                for group, keep in (("M_lt_80", meta[ids, 2] < 80), ("M_ge_80", meta[ids, 2] >= 80)):
                    stat[group] = pair_stats(pred[keep], y[t, ids][keep], mask[ids][keep])
                summary["targets"][target] = stat
            report["models"].setdefault(name, {})[split] = summary
            if split == "validation":
                validation_arrays[name] = np.concatenate((by_input, target_by_input), axis=1)
        save_json(job.output / "summary.partial.json", report)
    names = list(MEASURES) + ["target_" + t for t in TARGETS]
    for name in TARGETS:
        job.check_time()
        report["comparison"][name] = {}
        for reference in ("baseline", "rollout"):
            delta = validation_arrays[name] - validation_arrays[reference]
            interval = bootstrap_interval(delta, high)
            comparison = {k: {"delta": float(delta[:, i].mean()), "ci95": interval[i].tolist(),
                              "M_lt_80_delta": float(delta[~high, i].mean()),
                              "M_ge_80_delta": float(delta[high, i].mean())} for i, k in enumerate(names)}
            report["comparison"][name][reference] = comparison
    for target in ("immediate", "changed"):
        accuracy = report["models"][target]["validation"]["targets"][target]["pair_accuracy"]
        better = all(report["comparison"][target][reference]["target_" + target]["ci95"][1] < 0
                     for reference in ("baseline", "rollout"))
        if accuracy >= .55 and better:
            report["preregistered_diagnostic_pass"].append(target)
    np.savez(job.output / "validation_per_input.npz", input_ids=unique, M_ge_80=high,
             measure_names=np.array(names), **validation_arrays)
    report["finished_at"] = utc_now()
    save_json(job.output / "summary.json", report)
    lines = ["# v078 教師比較の結果", "", "固定120 epoch後の診断用2,048入力。採取済み候補上の比較であり、探索全体の評価は未実施。", "",
             "| 教師 | 自分の教師の候補対正解率 | 直後手数差 | 有効変更率の差 | 試行時間差(ms) | 30 ms後の手数差 |",
             "|---|---:|---:|---:|---:|---:|"]
    for name in TARGETS:
        accuracy = report["models"][name]["validation"]["targets"][name]["pair_accuracy"]
        diffs = report["comparison"][name]["baseline"]
        lines.append(f"| {name} | {accuracy:.3%} | {diffs['first_T']['delta']:+.6f} | {diffs['changed_nonworse']['delta']:+.6f} | {diffs['first_ms']['delta']:+.6f} | {diffs['rollout_T']['delta']:+.6f} |")
    passed = ", ".join(report["preregistered_diagnostic_pass"]) or "なし"
    lines += ["", "事前登録した診断基準の通過: " + passed,
              "", "候補対正解率の対象は教師ごとに異なる。横並びの選択差、M別の結果、入力単位の区間はsummary.jsonを参照する。",
              "時間教師は早い失敗も高く評価するため、単独の採用候補にはしない。最終評価用100入力は未使用。"]
    (job.output / "REPORT.md").write_text("\n".join(lines) + "\n")
    return report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--seconds", type=int, default=3600)
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    torch.set_num_threads(30)
    torch.set_num_interop_threads(1)
    if not torch.backends.mps.is_available():
        raise RuntimeError("MPS required; no automatic CPU substitution")
    device = torch.device("mps")
    checks = self_test(device)
    if args.self_test:
        print(json.dumps(checks))
        return
    if args.source is None or args.output is None or not 0 < args.seconds <= 3600:
        parser.error("source/output and time limit in (0, 3600] required")
    source, output = args.source.resolve(), args.output.resolve()
    if not args.resume:
        output.mkdir(parents=True, exist_ok=False)
    elif not output.is_dir():
        raise FileNotFoundError(output)
    lock = output / "running.lock"
    fd = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    os.write(fd, str(os.getpid()).encode())
    os.close(fd)
    job = Run(output, args.seconds)
    try:
        script_hash = digest(Path(__file__).read_bytes())
        helper_hash = digest(Path(__file__).with_name("train_v077_rank.py").read_bytes())
        config_path = output / "config.json"
        if args.resume:
            previous = json.loads(config_path.read_text())
            assert previous["script_sha256"] == script_hash and previous["helper_sha256"] == helper_hash
            assert previous["source"] == str(source)
            if (output / "summary.json").exists():
                raise ValueError("run already completed")
        else:
            save_json(config_path, {"created_at": utc_now(), "source": str(source), "seed": SEED,
                "script_sha256": script_hash, "helper_sha256": helper_hash, "epochs": 120,
                "batch_size": 256, "lr": .001, "targets": TARGETS, "seconds": args.seconds,
                "python": sys.version, "torch": torch.__version__, "numpy": np.__version__,
                "device": str(device), "platform": platform.platform(), "cpu_threads": 30})
            frozen = output / "frozen"
            frozen.mkdir()
            for filename in ("train_v078_targets.py", "train_v077_rank.py"):
                (frozen / filename).write_bytes(Path(__file__).with_name(filename).read_bytes())
        save_json(output / "mechanism_checks.json", checks)
        job.status("loading", completed_inputs=0, total_inputs=8192)
        data, description = load_dataset(source, job)
        tiny_diagnostics(data, job, device)
        training = data["meta"][:, 1] == 0
        predictions = {}
        for t, name in enumerate(TARGETS):
            model = train_one(job, name, data["x"][training], data["targets"][t, training],
                              data["mask"][training], device, 120, 256, .001, 5)
            predictions[name] = prediction(model, data["x"], device)
            np.save(output / f"prediction_{name}.npy", predictions[name])
        job.check_time()
        evaluate(data, predictions, job)
        job.status("finished", completed_models=list(TARGETS))
        save_json(output / "exit.json", {"reason": "registered_comparison_finished", "exit_code": 0, "time": utc_now()})
    except TimeoutError:
        job.status("paused", reason="registered_time_limit", resumable=True)
        save_json(output / "exit.json", {"reason": "registered_time_limit", "exit_code": 75, "time": utc_now()})
        sys.exit(75)
    except BaseException as error:
        job.status("failed", error=str(error))
        save_json(output / "exit.json", {"reason": type(error).__name__, "error": str(error), "exit_code": 1, "time": utc_now()})
        traceback.print_exc()
        sys.exit(1)
    finally:
        lock.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
