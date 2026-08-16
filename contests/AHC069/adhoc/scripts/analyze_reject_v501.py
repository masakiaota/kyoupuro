#!/usr/bin/env python3
"""v501 出力を盤面リプレイし、No を出した瞬間の空き状態から棄却の内訳を分類する。

分類 (No を出したターンについて):
- cap_short:   空きセル総数 < P (絶対容量不足)
- frag_short:  空きセル総数 >= P だが最大空き連結成分 < P (断片化棄却)
- fit_possible: 最大空き連結成分 >= P (置けたはずの No = 価格棄却 or 形状/評価の棄却)
また fit_possible のうち棄却グループの価値密度が高いもの (q>=1) を数える。
"""
from __future__ import annotations

import math
from collections import deque
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
IN_DIR = ROOT / "tools" / "in"
OUT_DIR = ROOT / "results" / "out" / "v501_pro_shadow_packing"


def parse_case(path: Path):
    lines = path.read_text().splitlines()
    N, M, R = lines[0].split()
    N, M, R = int(N), int(M), float(R)
    grid = [row for row in lines[1 : 1 + N]]
    groups = []
    for line in lines[1 + N : 1 + N + M]:
        i, S, T, P, V = map(int, line.split())
        groups.append((i, S, T, P, V))
    return N, M, R, grid, groups


def max_free_component(N, grass, occ):
    seen = [[False] * N for _ in range(N)]
    best = 0
    total = 0
    for sr in range(N):
        for sc in range(N):
            if seen[sr][sc] or not grass[sr][sc] or occ[sr][sc]:
                continue
            size = 0
            dq = deque([(sr, sc)])
            seen[sr][sc] = True
            while dq:
                r, c = dq.popleft()
                size += 1
                for dr, dc in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                    nr, nc = r + dr, c + dc
                    if 0 <= nr < N and 0 <= nc < N and not seen[nr][nc] and grass[nr][nc] and not occ[nr][nc]:
                        seen[nr][nc] = True
                        dq.append((nr, nc))
            best = max(best, size)
            total += size
    return total, best


def main() -> None:
    agg = dict(no=0, cap_short=0, frag_short=0, fit_possible=0, fit_possible_hi_q=0,
               frag_short_hi_q=0, yes=0)
    per_case = []
    for case_path in sorted(IN_DIR.glob("*.txt")):
        name = case_path.name
        N, M, R, grid, groups = parse_case(case_path)
        grass = [[ch == "." for ch in row] for row in grid]
        P_of = {g[0]: g[3] for g in groups}
        T_of = {g[0]: g[2] for g in groups}
        toks = (OUT_DIR / name).read_text().split()
        pos = 0
        occ = [[False] * N for _ in range(N)]
        cells_of: dict[int, list[tuple[int, int]]] = {}
        active: list[int] = []
        c = dict(no=0, cap_short=0, frag_short=0, fit_possible=0, fit_possible_hi_q=0,
                 frag_short_hi_q=0, yes=0)
        for turn in range(M):
            gid, S, T, P, V = groups[turn]
            # 退去処理 (T < S のアクティブグループを外す)
            for aid in [a for a in active if T_of[a] < S]:
                for (r, cc) in cells_of[aid]:
                    occ[r][cc] = False
                active.remove(aid)
                del cells_of[aid]
            # 移動ブロック
            A = int(toks[pos]); pos += 1
            moved: list[tuple[int, list[tuple[int, int]]]] = []
            for _ in range(A):
                j = int(toks[pos]); pos += 1
                cells = []
                for _ in range(P_of[j]):
                    z = int(toks[pos]); w = int(toks[pos + 1]); pos += 2
                    cells.append((z, w))
                moved.append((j, cells))
            for j, _ in moved:
                for (r, cc) in cells_of[j]:
                    occ[r][cc] = False
            for j, cells in moved:
                for (r, cc) in cells:
                    occ[r][cc] = True
                cells_of[j] = cells
            # 受け入れ
            tag = toks[pos]; pos += 1
            if tag == "Yes":
                cells = []
                for _ in range(P):
                    x = int(toks[pos]); y = int(toks[pos + 1]); pos += 2
                    cells.append((x, y))
                for (r, cc) in cells:
                    occ[r][cc] = True
                cells_of[gid] = cells
                active.append(gid)
                c["yes"] += 1
            else:
                c["no"] += 1
                total, best = max_free_component(N, grass, occ)
                q = V / (P * (T - S) ** 0.9)
                if total < P:
                    c["cap_short"] += 1
                elif best < P:
                    c["frag_short"] += 1
                    if q >= 1.0:
                        c["frag_short_hi_q"] += 1
                else:
                    c["fit_possible"] += 1
                    if q >= 1.0:
                        c["fit_possible_hi_q"] += 1
        for k in agg:
            agg[k] += c[k]
        per_case.append((name, c))

    print("== aggregate over 100 cases ==")
    for k, v in agg.items():
        print(f"{k:18s} {v}")
    no = agg["no"]
    print(f"\nno breakdown: cap_short={agg['cap_short']/no:.1%} frag_short={agg['frag_short']/no:.1%} fit_possible={agg['fit_possible']/no:.1%}")


if __name__ == "__main__":
    main()
