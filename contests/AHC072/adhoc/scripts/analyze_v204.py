#!/usr/bin/env python3
"""v204の保存結果を照合・記録する。solverは実行しない。"""
import csv
import json
from pathlib import Path

from tune_v204 import CONFIGS, NOTE, PARENT, PARENT_SHA, ROOT, RUN, SOURCE, digest


def main():
    assert json.loads((RUN / "status.json").read_text())["status"] == "completed"
    result = json.loads((RUN / "result.json").read_text())
    trials = json.loads((RUN / "trials.json").read_text())
    records = [json.loads(s) for s in (RUN / "cases.jsonl").read_text().splitlines()]
    assert digest(PARENT) == PARENT_SHA and digest(SOURCE) == result["source_sha256"]
    assert len(trials) == len({r["config_id"] for r in trials}) == 9
    assert all(r["status"] == "ok" for r in records)
    baseline, best, confirmation = result["baseline"], result["best"], result["confirmation"]
    for row in trials + ([confirmation] if confirmation else []):
        cases = [r for r in records if r["run_id"] == row["run_id"]]
        assert len(cases) == len({r["case_name"] for r in cases}) == 100
        assert sum(r["score"] for r in cases) == row["total_sum"]
    table = []
    for row in sorted(trials, key=lambda r: r["config"]):
        table.append(dict(candidate_count=row["config"][0], stage_percent=row["config"][1],
                          total_sum=row["total_sum"], delta_sum=row["total_sum"]-baseline["total_sum"],
                          total_avg=row["total_avg"], max_elapsed_ms=row["max_elapsed_ms"],
                          winner_changed=row["winner_changed"], race_resumes=row["race_resumes"],
                          lns_attempts=row["lns_attempts"], selected=row["config_id"]==best["config_id"]))
    with (RUN / "trials.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(table[0]))
        writer.writeheader()
        writer.writerows(table)
    selected = CONFIGS[best["config_id"]]
    verdict = "採用" if result["adopted"] else "採用見送り"
    tuning_delta = result["tuning_delta"]
    confirmation_text = "基準設定が最良のため、通常100件の確認評価は行わなかった。"
    if confirmation:
        parents = {r["case_name"]:r["score"] for r in json.loads((RUN / "parent_records.json").read_text())}
        differences = [dict(case=k, parent=parents[k], selected=v, delta=v-parents[k])
                       for k, v in sorted(confirmation["scores"].items())]
        with (RUN / "confirmation_cases.csv").open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(differences[0]))
            writer.writeheader()
            writer.writerows(differences)
        wins = sum(r["delta"] < 0 for r in differences)
        ties = sum(r["delta"] == 0 for r in differences)
        delta = result["confirmation_delta"]
        percent = 100*delta/result["parent_total_sum"]
        confirmation_text = (f"通常100件では保存済みv203比{delta:+d}手（{percent:+.4f}%）、"
                             f"{wins}勝{ties}分{100-wins-ties}敗、最大外部時間{confirmation['max_elapsed_ms']:,} msだった。")
    report = ["# v204 初期候補の段階選抜を調整した結果", "",
              f"- 判定: {verdict}。", f"- 選択設定: 最大{selected[0]}本、選抜の各段階{selected[1]}%。",
              f"- 新規100件で基準(4本、各15%)比{tuning_delta:+d}手。", f"- {confirmation_text}", "",
              "| 候補数 | 各段階の割合 | 合計 | 基準との差 | 最大ms | 選択 |",
              "| ---: | ---: | ---: | ---: | ---: | :--- |"]
    for row in table:
        report.append(f"| {row['candidate_count']} | {row['stage_percent']}% | {row['total_sum']} | "
                      f"{row['delta_sum']:+d} | {row['max_elapsed_ms']} | {'○' if row['selected'] else ''} |")
    report += ["", "全条件を各1回測定した。選択に使った新規100件と通常100件の確認評価を区別する。",
               "時間依存探索であり、単回の差から実行時変動を除いた優位性や最適性は確定しない。"]
    (RUN / "report.md").write_text("\n".join(report)+"\n")
    note = NOTE.read_text()
    marker = "## 実験後"
    if marker not in note:
        note += (f"\n{marker}\n\n- 判定: {verdict}。事前登録した通常100件の基準による。\n"
                 f"- 結果: 選択設定は最大{selected[0]}本、選抜の各段階{selected[1]}%。"
                 f"新規100件では基準(4本、各15%)比{tuning_delta:+d}手だった。{confirmation_text}\n"
                 "- 機構確認: 全9条件のLOCAL・非LOCALをビルドし、登録定数の逆適用後の完全な前処理結果が親に一致した。"
                 "非LOCALの端の2条件の診断を通過した。測定した全ケースで公式採点、独立再生、全帰巣、既存エラー0、"
                 "盤面領域4枚の全返却、段階ごとの候補順位・生存数・再開回数・試行数を照合した。"
                 "全条件で指定上限まで候補を保持するケースがあり、NNの床数選択も正常だった。\n"
                 "- 考察: 候補数と育成割合の局所的な調整結果が得られた。通常100件の確認結果を次の比較基準に使えるが、"
                 "両パラメーターの個別寄与、未知入力での最適性、実行時変動を除いた差は確定しない。"
                 "今回の設定の追加変更は行わない。\n"
                 "- 保存資料: `results/tuning/v204/result.json`、`trials.csv`、`confirmation_cases.csv`（確認評価実施時）、"
                 "`report.md`、`cases.jsonl`、`plan.json`、`frozen.json`と`adhoc/v204_audit/`。"
                 "最終ソースは`src/bin/v204_initial_race_tuned.cpp`である。\n"
                 "- 再開条件: 候補範囲や時間配分を追加検証する新たなユーザー指示がある場合。\n")
        NOTE.write_text(note)
    backlog_path = ROOT / "notes/backlog.md"
    backlog = backlog_path.read_text()
    entries = [s for s in backlog.splitlines(True) if s.startswith("- **[B-125]")]
    assert len(entries) == 1
    replacement = (f"- **[B-125] v203の初期候補数と育成時間を調整する** → [v204](experiments/v204.md) {verdict}。"
                   f"固定9条件を新規100件で比較し、最大{selected[0]}本・各段階{selected[1]}%を選択。"
                   f"新規100件で基準比{tuning_delta:+d}手。{confirmation_text}"
                   "再開条件: 追加検証のユーザー指示がある場合。\n")
    backlog = backlog.replace(entries[0], "", 1).replace("## 決着済み\n", "## 決着済み\n\n"+replacement, 1)
    backlog_path.write_text(backlog)
    print("\n".join(report[:7]))


if __name__ == "__main__":
    main()
