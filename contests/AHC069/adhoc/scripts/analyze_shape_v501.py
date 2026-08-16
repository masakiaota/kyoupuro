#!/usr/bin/env python3
"""v501 出力をリプレイし、受け入れ形状の C/C_max 分布と価値加重ロスを分析する。

受け入れ時の初期配置周長と、移動後を含む最悪周長から各グループの C[i] を求め、
価値加重で「どの形状経路がどれだけ realize ロスに寄与しているか」を出す。
"""
from __future__ import annotations

import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IN_DIR = ROOT / "tools" / "in"
OUT_DIR = ROOT / "results" / "out" / "v501_pro_shadow_packing"


def min_perimeter(P: int) -> int:
    return 2 * math.ceil(2 * math.sqrt(P) - 1e-12)


def perimeter_of(cells: list[tuple[int, int]]) -> int:
    s = set(cells)
    per = 0
    for (r, c) in cells:
        for dr, dc in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            if (r + dr, c + dc) not in s:
                per += 1
    return per


def main() -> None:
    # slack (worst_L - min_L) 別の 価値加重集計
    by_slack: dict[int, float] = {}
    total_v_cmax = 0.0
    total_v_c = 0.0
    move_cost_total = 0
    fee_total = 0.0
    for case_path in sorted(IN_DIR.glob("*.txt")):
        name = case_path.name
        lines = case_path.read_text().splitlines()
        N, M, R = lines[0].split()
        N, M, R = int(N), int(M), float(R)
        groups = []
        for line in lines[1 + N : 1 + N + M]:
            i, S, T, P, V = map(int, line.split())
            groups.append((i, S, T, P, V))
        P_of = {g[0]: g[3] for g in groups}
        V_of = {g[0]: g[4] for g in groups}
        toks = (OUT_DIR / name).read_text().split()
        pos = 0
        worst_L: dict[int, int] = {}
        for turn in range(M):
            gid, S, T, P, V = groups[turn]
            A = int(toks[pos]); pos += 1
            for _ in range(A):
                j = int(toks[pos]); pos += 1
                cells = []
                for _ in range(P_of[j]):
                    z = int(toks[pos]); w = int(toks[pos + 1]); pos += 2
                    cells.append((z, w))
                worst_L[j] = max(worst_L[j], perimeter_of(cells))
                move_cost_total += max(round(V_of[j] * R), 1)
            tag = toks[pos]; pos += 1
            if tag == "Yes":
                cells = []
                for _ in range(P):
                    x = int(toks[pos]); y = int(toks[pos + 1]); pos += 2
                    cells.append((x, y))
                worst_L[gid] = perimeter_of(cells)
        for gid, L in worst_L.items():
            P = P_of[gid]
            V = V_of[gid]
            c_max = 4 * math.sqrt(P) / min_perimeter(P)
            c = 4 * math.sqrt(P) / L
            slack = L - min_perimeter(P)
            by_slack.setdefault(slack, 0.0)
            by_slack[slack] += V * (c_max - c)
            total_v_cmax += V * c_max
            total_v_c += V * c
            fee_total += round(V * c)

    print(f"total V*C_max = {total_v_cmax/1e6:.1f}M")
    print(f"total V*C     = {total_v_c/1e6:.1f}M  (C ratio = {total_v_c/total_v_cmax:.3f})")
    print(f"total fee     = {fee_total/1e6:.1f}M")
    print(f"total move cost = {move_cost_total/1e6:.2f}M")
    print("\nslack(L - L_min) 別の価値加重ロス (V*(C_max - C) 合計, M 単位):")
    for slack in sorted(by_slack):
        loss = by_slack[slack] / 1e6
        print(f"  slack={slack:3d}: {loss:8.2f}M")


if __name__ == "__main__":
    main()
