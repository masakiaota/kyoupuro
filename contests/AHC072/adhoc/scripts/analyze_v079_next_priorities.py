#!/usr/bin/env python3
"""保存済みv078/v079から候補選択の取りこぼしを集計する。solver・学習は実行しない。"""
import hashlib
import json
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
TRAIN = ROOT / "results/nn_rank/v078/20261002T095446_studio"
EVAL = ROOT / "results/nn_rank/v079/20261002T104039_studio"
OUTPUT = ROOT / "results/analysis/v079/20261002_next_priorities/facts.json"


def main():
    with np.load(TRAIN / "dataset.npz") as data:
        meta, mask, targets, measures = (data[k] for k in ("meta", "mask", "targets", "measures"))
    prediction = np.load(TRAIN / "prediction_immediate.npy")
    selected = np.where(mask, prediction, np.inf).argmin(1)
    observed_best = np.where(mask, targets[1], np.inf).argmin(1)
    shorter = (measures[:, :, 5] > 0) & mask
    validation = np.flatnonzero(meta[:, 1] == 1)
    # 各入力4局面なので、ここでの局面平均は既存集計の入力平均と一致する。
    assert np.all(np.unique(meta[validation, 0], return_counts=True)[1] == 4)
    groups = {}
    for name, ids in (("all", validation), ("M_lt_80", validation[meta[validation, 2] < 80]),
                      ("M_ge_80", validation[meta[validation, 2] >= 80])):
        available = shorter[ids].any(1)
        chosen_shorter = shorter[ids, selected[ids]]
        baseline = targets[1, ids, 0]
        chosen = targets[1, ids, selected[ids]]
        minimum = targets[1, ids, observed_best[ids]]
        groups[name] = {
            "states": len(ids), "inputs": len(np.unique(meta[ids, 0])),
            "has_shorter_fraction": float(available.mean()),
            "baseline_shorter_fraction": float(shorter[ids, 0].mean()),
            "model_shorter_fraction": float(chosen_shorter.mean()),
            "model_capture_when_available": float(chosen_shorter.sum() / available.sum()),
            "baseline_gap_to_observed_min": float((baseline - minimum).mean()),
            "model_gap_to_observed_min": float((chosen - minimum).mean()),
            "fraction_of_observed_gap_closed": float((baseline - chosen).sum() / (baseline - minimum).sum()),
        }
    summary = json.loads((TRAIN / "summary.json").read_text())
    evaluation = json.loads((EVAL / "comparison.json").read_text())
    result = {
        "solver_runs": 0, "training_runs": 0, "groups": groups,
        "immediate_pair_accuracy": {split: summary["models"]["immediate"][split]["targets"]["immediate"]["pair_accuracy"]
                                    for split in ("train", "validation")},
        "validation_tied_fraction": summary["models"]["immediate"]["validation"]["targets"]["immediate"]["tied_group_fraction"],
        "whole_search_goal": {"baseline": "v076", "mean_reduction_goal": 5, "current_mean_reduction": -evaluation["mean_delta_T"],
                              "high_M_mean_reduction_needed_if_low_M_unchanged": (5*200-24)/86},
        "limits": ["v076の保存局面と、その局面で試した最大32候補の記録に限る。",
                   "候補の観測最良値は学習版が訪れる局面や、探索全体の性能上限を示さない。",
                   "学習用と検証用の正解率だけでは、特徴・容量・最適化・未観測要因を分離できない。"],
        "source_sha256": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in
                          (TRAIN / "dataset.npz", TRAIN / "prediction_immediate.npy", TRAIN / "summary.json", EVAL / "comparison.json")},
    }
    assert abs(groups["all"]["model_gap_to_observed_min"] - summary["models"]["immediate"]["validation"]["targets"]["immediate"]["gap_to_observed_min"]) < 1e-7
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "source_sha256"}, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
