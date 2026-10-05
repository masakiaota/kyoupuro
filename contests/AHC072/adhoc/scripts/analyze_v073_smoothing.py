#!/usr/bin/env python3
"""保存済みの係数別集計を平滑化する。solver は実行しない。"""

import csv
import json
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np


ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "results/analysis/v073/smoothing"
BANDWIDTHS = (0.125, 0.25, 0.375)


def smooth(x, y, queries, bandwidth):
    weights = np.exp(-0.5 * ((queries[:, None] - x) / bandwidth) ** 2)
    return weights @ y / weights.sum(axis=1)


def main():
    with (ROOT / "results/tuning/v073/trials.csv").open() as stream:
        rows = sorted(csv.DictReader(stream), key=lambda row: float(row["repair_weight"]))
    x = np.array([float(row["repair_weight"]) for row in rows])
    y = np.array([int(row["total_sum"]) for row in rows])
    assert len(x) == 21 and np.allclose(np.diff(x), 0.125)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    grid = np.linspace(0.0, 2.5, 2501)
    summary = {"source": "results/tuning/v073/trials.csv", "solver_executions": 0}
    curves = {}
    gaussian = []
    for bandwidth in BANDWIDTHS:
        curve = smooth(x, y, grid, bandwidth)
        curves[bandwidth] = curve
        values = smooth(x, y, np.array([0.875, 1.0]), bandwidth)
        gaussian.append({
            "bandwidth": bandwidth,
            "minimum_weight": float(grid[curve.argmin()]),
            "at_0.875": float(values[0]),
            "at_1.0": float(values[1]),
            "difference": float(values[0] - values[1]),
        })
    summary["gaussian"] = gaussian
    summary["moving_average"] = []
    for width in (3, 5, 7):
        radius = width // 2
        means = np.convolve(y, np.ones(width) / width, mode="valid")
        centers = x[radius:-radius]
        selected = float(means[centers == 0.875][0])
        baseline = float(means[centers == 1.0][0])
        summary["moving_average"].append({
            "points": width, "minimum_weight": float(centers[means.argmin()]),
            "at_0.875": selected, "at_1.0": baseline,
            "difference": selected - baseline,
        })
    # 最良の一点だけが谷を作っていないか、保存値を一つ除いた場合も記録する。
    summary["omit_one_point"] = []
    for omitted in (0.875, 0.5):
        keep = x != omitted
        for bandwidth in BANDWIDTHS:
            curve = smooth(x[keep], y[keep], grid, bandwidth)
            values = smooth(x[keep], y[keep], np.array([0.875, 1.0]), bandwidth)
            summary["omit_one_point"].append({
                "omitted": omitted, "bandwidth": bandwidth,
                "minimum_weight": float(grid[curve.argmin()]),
                "difference": float(values[0] - values[1]),
            })
    (OUTPUT / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    with (OUTPUT / "curves.csv").open("w", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(["repair_weight", *[f"gaussian_width_{b}" for b in BANDWIDTHS]])
        writer.writerows(zip(grid, *[curves[b] for b in BANDWIDTHS]))

    plt.rcParams.update({"font.family": "Hiragino Sans", "font.size": 11, "axes.unicode_minus": False})
    fig, ax = plt.subplots(figsize=(10, 4.9), layout="constrained")
    ax.scatter(x, y, color="#475569", s=30, zorder=3, label="各係数の実測値")
    for bandwidth, color in zip(BANDWIDTHS, ("#9b59b6", "#07856c", "#d47b14")):
        ax.plot(grid, curves[bandwidth], color=color, linewidth=2,
                label=f"近傍の加重平均：幅 {bandwidth}")
    ax.axvline(1.0, color="#94a3b8", linestyle=":", linewidth=1.4)
    ax.scatter([0.875], [y[x == 0.875][0]], s=85, facecolors="white", edgecolors="#d13b48", zorder=4)
    ax.annotate("0.875：実測の最小", xy=(0.875, y[x == 0.875][0]), xytext=(1.2, 20942),
                color="#a42a35", arrowprops={"arrowstyle": "->", "color": "#a42a35"})
    ax.set(title="v073：0.875付近は低いが、1.0との差は小さい",
           xlabel="修復負担の減点係数", ylabel="100件の合計手数（小さいほどよい）",
           xlim=(-0.04, 2.54), ylim=(20920, 21150))
    ax.set_xticks(np.arange(0, 2.501, 0.25))
    ax.ticklabel_format(axis="y", style="plain", useOffset=False)
    ax.grid(alpha=0.15)
    ax.legend(loc="upper left", fontsize=9)
    for extension in ("png", "svg"):
        fig.savefig(OUTPUT / f"parameter_curve.{extension}", dpi=170)
    plt.close(fig)

    lines = [
        "# v073：保存結果の平滑化", "",
        "新規100件を使った21候補の保存済み集計だけを分析した。solverの追加実行はない。", "",
        "近い係数ほど強く重みを付けて平均するガウス平滑化を用いた。幅は係数の単位での標準偏差で、格子間隔の1倍・2倍・3倍を比較した。曲線は傾向を見るためのもので、未測定の係数のスコアを検証したものではない。", "",
        "| 平滑化の幅 | 曲線の最小付近 | 0.875と1.0の差（100件合計） |",
        "| ---: | ---: | ---: |",
        *[f"| {r['bandwidth']} | {r['minimum_weight']:.2f} | {r['difference']:+.1f}手 |" for r in gaussian], "",
        "0.875は低い範囲に含まれる。ただし1.0との差は平滑化すると約2〜10手に縮む。単純な3・5・7点移動平均では最小の中心が1.0・0.75・0.75に変わる。0.875という一点の最適性や、観測された61手が再現することまでは支持しない。", "",
        "0.875の観測点を除いた診断では、幅0.25・0.375の最小は0.66・0.76付近へ動いた。良い範囲の存在と、最良点の正確な位置は区別する必要がある。幅は結果を見た後の分析であり、正式な採否基準は変更しない。", "",
        "候補選択が切り替わるため、係数を少し変えても探索経路は不連続に変わり得る。各係数1回の測定から、実行時変動と係数の効果を完全には分離できない。", "",
        "![係数とスコアの傾向](parameter_curve.png)", "",
        "再集計：adhoc/scripts/analyze_v073_smoothing.py。数値はsummary.json、曲線はcurves.csvに保存した。", "",
    ]
    (OUTPUT / "report.md").write_text("\n".join(lines))
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    print(OUTPUT / "parameter_curve.png")


if __name__ == "__main__":
    main()
