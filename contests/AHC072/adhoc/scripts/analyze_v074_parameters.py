#!/usr/bin/env python3
"""v074の保存結果から6項目の傾向を調べる。solverは実行しない。"""

import csv
import hashlib
import json
import os
from pathlib import Path

os.environ["OPENBLAS_NUM_THREADS"] = "1"
os.environ["OMP_NUM_THREADS"] = "1"
import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT / "results/tuning/v074"
OUT = ROOT / "results/analysis/v074/parameter_review"
TITLES = {
    "removal_size_exponent": "候補サイズの指数",
    "priority_noise_amplitude": "順位に掛ける乱数の振幅",
    "lns_start_temperature": "焼きなましの開始温度",
    "lns_end_temperature": "焼きなましの終了温度",
    "dependency_interval": "依存拡張を試す間隔",
    "regular_cooldown_fraction": "再試行待ちの係数",
}


def main():
    plan = json.loads((RUN / "plan.json").read_text())
    parameters = plan["parameters"]
    defaults = {p["name"]: p["default"] for p in parameters}
    with (RUN / "trials.csv").open() as stream:
        rows = list(csv.DictReader(stream))
    for row in rows:
        for key in ("config_id", "trial", "total_sum", "max_elapsed_ms", "baseline_delta"):
            row[key] = int(row[key])
        row["eligible"] = row["eligible"] == "True"
        for p in parameters:
            row[p["name"]] = float(row[p["name"]])
    by_id = {r["config_id"]: r for r in rows}
    assert len(by_id) == 65 and by_id[0]["total_sum"] == 23804
    with (RUN / "single_parameter_curves.csv").open() as stream:
        singles = list(csv.DictReader(stream))
    baseline = by_id[0]["total_sum"]
    OUT.mkdir(parents=True, exist_ok=True)

    def feature(row, kind, order=False):
        result = [1.0]
        for p in parameters:
            value = float(row[p["name"]])
            normalized = (value - p["default"]) / (max(p["values"]) - min(p["values"]))
            if kind == "categorical":
                result.extend(float(value == v) for v in p["values"] if v != p["default"])
            else:
                result.append(normalized)
                if kind == "quadratic":
                    result.append(normalized * normalized)
        if order:
            result.append((row["trial"] - 33) / 64)
        return np.array(result)

    def fit(data, kind, order=False):
        X = np.array([feature(r, kind, order) for r in data])
        y = np.array([r["total_sum"] - baseline for r in data])
        coefficients = np.linalg.lstsq(X, y, rcond=None)[0]
        residuals = y - X @ coefficients
        leverage = np.sum(X * (X @ np.linalg.pinv(X.T @ X)), axis=1)
        return dict(rows=len(data), columns=X.shape[1], rank=int(np.linalg.matrix_rank(X)),
                    rmse=float(np.sqrt(np.mean(residuals ** 2))),
                    leave_one_configuration_out_rmse=float(np.sqrt(np.mean((residuals / (1 - leverage)) ** 2))),
                    coefficients=coefficients.tolist())

    # 3種類の加算モデルで、仮定を変えても方向が一致するかを調べる。
    # 条件間の相互作用は表現しないため、予測値を新しい実測値として扱わない。
    fits, effects = {}, []
    eligible = [r for r in rows if r["eligible"]]
    for group, data in (("eligible", eligible), ("all", rows)):
        for kind in ("linear", "quadratic", "categorical"):
            name = group + "_" + kind
            fits[name] = fit(data, kind)
            coefficients = np.array(fits[name]["coefficients"])
            base_features = feature(defaults, kind)
            for p in parameters:
                for value in p["values"]:
                    delta = (feature({**defaults, p["name"]: value}, kind) - base_features) @ coefficients
                    effects.append(dict(model=name, parameter=p["name"], value=value, predicted_delta=float(delta)))
    fits["eligible_quadratic_with_order"] = fit(eligible, "quadratic", order=True)

    marginal = []
    for p in parameters:
        for value in p["values"]:
            group = [r for r in rows if r["config_id"] >= 25 and r[p["name"]] == value]
            assert len(group) == 8
            good = [r for r in group if r["eligible"]]
            marginal.append(dict(parameter=p["name"], value=value, count=len(group),
                mean_delta=float(np.mean([r["baseline_delta"] for r in group])),
                median_delta=float(np.median([r["baseline_delta"] for r in group])),
                eligible_count=len(good),
                eligible_mean_delta=float(np.mean([r["baseline_delta"] for r in good]))))

    records = [json.loads(line) for line in (RUN / "cases.jsonl").read_text().splitlines()]
    cases = sorted({r["case_name"] for r in records})
    lookup = {(r["config_id"], r["case_name"]): r for r in records}
    assert len(cases) == 100 and len(lookup) == len(records) == 6500
    assert all(r["status"] == "ok" for r in records)
    Y = np.array([[lookup[i, case]["score"] for case in cases] for i in range(65)])
    assert all(int(Y[i].sum()) == by_id[i]["total_sum"] for i in range(65))
    # 同じ100件の構成を変えた場合への感度。実行時変動の再測定ではない。
    weights = np.random.default_rng(7403).multinomial(100, np.full(100, 0.01), size=2000)
    contrasts = []
    for a, b in ((4, 0), (4, 1), (3, 0), (7, 5), (8, 5), (23, 21), (23, 0),
                 (24, 0), (16, 0), (11, 0), (17, 19), (18, 19)):
        delta = Y[a] - Y[b]
        contrasts.append(dict(first=a, second=b, delta=int(delta.sum()),
            case_bootstrap_95_percentile=np.percentile(weights @ delta, [2.5, 97.5]).tolist(),
            improved=int((delta < 0).sum()), tied=int((delta == 0).sum()), worsened=int((delta > 0).sum())))

    mechanism = {r["config_id"]: r["counters"] for r in
                 map(json.loads, (RUN / "mechanism.jsonl").read_text().splitlines())}
    for values in mechanism.values():
        values["dependency_completion_percent"] = 100 * values["lns_dependency_completed"] / values["lns_dependency_attempts"]
    source_files = ("plan.json", "trials.csv", "single_parameter_curves.csv", "cases.jsonl", "mechanism.jsonl")
    summary = dict(solver_executions=0, baseline_total=baseline, fits=fits,
        case_bootstrap=dict(seed=7403, samples=2000, contrasts=contrasts,
            limitation="入力ケースの構成への感度だけを調べる。実行時変動と候補選択による偏りは含まない。"),
        mechanism=mechanism,
        time_overruns=[dict(config_id=r["config_id"], case=r["case_name"], elapsed=r["elapsed"])
                       for r in records if r["elapsed"] > 2000],
        inputs_sha256={name: hashlib.sha256((RUN / name).read_bytes()).hexdigest() for name in source_files})
    (OUT / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    for name, data in (("adjusted_effects.csv", effects), ("joint_marginals.csv", marginal)):
        with (OUT / name).open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(data[0]))
            writer.writeheader()
            writer.writerows(data)

    plt.rcParams.update({"font.family": "Hiragino Sans", "font.size": 10, "axes.unicode_minus": False})
    fig, axes = plt.subplots(2, 3, figsize=(12, 8.2))
    fig.subplots_adjust(left=0.075, right=0.985, bottom=0.12, top=0.845, hspace=0.35, wspace=0.20)
    coefficients = np.array(fits["eligible_quadratic"]["coefficients"])
    base_features = feature(defaults, "quadratic")
    for axis, p in zip(axes.flat, parameters):
        data = sorted((r for r in singles if r["parameter"] == p["name"]), key=lambda r: float(r["value"]))
        xs = [float(r["value"]) for r in data]
        ys = [int(r["baseline_delta"]) for r in data]
        axis.plot(xs, ys, "o-", color="#2879ab", markersize=5, linewidth=1.3, label="1項目だけ変更した実測")
        grid = np.linspace(xs[0], xs[-1], 150)
        curve = [(feature({**defaults, p["name"]: value}, "quadratic") - base_features) @ coefficients for value in grid]
        axis.plot(grid, curve, color="#d47818", linewidth=2.1, label="他5項目も考慮した参考曲線")
        axis.axhline(0, color="#64748b", linewidth=0.8)
        axis.axvline(p["default"], color="#94a3b8", linestyle=":", linewidth=1)
        axis.set_title(TITLES[p["name"]], fontsize=11)
        axis.set_xticks(xs, [format(x, ".3g") for x in xs])
        if p["name"] == "lns_end_temperature":
            axis.tick_params(axis="x", labelrotation=25)
        axis.set_ylim(-150, 210)
        axis.grid(alpha=0.13)
    fig.suptitle("v074：6項目の傾向（修復係数は0.83固定）", fontsize=15, y=0.98)
    fig.supylabel("各項目の基準値からの差［100件合計・手］  ↓ 小さいほどよい", fontsize=11, x=0.01)
    fig.supxlabel("点は単独変更の実測。橙線は時間条件内の63設定を使った2次の加算モデル。中間値は未評価。", fontsize=10, y=0.025)
    handles, labels = axes[0, 0].get_legend_handles_labels()
    fig.legend(handles, labels, loc="upper center", bbox_to_anchor=(0.5, 0.935), ncol=2, fontsize=10, frameon=False)
    for extension in ("png", "svg"):
        fig.savefig(OUT / ("parameter_trends." + extension), dpi=170)
    plt.close(fig)

    descriptions = [
        ("候補サイズの指数", "0.8〜0.95、特に上側", "最も方向が揃う。単独変更は0.35で+136手、0.8で−19手、0.95で−102手。組み合わせの値別平均も高い側が良く、線形・2次・値別の補正の全てで増やす方向が支持された。探索上端が良いため、底の位置は未確定。"),
        ("順位の乱数幅", "0.2〜0.3", "次の候補。単独変更では0.2で−56手、0.3で−36手。組み合わせと補正でも高めの方向はおおむね一致する。ただし0.2と0.3の細かな優劣は決まらず、効果量は小さい。"),
        ("再試行待ちの係数", "0.5〜0.75を中心に考える", "短すぎる設定の悪化が一貫する。単独の0.125は+182手、0.25は+62手。0.5以上の差は小さく、0.75が確実に0.5より良いとは言えない。1.0まで広げる利点も明確でない。"),
        ("開始温度", "0.7〜0.95。変更の優先度は低い", "単独では0.95が−53手だが、組み合わせの影響を補正すると改善が再現しない。2次曲線の底は0.7と0.95の間にあるが、厳密な値を選べるほどの根拠はない。"),
        ("終了温度", "0.08〜0.12を基準に残す。0.2は保留", "単独の0.2は−91手だが、補正では0.2の優位が消える。高い開始温度との併用も含むため、単独最良の値を他の変更へそのまま移せるとは判断しない。"),
        ("依存拡張の間隔", "方向は未確定。4を基準に残す", "単独では2・3・6が−75・−82・−84手でほぼ同じだが、頻度は大きく違う。4の一点が悪く、増減のどちらを支持する滑らかな傾向も弱い。3や6への変更を確定するには足りない。"),
    ]
    report = ["# v074：6パラメーターの大局的な考察", "",
        "保存済み65条件・6,500件だけを分析した。solverの変更・追加実行はない。修復係数は0.83で共通である。", "",
        "## 判断", "",
        "候補サイズの指数を高める方向が最も支持される。乱数幅0.2〜0.3は次に有望である。再試行待ちは短くしすぎないことが大切だが、0.5から0.75へ増やす効果は小さい。温度と依存拡張の間隔は保留する。", "",
        "| 項目 | 有望な範囲・判断 | 根拠 |", "| --- | --- | --- |",
        *[f"| {name} | {recommendation} | {reason} |" for name, recommendation, reason in descriptions], "",
        "![パラメーター別の傾向](parameter_trends.png)", "",
        "## 集計と補正", "",
        "1項目だけを変えた24条件と基準の曲線に加え、40組の各値8回の平均を確認した。40組は各値の出現数を揃えた設計であり、他の項目との組み合わせまでは直交していない。値別平均だけから独立な効果とは判断しない。", "",
        "そこで時間条件内の63設定を使い、6項目の効果を足し合わせる線形・2次・値別の3モデルを比較した。参考曲線は2次モデルで、他の項目を基準値に保ったときの差を描く。基準の予測値との差であり、線上の値を新しい実測値として扱わない。", "",
        f"設定を1つずつ外して予測したときの誤差は、2次モデルで約{fits['eligible_quadratic']['leave_one_configuration_out_rmse']:.0f}手、"
        f"線形モデルで約{fits['eligible_linear']['leave_one_configuration_out_rmse']:.0f}手、値別モデルで約{fits['eligible_categorical']['leave_one_configuration_out_rmse']:.0f}手だった。"
        "モデルで説明できない相互作用や実行時変動があり、小さい差から最適値を断定しない。時間超過の2設定を含めた分析も保存し、傾向の感度を調べた。", "",
        "測定順を追加で考慮した2次モデルでも、候補サイズの指数を高める方向と短すぎる再試行待ちを避ける方向は同じだった。測定順の項目から、温度や背景負荷を原因として特定することはできない。", "",
        "## 計数が示すこと", "",
        f"候補サイズ指数0.65から0.95への変更で、依存拡張の再挿入完了率は{mechanism[0]['dependency_completion_percent']:.1f}%から"
        f"{mechanism[4]['dependency_completion_percent']:.1f}%へ変わった。高い指数で大きい除去集合への割引を強めることと整合する。"
        "ただし探索経路も変わるため、完了率の変化を単独の因果効果とは断定しない。", "",
        "開始温度0.3・0.7・1.25では悪化受理数が192・1,461・3,284回、終了温度0.02・0.08・0.2では920・1,461・2,631回だった。温度変更は発動しており、悪化受理を増やせば完成解が一律に短くなるという結果ではない。", "",
        "依存拡張の間隔2・4・6・8では同近傍の試行数が102,021・50,544・33,379・24,866回だった。頻度変更が発動している一方、最終スコアとの関係は単調でない。", "",
        "再試行待ちは候補数に係数を掛けた後、24〜200反復へ制限する。候補数が400以上なら、係数0.5・0.75・1.0はいずれも200反復になる。係数を大きくしても待ち時間が変わらない局面がある。これが今回の横ばいをどれだけ説明するかは、保存ログからは分離していない。", "",
        "## 比較の限界", "",
        "100ケースを対応づけて再抽出した場合の95%範囲は、指数0.95対0.65の差で約−224〜+25手だった。0.95対0.35では約−385〜−97手、再試行待ち0.75対0.125では約−343〜−66手で、極端な小さい値を避ける方向の方が支持される。これは入力構成への感度だけの検査であり、実行時変動を測ったものでも、選択後の改善を保証するものでもない。", "",
        "時間条件内の組み合わせ最良は設定44の基準比−98手で、単独変更の設定04の−102手とほぼ同じだった。各項目を単独で変えた削減量を足し合わせて、組み合わせの改善量を予測することはできない。今回提示した範囲をまとめて採用した設定の改善も未確認である。", "",
        "設定46・59の時間超過は、それぞれ0000・0001の先頭2件で生じている。パラメーターが遅延の原因とは特定できないが、事前の時間条件による不適格判定は保持した。", "",
        "## 保存資料", "",
        "- summary.json：モデルの適合と予測誤差、入力再抽出の感度、計数、入力ファイルのハッシュ。",
        "- adjusted_effects.csv：各モデルが推定した、項目ごとの基準値との差。",
        "- joint_marginals.csv：組み合わせ40条件の値別平均と中央値。",
        "- parameter_trends.png / .svg：単独変更の実測と、組み合わせを含めた参考曲線。",
        "- 再集計：adhoc/scripts/analyze_v074_parameters.py。", "",
    ]
    (OUT / "report.md").write_text("\n".join(report))
    print(json.dumps({"output": str(OUT), "eligible": len(eligible), "fits": {name: {k: v for k, v in data.items() if k != 'coefficients'} for name, data in fits.items()}}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
