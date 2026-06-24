#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
from pathlib import Path
from statistics import mean, median

N = 20
C = N * N
START = 0
GOAL = C - 1


def cell_id(i: int, j: int) -> int:
    return i * N + j


def read_latest_scores(path: Path, bin_name: str) -> dict[str, int]:
    if not path.is_file():
        return {}
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle))
    if len(rows) < 2:
        return {}
    header = rows[0]
    for row in reversed(rows[1:]):
        if row and row[0] == bin_name:
            return {
                name: int(value)
                for name, value in zip(header[3:-2], row[3:-2])
                if value
            }
    return {}


def build_graph(grid: list[str]) -> tuple[list[tuple[int, int]], list[list[tuple[int, int]]]]:
    edges: list[tuple[int, int]] = []
    adj: list[list[tuple[int, int]]] = [[] for _ in range(C)]
    for i in range(N):
        for j in range(N):
            if grid[i][j] != ".":
                continue
            if i + 1 < N and grid[i + 1][j] == ".":
                a = cell_id(i, j)
                b = cell_id(i + 1, j)
                e = len(edges)
                edges.append((a, b))
                adj[a].append((b, e))
                adj[b].append((a, e))
            if j + 1 < N and grid[i][j + 1] == ".":
                a = cell_id(i, j)
                b = cell_id(i, j + 1)
                e = len(edges)
                edges.append((a, b))
                adj[a].append((b, e))
                adj[b].append((a, e))
    return edges, adj


def find_bridges(edges: list[tuple[int, int]], adj: list[list[tuple[int, int]]]) -> list[bool]:
    tin = [-1] * C
    low = [0] * C
    is_bridge = [False] * len(edges)
    timer = 0

    def dfs(v: int, pe: int) -> None:
        nonlocal timer
        tin[v] = low[v] = timer
        timer += 1
        for to, e in adj[v]:
            if e == pe:
                continue
            if tin[to] >= 0:
                low[v] = min(low[v], tin[to])
                continue
            dfs(to, e)
            low[v] = min(low[v], low[to])
            if low[to] > tin[v]:
                is_bridge[e] = True

    dfs(START, -1)
    return is_bridge


def bridge_tree_stats(
    grid: list[str], edges: list[tuple[int, int]], adj: list[list[tuple[int, int]]], is_bridge: list[bool]
) -> tuple[int, int, bool, int, int]:
    comp = [-1] * C
    comps: list[list[int]] = []
    for s in range(C):
        if grid[s // N][s % N] != "." or comp[s] >= 0:
            continue
        cid = len(comps)
        comps.append([])
        queue = [s]
        comp[s] = cid
        for v in queue:
            comps[cid].append(v)
            for to, e in adj[v]:
                if is_bridge[e] or comp[to] >= 0:
                    continue
                comp[to] = cid
                queue.append(to)

    tree: list[list[tuple[int, int]]] = [[] for _ in comps]
    for e, (a, b) in enumerate(edges):
        if not is_bridge[e]:
            continue
        ca = comp[a]
        cb = comp[b]
        if ca != cb:
            tree[ca].append((cb, e))
            tree[cb].append((ca, e))

    root = comp[START]
    goal = comp[GOAL]
    parent = [-1] * len(comps)
    pedge = [-1] * len(comps)
    order = [root]
    parent[root] = root
    for v in order:
        for to, e in tree[v]:
            if parent[to] < 0:
                parent[to] = v
                pedge[to] = e
                order.append(to)

    path: list[int] = []
    if parent[goal] >= 0:
        v = goal
        while v != root:
            path.append(v)
            v = parent[v]
        path.append(root)
        path.reverse()

    cand_max = 0
    for t in range(1, len(path)):
        goal_sub = [False] * len(comps)
        stack = [path[t]]
        goal_sub[path[t]] = True
        for v in stack:
            for to, _ in tree[v]:
                if parent[to] == v and not goal_sub[to]:
                    goal_sub[to] = True
                    stack.append(to)
        prefix = [not x for x in goal_sub]
        main_edges = {pedge[path[i]] for i in range(1, t) if pedge[path[i]] >= 0}
        count = 0
        for v in range(len(comps)):
            if not prefix[v] or v == root:
                continue
            if any(parent[to] == v and prefix[to] for to, _ in tree[v]):
                continue
            x = v
            tail_len = 0
            while x != root:
                e = pedge[x]
                if e < 0 or e in main_edges:
                    break
                tail_len += 1
                x = parent[x]
            if tail_len:
                count += 1
        cand_max = max(cand_max, count)

    return len(comps), sum(is_bridge), root == goal, max(0, len(path) - 1), cand_max


def analyze_case(path: Path, score: int | None) -> dict[str, int | str | bool | None]:
    lines = path.read_text(encoding="utf-8").splitlines()
    grid = lines[1 : 1 + N]
    edges, adj = build_graph(grid)
    is_bridge = find_bridges(edges, adj)
    comps, bridges, same_comp, path_len, cand_max = bridge_tree_stats(grid, edges, adj, is_bridge)
    cell_cut_2_4 = sum(1 for v in range(C) if grid[v // N][v % N] == "." and 2 <= len(adj[v]) <= 4)
    return {
        "case": path.name,
        "score": score,
        "bridges": bridges,
        "components": comps,
        "same_start_goal_component": same_comp,
        "bridge_path_len": path_len,
        "chinese_candidate_max": cand_max,
        "cell_cut_2_4": cell_cut_2_4,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_dir", nargs="?", default="tools/in")
    parser.add_argument("--bin", default="v105_virtual_bridge_no_sa")
    parser.add_argument("--score-detail", default="results/score_detail.csv")
    parser.add_argument("--low", type=int, default=12)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    input_dir = (root / args.input_dir).resolve()
    scores = read_latest_scores(root / args.score_detail, args.bin)
    rows = [
        analyze_case(path, scores.get(path.name))
        for path in sorted(input_dir.rglob("*"))
        if path.is_file()
    ]

    print(f"cases={len(rows)} bin={args.bin} scores={'yes' if scores else 'no'}")
    for key in ["bridges", "components", "bridge_path_len", "chinese_candidate_max", "cell_cut_2_4"]:
        values = [int(row[key]) for row in rows]
        print(
            f"{key}: avg={mean(values):.2f} median={median(values)} min={min(values)} max={max(values)}"
        )
    print(f"same_start_goal_component={sum(bool(row['same_start_goal_component']) for row in rows)}")

    if scores:
        print("low cases:")
        for row in sorted(rows, key=lambda x: int(x["score"] or 0))[: args.low]:
            print(
                "{case} score={score} bridges={bridges} path={bridge_path_len} "
                "cand={chinese_candidate_max} cell_cut_2_4={cell_cut_2_4} same={same_start_goal_component}".format(
                    **row
                )
            )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
