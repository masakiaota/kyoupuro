#!/usr/bin/env python3
"""v501 の実スコアと理論上限(LP緩和)のギャップをケース特性別に分析する。

- 上限: 時間平均容量 (grass_count × 100000 セル時間) に対する LP 緩和ナップサック。
  価値は V×C_max(P)、コストは P×(T-S)。瞬間容量の偏りを無視するため真の上限より緩い。
- v501 出力から受け入れ集合を再構成し、利用率(受け入れセル時間/容量)と
  選択品質(受け入れ集合の価値 / 同セル時間を上限選択で使った場合の価値)を分ける。
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IN_DIR = ROOT / "tools" / "in"
RECORDS = ROOT / "results" / "eval_records.jsonl"
HORIZON = 100_000


def cmax(P: int) -> float:
    Lmin = 2 * math.ceil(2 * math.sqrt(P) - 1e-12)
    return 4.0 * math.sqrt(P) / Lmin


def load_scores(bin_name: str) -> dict[str, int]:
    scores: dict[str, int] = {}
    with RECORDS.open() as f:
        for line in f:
            rec = json.loads(line)
            if rec["bin"] == bin_name and rec["status"] == "ok":
                scores[rec["case_name"]] = rec["score"]
    return scores


def parse_case(path: Path):
    lines = path.read_text().splitlines()
    N, M, R = lines[0].split()
    N, M, R = int(N), int(M), float(R)
    grass = sum(row.count(".") for row in lines[1 : 1 + N])
    groups = []
    for line in lines[1 + N : 1 + N + M]:
        i, S, T, P, V = map(int, line.split())
        groups.append((i, S, T, P, V))
    return N, M, R, grass, groups


def parse_accepted(path: Path, M: int) -> set[int]:
    """solver 出力から Yes を出したグループ番号を再構成する。"""
    accepted: set[int] = set()
    toks = path.read_text().split()
    pos = 0
    # 出力構造: 各ターン: A, A回の(j + P[j]行の座標), Yes+P行 or No
    # P[j] はケース入力に依存するので、入力からグループ P を取って進める。
    return accepted  # プレースホルダ(下の parse_accepted_full を使う)


def parse_accepted_full(out_path: Path, groups) -> tuple[set[int], int]:
    """出力をトークン列として厳密に辿り、受け入れ集合と移動回数を返す。"""
    toks = out_path.read_text().split()
    P_of = {g[0]: g[3] for g in groups}
    pos = 0
    accepted: set[int] = set()
    moves = 0
    for turn in range(len(groups)):
        A = int(toks[pos]); pos += 1
        for _ in range(A):
            j = int(toks[pos]); pos += 1
            pos += 2 * P_of[j]
            moves += 1
        tag = toks[pos]; pos += 1
        if tag == "Yes":
            gid = groups[turn][0]
            accepted.add(gid)
            pos += 2 * P_of[gid]
    return accepted, moves


def main() -> None:
    bin_name = sys.argv[1] if len(sys.argv) >= 2 else "v501_pro_shadow_packing"
    out_dir = ROOT / "results" / "out" / bin_name
    scores = load_scores(bin_name)
    rows = []
    for case_path in sorted(IN_DIR.glob("*.txt")):
        name = case_path.name
        N, M, R, grass, groups = parse_case(case_path)
        budget = grass * HORIZON
        items = []
        for (i, S, T, P, V) in groups:
            cost = P * (T - S)
            value = V * cmax(P)
            items.append((value / cost, value, cost, i))
        items.sort(reverse=True)
        # LP 緩和上限 (端数は比例配分)
        ub = 0.0
        used = 0
        for dens, value, cost, i in items:
            if used + cost <= budget:
                ub += value
                used += cost
            else:
                frac = (budget - used) / cost
                if frac > 0:
                    ub += value * frac
                    used = budget
                break
        total_demand = sum(P * (T - S) for (_, S, T, P, _) in groups)
        load = total_demand / budget
        theta_hat = sum(T - S for (_, S, T, _, _) in groups) / M - 1

        out_path = out_dir / name
        acc, moves = parse_accepted_full(out_path, groups)
        acc_cell_time = sum(P * (T - S) for (i, S, T, P, V) in groups if i in acc)
        acc_value_ub = sum(V * cmax(P) for (i, S, T, P, V) in groups if i in acc)
        util = acc_cell_time / budget
        # 同じセル時間を density 順に使った場合の上限価値 (選択品質の分母)
        sel_ub = 0.0
        used2 = 0
        for dens, value, cost, i in items:
            if used2 + cost <= acc_cell_time:
                sel_ub += value
                used2 += cost
            else:
                frac = (acc_cell_time - used2) / cost
                if frac > 0:
                    sel_ub += value * frac
                break
        score = scores.get(name, 0)
        rows.append(
            dict(
                name=name, R=R, grass=grass, theta=theta_hat, load=load,
                ub=ub, score=score, ratio=score / ub if ub else 0.0,
                util=util, n_acc=len(acc), moves=moves,
                sel_quality=acc_value_ub / sel_ub if sel_ub else 0.0,
                # 受け入れ集合の価値のうち実際に得られた率 (compactness 低下 + 移動費)
                realize=score / acc_value_ub if acc_value_ub else 0.0,
            )
        )

    def summarize(label: str, subset):
        subset = list(subset)
        if not subset:
            return
        n = len(subset)
        avg = lambda k: sum(r[k] for r in subset) / n
        print(
            f"{label:24s} n={n:3d} ratio={avg('ratio'):.3f} util={avg('util'):.3f} "
            f"selq={avg('sel_quality'):.3f} realize={avg('realize'):.3f} "
            f"score={avg('score')/1e6:.1f}M ub={avg('ub')/1e6:.1f}M moves={avg('moves'):.0f}"
        )

    print("== overall ==")
    summarize("all", rows)
    print("== by load ==")
    summarize("load<0.8", (r for r in rows if r["load"] < 0.8))
    summarize("0.8<=load<1.3", (r for r in rows if 0.8 <= r["load"] < 1.3))
    summarize("1.3<=load<1.9", (r for r in rows if 1.3 <= r["load"] < 1.9))
    summarize("load>=1.9", (r for r in rows if r["load"] >= 1.9))
    print("== by R ==")
    summarize("R<=0.02", (r for r in rows if r["R"] <= 0.02))
    summarize("0.02<R<0.07", (r for r in rows if 0.02 < r["R"] < 0.07))
    summarize("R>=0.07", (r for r in rows if r["R"] >= 0.07))
    print("== by load x R ==")
    summarize("hi-load lo-R", (r for r in rows if r["load"] >= 1.3 and r["R"] <= 0.03))
    summarize("hi-load hi-R", (r for r in rows if r["load"] >= 1.3 and r["R"] > 0.03))
    summarize("lo-load lo-R", (r for r in rows if r["load"] < 1.3 and r["R"] <= 0.03))
    summarize("lo-load hi-R", (r for r in rows if r["load"] < 1.3 and r["R"] > 0.03))
    print("== worst 12 by ratio ==")
    for r in sorted(rows, key=lambda r: r["ratio"])[:12]:
        print(
            f"{r['name']} R={r['R']:.3f} load={r['load']:.2f} grass={r['grass']} "
            f"ratio={r['ratio']:.3f} util={r['util']:.3f} selq={r['sel_quality']:.3f} "
            f"realize={r['realize']:.3f} moves={r['moves']}"
        )


if __name__ == "__main__":
    main()
