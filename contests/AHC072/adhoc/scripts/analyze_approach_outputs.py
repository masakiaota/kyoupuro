#!/usr/bin/env python3
"""保存済みのA/B/C操作列を再生して比較する。solver実行や解の構築は行わない。"""

from collections import Counter, deque
import json
from math import ceil
from pathlib import Path
import re
from statistics import mean, median

from analyze_input_structure import analyze


ROOT = Path(__file__).resolve().parents[2]
BINS = {"A": "v002_mono_steiner", "B": "v001_mixed_deque", "C": "v003_spring_network"}


def trace(path):
    return {
        key: float(value) if "." in value else int(value)
        for key, value in re.findall(r"\[summary\.(?:count|time_ms)\] (\w+)=([\d.]+)", path.read_text())
    }


def replay(input_path, output_path, counts, approach):
    lines = input_path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    C = "".join(lines[1 : N + 1])
    tower = [[] for _ in C]
    colors, starts = [], []
    for p, char in enumerate(C):
        if "a" <= char <= "z":
            tower[p].append(len(colors))
            colors.append(ord(char) - ord("a"))
            starts.append(p)
    b = Counter(colors)
    nest = {p: ord(char) - ord("A") for p, char in enumerate(C) if "A" <= char <= "Z"}
    directions = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}
    distance = []
    for color in range(K):
        root = next(p for p in nest if nest[p] == color)
        dist = [-1] * len(C)
        dist[root] = 0
        queue = deque([root])
        while queue:
            p = queue.popleft()
            i, j = divmod(p, N)
            for di, dj in directions.values():
                ni, nj = i + di, j + dj
                if 0 <= ni < N and 0 <= nj < N:
                    q = ni * N + nj
                    if C[q] != "#" and dist[q] < 0:
                        dist[q] = dist[p] + 1
                        queue.append(q)
        distance.append(dist)

    result = Counter()
    payload, jump, phase_cost, color_cost = Counter(), Counter(), Counter(), Counter()
    phase_single, phase_moved = Counter(), Counter()
    edge_flow, edge_moves, edge_capacity = Counter(), Counter(), {}
    setup_moves = counts.get("setup_moves", 0) if approach == "C" else 0
    pad_info, final_pad_by_id, setup_payloads = {}, {}, []
    phase_order = []
    static_tower = None
    for t, line in enumerate(output_path.read_text().splitlines()):
        if approach == "C" and t == setup_moves:
            static_tower = [list(ids) for ids in tower]
            for p, ids in enumerate(tower):
                if len(ids) >= 2:
                    pad_info[p] = {"p": p, "height": len(ids), "color": colors[ids[0]], "setup": 0,
                                   "uses": 0, "long_uses": 0, "jump_extra": 0, "users": [], "first_move": None}
                    for id_ in ids:
                        final_pad_by_id[id_] = p
            for ids in setup_payloads:
                sites = {final_pad_by_id[id_] for id_ in ids}
                assert len(sites) == 1
                pad_info[sites.pop()]["setup"] += 1
        i, j, k_text, d, l_text = line.split()
        i, j, k, l = int(i), int(j), int(k_text), int(l_text)
        p = i * N + j
        di, dj = directions[d]
        assert 0 <= k < len(tower[p]) and 1 <= l <= k + 1
        for step in range(1, l + 1):
            ni, nj = i + di * step, j + dj * step
            assert 0 <= ni < N and 0 <= nj < N and C[ni * N + nj] != "#"
        q = (i + di * l) * N + j + dj * l
        moving = tower[p][k:]
        m = len(moving)
        assert len(tower[q]) + m <= 8
        cargo_colors = Counter(colors[id_] for id_ in moving)
        delta = sum(distance[colors[id_]][p] - distance[colors[id_]][q] for id_ in moving)
        result["T"] += 1
        result["work"] += m * l
        result["moved"] += m
        result["potential_decrease"] += delta
        result["away_work"] += sum(max(0, distance[colors[id_]][q] - distance[colors[id_]][p]) for id_ in moving)
        result["nonpositive_moves"] += delta <= 0
        result["mixed_moves"] += len(cargo_colors) > 1
        payload[m] += 1
        jump[l] += 1
        for color, amount in cargo_colors.items():
            # 混載操作の費用は動いた匹数に比例して配賦する。反実仮想の単独輸送費ではない。
            color_cost[color] += amount / m

        if approach == "C":
            assert len(cargo_colors) == 1
            color = next(iter(cargo_colors))
            if t < setup_moves:
                setup_payloads.append(moving)
            else:
                if not phase_order or phase_order[-1] != color:
                    assert color not in phase_order
                    phase_order.append(color)
                rank = len(phase_order) - 1
                phase_cost[color] += 1
                phase_single[color] += m == 1
                phase_moved[color] += m
                result["delivery_moves"] += 1
                result["delivery_single_moves"] += m == 1
                result["delivery_moved"] += m
                result["delivery_work"] += m * l
                result["delivery_away_work"] += sum(max(0, distance[color][q] - distance[color][p]) for _ in moving)
                key = (color, p, q)
                edge_flow[key] += m
                edge_moves[key] += 1
                # その色の輸送開始時に固定された他色の下段だけを容量から引く。
                def base_at(cell):
                    ids = static_tower[cell]
                    if not ids or colors[ids[0]] in phase_order:
                        return 0
                    return len(ids)
                capacity = min(8 - base_at(p), 8 - base_at(q))
                edge_capacity[key] = capacity
                assert m <= capacity
                if p in pad_info and colors[moving[0]] != pad_info[p]["color"] and k >= pad_info[p]["height"]:
                    pad = pad_info[p]
                    pad["uses"] += 1
                    pad["long_uses"] += l > 1
                    pad["jump_extra"] += l - 1
                    pad["users"].append(rank)
                for id_ in moving:
                    if id_ in final_pad_by_id:
                        pad = pad_info[final_pad_by_id[id_]]
                        if pad["first_move"] is None:
                            pad["first_move"] = rank

        tower[p] = tower[p][:k]
        tower[q].extend(reversed(moving))
        for cell, name in [(p, "source_returned"), (q, "destination_returned")]:
            while tower[cell] and colors[tower[cell][-1]] == nest.get(cell, -1):
                tower[cell].pop()
                result[name] += 1

    assert not any(tower)
    assert result["T"] == counts["final_T"]
    assert result["potential_decrease"] == counts["individual_distance_sum"]
    result = dict(result)
    result["payload"] = dict(payload)
    result["jump"] = dict(jump)
    result["color_cost"] = [{"color": color, "b": b[color], "cost": color_cost[color]} for color in range(K)]
    if approach == "C":
        assert len(pad_info) == counts["built_pads"]
        assert sum(pad["setup"] for pad in pad_info.values()) == setup_moves
        assert sum(pad["long_uses"] for pad in pad_info.values()) == counts["built_support_long_moves"]
        result["fixed_flow_capacity_lower_bound"] = sum(ceil(flow / edge_capacity[key]) for key, flow in edge_flow.items())
        result["fixed_flow_packing_gap"] = sum(edge_moves.values()) - result["fixed_flow_capacity_lower_bound"]
        result["phase_order"] = phase_order
        result["phase_cost"] = [{"color": color, "b": b[color], "cost": phase_cost[color],
                                 "single_moves": phase_single[color], "moved": phase_moved[color]} for color in phase_order]
        result["delivery_moves_on_single_flow_edges"] = sum(edge_moves[key] for key, flow in edge_flow.items() if flow == 1)
        result["large_color_single_moves"] = sum(phase_single[color] for color in phase_order if b[color] >= 8)
        result["pads"] = list(pad_info.values())
    return result


