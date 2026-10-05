#!/usr/bin/env python3
"""保存したv076の出力を検査・集計する。solverは実行しない。"""
from collections import Counter
import csv
import hashlib
import json
from pathlib import Path

from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "results/analysis/v076"
BIN = "v076_relative_tuned"
MECHANISM = (
    "lns_attempts", "lns_insertions", "lns_uphill", "lns_restarts",
    "lns_repair_priority_evaluated", "lns_dependency_attempts",
    "lns_dependency_completed",
)


def relative(minimum, score):
    return (2 * 10**9 * minimum + score) // (2 * score)


def main():
    frozen = json.loads((ROOT / "adhoc/v076_audit/preflight.json").read_text())
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    assert digest(ROOT / f"src/bin/{BIN}.cpp") == frozen["solver_sha256"]
    for name, expected in frozen["input_sha256"].items():
        assert digest(ROOT / "tools/in" / name) == expected
    records = [json.loads(line) for line in (ROOT / "results/eval_records.jsonl").open()]
    current = [r for r in records if r["bin"] == BIN]
    assert len(current) == len({r["case_name"] for r in current}) == 100
    assert len({r["run_id"] for r in current}) == 1
    assert {r["case_name"] for r in current} == set(frozen["input_sha256"])
    totals, cases = Counter(), []
    for r in sorted(current, key=lambda item: item["case_name"]):
        assert r["status"] == "ok" and r["local"] and r["input_dir"] == "tools/in"
        path = ROOT / r["stdout_path"]
        count = verify_output(r["case_name"], path)
        c, times, diagnostics = log_values(path.with_suffix(".txt.err"))
        assert count == r["score"] == c["T"] == c["final_ops"] == c["validated_moves"]
        assert c["E"] == 0 and count <= 100000 and not diagnostics
        assert not any(c[k] for k in ERRORS)
        assert c["state_pool_free_at_end"] == c["state_slots"] == 4
        totals.update({key: c[key] for key in MECHANISM})
        cases.append({"case": r["case_name"], "score": r["score"], "elapsed_ms": r["elapsed"],
                      "counts": {key: c[key] for key in MECHANISM}, "search_limit_ms": times["search_limit"]})
    assert all(totals[k] > 0 for k in ("lns_repair_priority_evaluated", "lns_dependency_attempts", "lns_uphill"))
    sets = {**frozen["baseline_records"], "v076": current}
    scores = {key: {r["case_name"]: r["score"] for r in rows} for key, rows in sets.items()}
    minima = {case: min(old, scores["v076"][case]) for case, old in frozen["historical_min"].items()}
    summary = {}
    for key, rows in sets.items():
        rel_sum = sum(relative(minima[case], score) for case, score in scores[key].items())
        summary[key] = {
            "run_id": rows[0]["run_id"], "total_sum": sum(scores[key].values()),
            "total_avg": sum(scores[key].values()) / 100, "relative_avg": rel_sum / 100,
            "relative_avg_100": rel_sum / 10**9,
            "max_elapsed_ms": max(r["elapsed"] for r in rows),
            "avg_elapsed_ms": sum(r["elapsed"] for r in rows) / 100,
        }
    comparisons = {}
    for key in frozen["baseline_records"]:
        delta = [scores["v076"][c] - scores[key][c] for c in minima]
        comparisons[key] = {
            "absolute_delta": sum(delta), "absolute_delta_percent": 100 * sum(delta) / summary[key]["total_sum"],
            "relative_delta_pp": summary["v076"]["relative_avg_100"] - summary[key]["relative_avg_100"],
            "wins": sum(d < 0 for d in delta), "draws": sum(d == 0 for d in delta), "losses": sum(d > 0 for d in delta),
        }
    adopted = summary["v076"]["max_elapsed_ms"] <= 2000 and all(
        comparisons[key]["relative_delta_pp"] > 0 for key in ("v058", "v059"))
    result = {
        "solver_sha256": frozen["solver_sha256"], "parameters": frozen["parameters"],
        "verified_cases": 100, "official_success_cases": 100, "errors": {k: 0 for k in ERRORS},
        "reference": {"historical_run_ids": frozen["historical_reference_run_ids"], "minima_including_v076": minima},
        "summary": summary, "comparisons": comparisons, "mechanism": dict(totals), "adopted": adopted,
        "additional_solver_executions": 0,
    }
    (OUT / "comparison.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    with (OUT / "mechanism.jsonl").open("w") as f:
        for row in cases:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    with (OUT / "case_comparison.csv").open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["case", "reference_min", *scores, "delta_vs_v058", "delta_vs_v059", "elapsed_ms"])
        for row in cases:
            case = row["case"]
            writer.writerow([case, minima[case], *(scores[key][case] for key in scores),
                             scores["v076"][case]-scores["v058"][case], scores["v076"][case]-scores["v059"][case], row["elapsed_ms"]])
    verdict = "採用" if adopted else "採用基準未達"
    lines = ["# v076: 相対スコアを優先した固定パラメータの評価", "", f"判定: {verdict}。", "",
             "`tools/in`100件、LOCAL、ケース単位2並列で1回測定した。全100出力が公式採点と独立再生を通過し、全帰巣・エラー計数0・盤面領域の全返却を確認した。", "",
             "擬似相対は評価前の63正常実行と今回のv076のケース別最小値を共通の参照値とし、公式の丸め式による平均を100点満点へ換算した。他参加者を含む公式相対スコアではない。", "",
             "| 版 | 絶対合計 ↓ | 平均手数 ↓ | 擬似相対 ↑ | 最大時間 |", "| --- | ---: | ---: | ---: | ---: |"]
    for key, row in summary.items():
        lines.append(f"| {key} | {row['total_sum']:,} | {row['total_avg']:.2f} | {row['relative_avg_100']:.6f} | {row['max_elapsed_ms']} ms |")
    lines += ["", "## 保存基準との比較", ""]
    for key, row in comparisons.items():
        lines.append(f"- {key}比: 絶対合計{row['absolute_delta']:+d}手（{row['absolute_delta_percent']:+.3f}%）、擬似相対{row['relative_delta_pp']:+.6f}ポイント、{row['wins']}勝{row['draws']}分{row['losses']}敗。")
    lines += ["", "単回の実時間比較であり、係数変更の効果と実行時変動を分離していない。LOCALの結果から提出環境の実時間や順位を保証するものではない。", "",
              "## 保存資料", "", "- `comparison.json`: 設定、ソースハッシュ、共通参照値、比較と判定。",
              "- `case_comparison.csv`: ケース別の手数比較。", "- `mechanism.jsonl`: 既存の主要計数と時間。",
              "- `eval.log`: 通常評価の出力。", "- 再集計: `python3 adhoc/scripts/analyze_v076.py`（solverの実行なし）。", ""]
    (OUT / "report.md").write_text("\n".join(lines))
    print(json.dumps({k: result[k] for k in ("verified_cases", "summary", "comparisons", "mechanism", "adopted")}, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
