#!/usr/bin/env python3
"""保存済みのv073〜v075を絶対スコアと擬似相対スコアで比較する。"""
import csv
import hashlib
import json
import os
from pathlib import Path

os.environ["OPENBLAS_NUM_THREADS"] = "1"
os.environ["OMP_NUM_THREADS"] = "1"
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "results/analysis/tuning_synthesis_20260930"
VERSIONS = ("v073", "v074", "v075")


def write_csv(name, rows):
    with (OUT / name).open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def load(version):
    folder = ROOT / "results/tuning" / version
    key = "tick" if version == "v073" else "config_id"
    rows = json.loads((folder / "trials.json").read_text())
    records = [json.loads(line) for line in (folder / "cases.jsonl").read_text().splitlines()]
    cases = sorted({r["case_name"] for r in records})
    lookup = {(r[key], r["case_name"]): r for r in records}
    assert len(cases) == 100 and len(lookup) == len(records) == len(rows) * 100
    assert all(r["status"] == "ok" for r in records)
    scores = np.array([[lookup[r[key], c]["score"] for c in cases] for r in rows], dtype=np.int64)
    assert np.all(scores > 0)
    for row, values in zip(rows, scores):
        assert int(values.sum()) == row["total_sum"]
        assert max(lookup[row[key], c]["elapsed"] for c in cases) == row["max_elapsed_ms"]
    eligible = np.array([r["eligible"] for r in rows])
    minima = scores[eligible].min(axis=0)
    # 正の整数に対するround(10^9 * MIN / YOUR)。MINは同じ入力の適格条件内で固定する。
    relative_cases = (2 * 10**9 * minima + scores) // (2 * scores)
    relative = relative_cases.mean(axis=1) / 10**7  # 100点満点へ換算
    base = scores[0]
    metrics = []
    for i, row in enumerate(rows):
        delta = scores[i] - base
        metrics.append(dict(version=version, config_id=row[key], trial=row["trial"],
            eligible=row["eligible"], absolute_sum=int(scores[i].sum()),
            absolute_delta=int(delta.sum()), absolute_delta_percent=float(delta.sum() / base.sum() * 100),
            pseudo_relative_percent=float(relative[i]),
            pseudo_relative_delta_pp=float(relative[i] - relative[0]),
            baseline_ratio_gain_percent=float(np.mean(base / scores[i] - 1) * 100),
            case_improved=int((delta < 0).sum()), case_tied=int((delta == 0).sum()),
            case_worsened=int((delta > 0).sum()), max_elapsed_ms=row["max_elapsed_ms"]))
    write_csv(version + "_objectives.csv", metrics)
    manifest = json.loads((folder / "input_manifest.json").read_text())
    input_hashes = {r["sha256"] for r in manifest["cases"]}
    assert len(input_hashes) == 100
    saved_hashes = {name: hashlib.sha256((folder / name).read_bytes()).hexdigest()
                    for name in ("trials.json", "cases.jsonl", "plan.json", "input_manifest.json")}
    return dict(rows=rows, key=key, scores=scores, relative_cases=relative_cases,
                relative=relative, eligible=eligible, metrics=metrics,
                input_hashes=input_hashes, saved_hashes=saved_hashes)


