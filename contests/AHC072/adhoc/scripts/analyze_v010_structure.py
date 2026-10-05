#!/usr/bin/env python3
"""Replay saved v008/v010 outputs to measure transport structure; never run a solver."""

from collections import Counter, defaultdict, deque
import json
from pathlib import Path

from analyze_approach_outputs import trace


ROOT = Path(__file__).resolve().parents[2]
BINS = ("v008_lightweight", "v010_scaffold_chain")
DIRECTIONS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}


def replay(input_path, output_path):
    lines = input_path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    C = "".join(lines[1 : N + 1])
    tower = [[] for _ in C]
    colors, origins = [], []
    for p, char in enumerate(C):
        if "a" <= char <= "z":
            tower[p].append(len(colors))
            colors.append(ord(char) - ord("a"))
            origins.append(p)
    b = Counter(colors)
    nest = {p: ord(char) - ord("A") for p, char in enumerate(C) if "A" <= char <= "Z"}
    distance = []
    for c in range(K):
        root = next(p for p in nest if nest[p] == c)
        dist = [-1] * len(C)
        dist[root] = 0
        queue = deque([root])
        while queue:
            p = queue.popleft()
            i, j = divmod(p, N)
            for di, dj in DIRECTIONS.values():
                ni, nj = i + di, j + dj
                if 0 <= ni < N and 0 <= nj < N:
                    q = ni * N + nj
                    if C[q] != "#" and dist[q] < 0:
                        dist[q] = dist[p] + 1
                        queue.append(q)
        distance.append(dist)

    counts = trace(output_path.with_name(output_path.name + ".err"))
    result = Counter(M=len(colors), shortest_distance_sum=sum(
        distance[c][p] for c, p in zip(colors, origins)
    ))
    payload, jump, support_height = Counter(), Counter(), Counter()
    edge_ids = defaultdict(set)
    single_edges = []
    returned = set()
    for line in output_path.read_text().splitlines():
        i_text, j_text, k_text, d, l_text = line.split()
        i, j, k, l = map(int, (i_text, j_text, k_text, l_text))
        p = i * N + j
        di, dj = DIRECTIONS[d]
        assert 0 <= k < len(tower[p]) and 1 <= l <= k + 1
        for step in range(1, l + 1):
            ni, nj = i + di * step, j + dj * step
            assert 0 <= ni < N and 0 <= nj < N and C[ni * N + nj] != "#"
        q = (i + di * l) * N + j + dj * l
        moving = tower[p][k:]
        m = len(moving)
        assert len(tower[q]) + m <= 8
        cargo_colors = {colors[id_] for id_ in moving}
        result["T"] += 1
        result["W"] += m * l
        result["moved"] += m
        result["potential_decrease"] += sum(distance[colors[id_]][p] - distance[colors[id_]][q] for id_ in moving)
        result["mixed_moves"] += len(cargo_colors) > 1
        result["single_moves"] += m == 1
        result["large_color_single_moves"] += m == 1 and b[colors[moving[0]]] >= 8
        result["maximum_transport_moves"] += m * l == 20
        payload[m] += 1
        jump[l] += 1
        support_height[k] += 1
        if len(cargo_colors) == 1:
            key = (colors[moving[0]], p, q)
            edge_ids[key].update(moving)
            if m == 1:
                single_edges.append((key, b[key[0]] >= 8))

        tower[p] = tower[p][:k]
        tower[q].extend(reversed(moving))
        for cell, name in ((p, "source_returned"), (q, "destination_returned")):
            while tower[cell] and colors[tower[cell][-1]] == nest.get(cell, -1):
                id_ = tower[cell].pop()
                assert id_ not in returned
                returned.add(id_)
                result[name] += 1

    assert len(returned) == len(colors) and not any(tower)
    assert result["T"] == counts.get("final_ops", counts.get("final_T"))
    assert result["potential_decrease"] == result["shortest_distance_sum"]
    # These edge counts ignore timing, mixed trips, and installation boundaries.
    # They describe route overlap, not an achievable reduction or a packing gap.
    result["single_moves_on_one_id_mono_edges"] = sum(len(edge_ids[key]) == 1 for key, _ in single_edges)
    result["single_moves_on_shared_mono_edges"] = sum(len(edge_ids[key]) > 1 for key, _ in single_edges)
    result["large_color_single_moves_on_shared_mono_edges"] = sum(
        large and len(edge_ids[key]) > 1 for key, large in single_edges
    )
    result["setup_ops"] = counts.get("setup_ops", counts.get("setup_moves", 0))
    return {"counts": dict(result), "payload": dict(payload), "jump": dict(jump),
            "support_height": dict(support_height)}


def aggregate(rows):
    summary = {"cases": len(rows)}
    for name in BINS:
        totals = Counter()
        histograms = {key: Counter() for key in ("payload", "jump", "support_height")}
        for row in rows:
            totals.update(row[name]["counts"])
            for key, histogram in histograms.items():
                histogram.update(row[name][key])
        summary[name] = {
            **totals,
            **{key: dict(sorted(value.items())) for key, value in histograms.items()},
            "mean_payload": totals["moved"] / totals["T"],
            "mean_transport": totals["W"] / totals["T"],
            "detour_percent": 100 * (totals["W"] / totals["shortest_distance_sum"] - 1),
            "single_move_percent": 100 * totals["single_moves"] / totals["T"],
            "large_color_share_of_single_moves_percent": 100 * totals["large_color_single_moves"] / totals["single_moves"],
            "single_moves_on_one_id_mono_edges_percent": 100 * totals["single_moves_on_one_id_mono_edges"] / totals["single_moves"],
            "setup_percent": 100 * totals["setup_ops"] / totals["T"],
        }
    return summary


def main():
    rows = [{"case": path.stem, **{
        name: replay(path, ROOT / "results/out" / name / path.name) for name in BINS
    }} for path in sorted((ROOT / "tools/in").glob("*.txt"))]
    result = {
        "scope": "All final-output moves, including setup. Not directly comparable to delivery-only O8/O9.",
        "edge_overlap": "Same color and directed endpoints in monochrome moves; distinct slime IDs; ignores time and mixed moves.",
        "all_100": aggregate(rows),
        "generated_99": aggregate([row for row in rows if row["case"] != "0000"]),
        "cases": rows,
    }
    path = ROOT / "adhoc/v010_structure_analysis.json"
    path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(result["generated_99"], ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