def aggregate(rows):
    summary = {"cases": len(rows)}
    if not rows:
        return summary
    for approach in BINS:
        replays = [row[approach]["replay"] for row in rows]
        traces = [row[approach]["trace"] for row in rows]
        totals = {key: sum(r[key] for r in replays) for key, value in replays[0].items() if isinstance(value, (int, float))}
        for hist in ["payload", "jump"]:
            counter = Counter()
            for r in replays:
                counter.update(r[hist])
            totals[hist] = dict(sorted(counter.items()))
        totals["initial_T"] = sum(r["initial_T"] for r in traces)
        totals["sa_iterations_median"] = median(r["sa_iterations"] for r in traces)
        totals["sa_evaluated_median"] = median(r["sa_evaluated"] for r in traces)
        totals["small_color_cost"] = sum(c["cost"] for r in replays for c in r["color_cost"] if c["b"] <= 3)
        totals["large_color_cost"] = sum(c["cost"] for r in replays for c in r["color_cost"] if c["b"] >= 8)
        summary[approach] = totals
    for first, second in [("C", "B"), ("C", "A"), ("A", "B")]:
        deltas = [row[first]["replay"]["T"] - row[second]["replay"]["T"] for row in rows]
        summary[first + "_vs_" + second] = {
            "wins": sum(d < 0 for d in deltas), "ties": sum(d == 0 for d in deltas),
            "losses": sum(d > 0 for d in deltas), "delta": sum(deltas),
            "percent_reduction": -100 * sum(deltas) / summary[second]["T"],
        }
    pads = [pad for row in rows for pad in row["C"]["replay"]["pads"]]
    summary["C_pads"] = {
        "count": len(pads), "setup": sum(p["setup"] for p in pads),
        "zero_long_use": sum(p["long_uses"] == 0 for p in pads),
        "one_long_use": sum(p["long_uses"] == 1 for p in pads),
        "at_least_two_long_use": sum(p["long_uses"] >= 2 for p in pads),
        "setup_zero_long_use": sum(p["setup"] for p in pads if p["long_uses"] == 0),
        "setup_one_long_use": sum(p["setup"] for p in pads if p["long_uses"] == 1),
        "extra_jump": sum(p["jump_extra"] for p in pads),
        "last_use_before_own_by_2_phases": sum(bool(p["users"]) and p["first_move"] - max(p["users"]) >= 2 for p in pads),
    }
    summary["input_means"] = {key: mean(row["input"][key] for row in rows) for key in [
        "N", "K", "M", "wall_fraction", "slime_density", "largest_color_share",
        "small_color_slime_share", "floor_with_ray_4", "nearest_same_distance",
    ]}
    return summary


