#!/usr/bin/env python3
"""保存済み8試行から、候補差と試行ノイズを分ける。solverは実行しない。"""
import argparse
import json
from pathlib import Path

import numpy as np

from v083_data import comparison, load, save, sha


def agreement(a, b, mask):
    va, vb, cov, concordance = [], [], [], []
    same_winner = []
    for x, y, m in zip(a, b, mask):
        x, y = x[m].astype(np.float64), y[m].astype(np.float64)
        if len(x) < 2:
            continue
        xx, yy = x-x.mean(), y-y.mean()
        va.append(float(xx@xx/(len(x)-1)))
        vb.append(float(yy@yy/(len(y)-1)))
        cov.append(float(xx@yy/(len(x)-1)))
        i, j = np.triu_indices(len(x), 1)
        dx, dy = np.sign(x[i]-x[j]), np.sign(y[i]-y[j])
        informative = (dx != 0) & (dy != 0)
        if informative.any():
            concordance.append(float((dx[informative] == dy[informative]).mean()))
        same_winner.append(bool(y[x.argmin()] == y.min()))
    va, vb, cov = map(np.mean, (va, vb, cov))
    return {"half_a_candidate_variance": float(va), "half_b_candidate_variance": float(vb),
            "cross_half_candidate_covariance": float(cov),
            "estimated_half_mean_noise_variance": float((va+vb)/2-cov),
            "pooled_half_correlation": float(cov/np.sqrt(va*vb)) if va*vb > 0 else None,
            "mean_pair_concordance_excluding_ties": float(np.mean(concordance)) if concordance else None,
            "informative_pair_groups": len(concordance),
            "selected_in_other_half_best_tie_fraction": float(np.mean(same_winner))}


def analyze(data):
    valid = data["mask"].any(1)
    mask, meta = data["mask"][valid], data["meta"][valid]
    baseline = data["baseline"][valid]
    group = np.arange(len(mask))
    result = {"groups": len(mask), "targets": {}, "diagnostic_only": True}
    future = data["future"][valid]
    for name in ("immediate", "first_best", "future"):
        y = data[name][valid]
        half = (y[:, :, :4].mean(2), y[:, :, 4:].mean(2))
        report = {"agreement": agreement(*half, mask), "splits": {}}
        for label, selector, evaluation in (("forward", 0, 1), ("reverse", 1, 0)):
            selected = np.where(mask, half[selector], np.inf).argmin(1)
            evaluation_replicas = slice(0, 4) if evaluation == 0 else slice(4, 8)
            future_test = future[:, :, evaluation_replicas].mean(2)
            report["splits"][label] = {
                "same_target_vs_v079": comparison(half[evaluation][group, selected]-half[evaluation][group, baseline], meta),
                "future_vs_v079": comparison(future_test[group, selected]-future_test[group, baseline], meta),
                "in_selection_half_vs_v079": comparison(half[selector][group, selected]-half[selector][group, baseline], meta)}
        report["by_M"] = {}
        for label, select in (("lt80", meta[:, 2] < 80), ("ge80", meta[:, 2] >= 80)):
            report["by_M"][label] = agreement(half[0][select], half[1][select], mask[select])
        result["targets"][name] = report
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", type=Path, required=True)
    args = parser.parse_args()
    directory = args.run / "pilot_data"
    meta = json.loads((directory / "dataset.json").read_text())
    for name, digest in meta["files"].items():
        assert sha(directory / f"{name}.npy") == digest
    result = analyze(load(directory))
    result["dataset_sha256"] = sha(directory / "dataset.json")
    save(args.run / "reliability.json", result)
    # コンソールには主要値だけを出し、入力別差分はJSONへ保存する。
    print(json.dumps({name: {"agreement": r["agreement"],
          "forward_same_target_mean": r["splits"]["forward"]["same_target_vs_v079"]["mean"],
          "forward_same_target_ci95": r["splits"]["forward"]["same_target_vs_v079"]["ci95"],
          "forward_future_mean": r["splits"]["forward"]["future_vs_v079"]["mean"],
          "reverse_same_target_mean": r["splits"]["reverse"]["same_target_vs_v079"]["mean"]}
          for name, r in result["targets"].items()}, indent=2))


if __name__ == "__main__":
    main()