def adjusted_effects(data):
    parameters = json.loads((ROOT / "results/tuning/v074/plan.json").read_text())["parameters"]
    defaults = {p["name"]: p["default"] for p in parameters}
    rows, eligible = data["rows"], data["eligible"]

    def feature(row, kind):
        result = [1.0]
        for p in parameters:
            value = row[p["name"]]
            x = (value - p["default"]) / (max(p["values"]) - min(p["values"]))
            if kind == "categorical":
                result.extend(float(value == v) for v in p["values"] if v != p["default"])
            else:
                result.append(x)
                if kind == "quadratic":
                    result.append(x * x)
        return np.array(result)

    effects, fits = [], []
    for objective, y in (("absolute_sum", data["scores"].sum(axis=1)),
                         ("pseudo_relative_percent", data["relative"])):
        for kind in ("linear", "quadratic", "categorical"):
            X = np.array([feature(r, kind) for r in rows])[eligible]
            coefficients = np.linalg.lstsq(X, y[eligible], rcond=None)[0]
            errors = y[eligible] - X @ coefficients
            leverage = np.sum(X * (X @ np.linalg.pinv(X.T @ X)), axis=1)
            fits.append(dict(objective=objective, model=kind, configurations=int(eligible.sum()),
                leave_one_configuration_out_rmse=float(np.sqrt(np.mean((errors / (1 - leverage)) ** 2)))))
            for p in parameters:
                for value in p["values"]:
                    effect = (feature({**defaults, p["name"]: value}, kind) - feature(defaults, kind)) @ coefficients
                    effects.append(dict(objective=objective, model=kind, parameter=p["name"],
                                        value=value, adjusted_delta=float(effect)))
    write_csv("v074_adjusted_effects.csv", effects)
    return fits


def smooth_repair(data):
    x = np.array([r["repair_weight"] for r in data["rows"]])
    grid = np.linspace(0, 2.5, 2501)
    result = []
    for width in (0.125, 0.25, 0.375):
        weights = np.exp(-0.5 * ((grid[:, None] - x) / width) ** 2)
        for objective, y in (("absolute_sum", data["scores"].sum(axis=1)),
                             ("pseudo_relative_percent", data["relative"])):
            curve = weights @ y / weights.sum(axis=1)
            index = np.argmin(curve) if objective == "absolute_sum" else np.argmax(curve)
            result.append(dict(objective=objective, width=width, optimum_on_smoothed_curve=float(grid[index])))
    return result


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    data = {version: load(version) for version in VERSIONS}
    for i, a in enumerate(VERSIONS):
        for b in VERSIONS[i + 1:]:
            assert not data[a]["input_hashes"] & data[b]["input_hashes"]
    summary = dict(solver_executions=0, records=9200,
        relative_reference="同一実験の時間条件内の設定からケースごとに最小スコアを選ぶ。別実験とは比較しない。",
        limitations=["公式の他参加者のMINは不明。擬似相対は補助指標である。",
                     "各条件1回。相互作用と実行時変動を分離できない。",
                     "回帰は加算モデル。未評価の組み合わせの改善を保証しない。"],
        fits=adjusted_effects(data["v074"]), repair_smoothing=smooth_repair(data["v073"]),
        saved_hashes={v: d["saved_hashes"] for v, d in data.items()})
    # 対応する100ケースを再抽出する感度分析。solverの再評価はしない。
    weights = np.random.default_rng(7502).multinomial(100, np.full(100, 0.01), size=2000)
    contrasts = []
    for version, ids in (("v074", (4, 7, 15, 18, 23)), ("v075", (5,))):
        d = data[version]
        for config_id in ids:
            i = next(i for i, r in enumerate(d["rows"]) if r[d["key"]] == config_id)
            absolute = d["scores"][i] - d["scores"][0]
            relative = (d["relative_cases"][i] - d["relative_cases"][0]) / 10**7
            contrasts.append(dict(version=version, config_id=config_id,
                absolute_delta=int(absolute.sum()),
                absolute_case_resampling_95_range=np.percentile(weights @ absolute, (2.5, 97.5)).tolist(),
                relative_delta_pp=float(relative.mean()),
                relative_case_resampling_95_range=np.percentile(weights @ relative / 100, (2.5, 97.5)).tolist()))
    summary["case_resampling"] = dict(seed=7502, samples=2000, contrasts=contrasts,
        limitation="入力構成への感度だけを表す。実行時変動と最良条件を選んだ偏りを含まない。")
    summary["observed_best"] = {}
    for version, d in data.items():
        valid = [m for m in d["metrics"] if m["eligible"]]
        summary["observed_best"][version] = dict(
            absolute=min(valid, key=lambda r: r["absolute_sum"]),
            pseudo_relative=max(valid, key=lambda r: r["pseudo_relative_percent"]))
    (OUT / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({k: summary[k] for k in ("records", "repair_smoothing", "fits", "case_resampling")},
                     ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