def main():
    rows = []
    for input_path in sorted((ROOT / "tools/in").glob("*.txt")):
        row = {"case": input_path.stem, "input": analyze(input_path)}
        for approach, bin_name in BINS.items():
            output_path = ROOT / "results/out" / bin_name / input_path.name
            counts = trace(output_path.with_suffix(".txt.err"))
            row[approach] = {"trace": counts, "replay": replay(input_path, output_path, counts, approach)}
        rows.append(row)
    generated = [row for row in rows if row["case"] != "0000"]
    groups = {
        "all_100": rows,
        "generated_99": generated,
        "B_beats_C": [r for r in generated if r["B"]["replay"]["T"] < r["C"]["replay"]["T"]],
        "A_beats_B": [r for r in generated if r["A"]["replay"]["T"] < r["B"]["replay"]["T"]],
        "few_colors": [r for r in generated if r["input"]["K"] <= 6],
        "many_colors": [r for r in generated if r["input"]["K"] >= 10],
        "few_walls": [r for r in generated if r["input"]["wall_fraction"] <= .10],
        "many_walls": [r for r in generated if r["input"]["wall_fraction"] >= .25],
        "sparse": [r for r in generated if r["input"]["slime_density"] < .25],
        "dense": [r for r in generated if r["input"]["slime_density"] >= .50],
        "large": [r for r in generated if r["input"]["M"] >= 100],
        "small": [r for r in generated if r["input"]["M"] <= 35],
    }
    summaries = {name: aggregate(group) for name, group in groups.items()}
    path = ROOT / "adhoc/approach_output_analysis_20260925.json"
    path.write_text(json.dumps({"summaries": summaries, "cases": rows}, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(summaries, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
