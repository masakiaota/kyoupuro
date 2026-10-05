#!/usr/bin/env python3
"""入力の分布と地形だけを集計する。操作列の構築、solver の実行、採点は行わない。"""

import argparse
from collections import deque
import json
from pathlib import Path
from statistics import mean, median


def analyze(path):
    lines = path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    C = "".join(lines[1 : N + 1])
    floor = [p for p, char in enumerate(C) if char != "#"]
    adjacent = [[] for _ in C]
    rays = [[] for _ in C]
    for p in floor:
        i, j = divmod(p, N)
        for di, dj in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            ray = []
            for l in range(1, N):
                ni, nj = i + di * l, j + dj * l
                if not (0 <= ni < N and 0 <= nj < N):
                    break
                q = ni * N + nj
                if C[q] == "#":
                    break
                ray.append(q)
            rays[p].append(ray)
            if ray:
                adjacent[p].append(ray[0])

    def distances(start):
        dist = [-1] * (N * N)
        dist[start] = 0
        queue = deque([start])
        while queue:
            p = queue.popleft()
            for q in adjacent[p]:
                if dist[q] < 0:
                    dist[q] = dist[p] + 1
                    queue.append(q)
        return dist

    positions = [[p for p, char in enumerate(C) if char == chr(97 + c)] for c in range(K)]
    nests = [C.index(chr(65 + c)) for c in range(K)]
    b = [len(cells) for cells in positions]
    M = sum(b)
    slimes = [p for cells in positions for p in cells]
    slime_set = set(slimes)
    own_distances = []
    shortest_path_turns = []
    for c in range(K):
        dist = distances(nests[c])
        own_distances.extend(dist[p] for p in positions[c])
        # 最短路に限定し、直前の移動方向ごとに残りの最小屈曲数を求める。
        turns = [[0] * 4 for _ in C]
        for p in sorted(floor, key=lambda p: dist[p]):
            if p == nests[c]:
                continue
            next_steps = [(d, ray[0]) for d, ray in enumerate(rays[p]) if ray and dist[ray[0]] == dist[p] - 1]
            for previous in range(4):
                turns[p][previous] = min((d != previous) + turns[q][d] for d, q in next_steps)
        shortest_path_turns.extend(min(turns[p]) for p in positions[c])

    nearest_any = []
    nearest_same = []
    for c, cells in enumerate(positions):
        for p in cells:
            dist = distances(p)
            nearest_any.append(min(dist[q] for q in slimes if q != p))
            if b[c] > 1:
                nearest_same.append(min(dist[q] for q in cells if q != p))

    aligned_nest_pairs = sum(
        any(nests[other] in ray[:8] for ray in rays[p])
        for c, p in enumerate(nests)
        for other in range(c + 1, K)
    )
    return {
        "case": path.stem,
        "N": N,
        "K": K,
        "M": M,
        "wall_fraction": 1 - len(floor) / (N * N),
        "slime_density": M / (len(floor) - K),
        "largest_color_share": max(b) / M,
        "top_two_color_share": sum(sorted(b, reverse=True)[:2]) / M,
        "effective_colors": M * M / sum(count * count for count in b),
        "small_color_fraction": sum(count <= 3 for count in b) / K,
        "small_color_slime_share": sum(count for count in b if count <= 3) / M,
        "own_nest_distance": mean(own_distances),
        "individual_distance_sum": sum(own_distances),
        "shortest_path_turns": mean(shortest_path_turns),
        "nearest_any_distance": mean(nearest_any),
        "nearest_same_distance": mean(nearest_same) if nearest_same else None,
        "floor_with_ray_4": mean(any(len(ray) >= 4 for ray in rays[p]) for p in floor),
        "floor_with_ray_6": mean(any(len(ray) >= 6 for ray in rays[p]) for p in floor),
        "slime_with_other_at_distance_2": mean(
            any(len(ray) >= 2 and ray[1] in slime_set for ray in rays[p]) for p in slimes
        ),
        "aligned_nest_pairs_within_8": aligned_nest_pairs,
    }


def summarize(rows):
    result = {"cases": len(rows)}
    if not rows:
        return result
    for key in rows[0]:
        if key == "case":
            continue
        values = [row[key] for row in rows if row[key] is not None]
        if values:
            result[key] = {
                "mean": round(mean(values), 4),
                "median": round(median(values), 4),
                "min": round(min(values), 4),
                "max": round(max(values), 4),
            }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="?", type=Path, default=Path(__file__).resolve().parents[2] / "tools/in")
    args = parser.parse_args()
    # seed=0 は手作り入力であり、生成分布の集計から除く。
    rows = [analyze(path) for path in sorted(args.inputs.glob("*.txt")) if path.stem != "0000"]
    groups = {
        "all_generated": rows,
        "few_colors_K_le_6": [row for row in rows if row["K"] <= 6],
        "many_colors_K_ge_10": [row for row in rows if row["K"] >= 10],
        "few_walls_le_10_percent": [row for row in rows if row["wall_fraction"] <= 0.10],
        "many_walls_ge_25_percent": [row for row in rows if row["wall_fraction"] >= 0.25],
        "sparse_slimes_lt_25_percent": [row for row in rows if row["slime_density"] < 0.25],
        "dense_slimes_ge_50_percent": [row for row in rows if row["slime_density"] >= 0.50],
    }
    print(json.dumps({name: summarize(group) for name, group in groups.items()}, indent=2))


if __name__ == "__main__":
    main()
