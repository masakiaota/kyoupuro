#!/usr/bin/env python3
"""保存済み0060の2解を個体ID付きで再生する。操作の生成やsolver実行はしない。"""

from collections import Counter, deque
from fractions import Fraction
import json
from pathlib import Path

from replay_slime_output import replay


ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "adhoc/score_cliff_strategy_review_20260928"
BINS = {"v022": "v022_dependency_lns", "v039": "v039_exact_board_lns"}
DIR = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}


def analyze(name):
    input_path = ROOT / "tools/in/0060.txt"
    output_path = ROOT / "results/out" / BINS[name] / "0060.txt"
    checked = replay(input_path, output_path)
    lines = input_path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    board = lines[1:N + 1]
    towers, nests, tokens = {}, {}, {}
    color_number = Counter()
    for i, row in enumerate(board):
        for j, c in enumerate(row):
            if c == "#":
                continue
            towers[i, j] = []
            if c.islower():
                color_number[c] += 1
                label = f"{c}{color_number[c]}"
                towers[i, j].append(label)
                tokens[label] = {"color": c, "start": [i, j], "moves": 0,
                                 "W": 0, "away": 0, "cost": Fraction(),
                                 "support_long": [], "trajectory": []}
            elif c.isupper():
                nests[c.lower()] = (i, j)
    distances = {}
    for c, p in nests.items():
        dist = {p: 0}
        queue = deque([p])
        while queue:
            u = queue.popleft()
            for di, dj in DIR.values():
                q = u[0] + di, u[1] + dj
                if q in towers and q not in dist:
                    dist[q] = dist[u] + 1
                    queue.append(q)
        distances[c] = dist
    operations, snapshots = [], []
    def snapshot():
        return {f"{p[0]},{p[1]}": list(ids) for p, ids in towers.items() if ids}
    snapshots.append(snapshot())
    for t, base in enumerate(checked["operations"]):
        i, j, k, d, length = base["action"].split()
        i, j, k, length = map(int, (i, j, k, length))
        p = i, j
        q = tuple(base["destination"])
        moving, support = list(towers[p][k:]), list(towers[p][:k])
        landing = list(towers[q])
        for token in moving:
            info = tokens[token]
            info["moves"] += 1
            info["W"] += length
            # 長いジャンプの途中で巣からの距離が増減しうるため、1マスずつ数える。
            di, dj = DIR[d]
            for step in range(length):
                u = i + di * step, j + dj * step
                v = u[0] + di, u[1] + dj
                info["away"] += max(0, distances[info["color"]][v] - distances[info["color"]][u])
            info["cost"] += Fraction(1, len(moving))
            info["trajectory"].append({"turn": t, "source": p, "destination": q,
                                       "moving": moving, "support": support})
        if length > 1:
            for token in support:
                tokens[token]["support_long"].append(t)
        towers[p] = list(support)
        towers[q].extend(reversed(moving))
        returned = []
        for cell in (p, q):
            while towers[cell] and nests[tokens[towers[cell][-1]]["color"]] == cell:
                token = towers[cell].pop()
                tokens[token]["returned"] = t
                returned.append(token)
        # 既存の色だけの再生器と、全操作で移動束と出発・到着マスを照合する。
        assert "".join(tokens[x]["color"] for x in moving) == base["moving_bottom_to_top"]
        for cell in (p, q):
            assert "".join(tokens[x]["color"] for x in towers[cell]) == base["after"][str(cell)]
        operations.append({"turn": t, "source": p, "destination": q,
                           "k": k, "length": length, "moving": moving,
                           "support": support, "landing": landing, "returned": returned})
        snapshots.append(snapshot())
    assert not any(towers.values())
    assert sum(x["cost"] for x in tokens.values()) == len(operations)
    assert sum(x["W"] for x in tokens.values()) == checked["metrics"]["W"]
    for info in tokens.values():
        info["shortest"] = distances[info["color"]][tuple(info["start"])]
        assert info["W"] == info["shortest"] + 2 * info["away"]
        info["cost_exact"] = str(info["cost"])
        info["cost"] = float(info["cost"])
    regions = Counter()
    for o in operations:
        p, q = o["source"], o["destination"]
        if max(p[0], q[0]) > 9:
            region = "southeast" if min(p[1], q[1]) >= 10 else "southwest"
        else:
            region = "central_row9" if p[0] == q[0] == 9 else "north"
        regions[region] += 1
    assert sum(regions.values()) == len(operations)
    # v022の南東で同じ塔に合流した、A2匹とC/E/H/Jの計6匹を追う。
    # この6匹が改善の必要最小集合だという意味ではない。
    selected = {"a1", "a2", "c7", "e1", "h10", "j1"}
    boundaries = []
    for applied_count, state in enumerate(snapshots):
        cells = {p: [x for x in ids if x in selected]
                 for p, ids in state.items() if any(x in selected for x in ids)}
        if sum(map(len, cells.values())) == len(selected):
            boundaries.append({"after_turn": applied_count - 1, "tower_count": len(cells), "cells": cells})
    transcript = []
    for o in operations:
        transcript.append(f"{o['turn']:3} {str(o['source']):8} -> {str(o['destination']):8} "
                          f"move={','.join(o['moving']):30} "
                          f"support={','.join(o['support']):24} "
                          f"land={','.join(o['landing']):24} "
                          f"return={','.join(o['returned'])}")
    (DEST / f"case0060_{name}_identity.txt").write_text("\n".join(transcript) + "\n")
    return {"bin": BINS[name], "indexing": "座標とturnは0-based。snapshots[t]はt手適用後。",
            "metrics": checked["metrics"], "tokens": tokens,
            "operations": operations, "snapshots": snapshots, "regions": dict(regions),
            "southeast_six": {"minimum_towers": min(x["tower_count"] for x in boundaries),
                              "all_alive_boundaries": boundaries}}


def ordinary_candidates(report):
    """v039の通常除去集合と依存拡張だけを保存軌跡から集計する。再挿入はしない。"""
    rows = (ROOT / "tools/in/0060.txt").read_text().splitlines()[1:]
    floor = {(i, j) for i, row in enumerate(rows) for j, c in enumerate(row) if c != "#"}
    nests = {c.lower(): (i, j) for i, row in enumerate(rows) for j, c in enumerate(row) if c.isupper()}
    ids = list(report["tokens"])
    id_order = {token: index for index, token in enumerate(ids)}
    dist = {}
    for p in floor:
        dist[p] = {p: 0}
        queue = deque([p])
        while queue:
            u = queue.popleft()
            for di, dj in DIR.values():
                q = u[0] + di, u[1] + dj
                if q in floor and q not in dist[p]:
                    dist[p][q] = dist[p][u] + 1
                    queue.append(q)
    candidates = set()
    def add(tokens):
        if 0 < len(tokens) <= 12:
            candidates.add(frozenset(tokens))
    for token in ids:
        add([token])
    for c in nests:
        add([token for token in ids if token.startswith(c)])
    for token in ids:
        p = tuple(report["tokens"][token]["start"])
        other = []
        for qtoken in ids:
            if token == qtoken:
                continue
            q = tuple(report["tokens"][qtoken]["start"])
            score = dist[p][q] + .20 * dist[nests[token[0]]][nests[qtoken[0]]]
            other.append((score, id_order[qtoken], qtoken))
        other.sort()
        for seq, limit in [(other, 2), ([x for x in other if x[2][0] == token[0]], 2),
                           ([x for x in other if x[2][0] != token[0]], 1)]:
            selected = [token]
            for _, _, qtoken in seq[:limit]:
                selected.append(qtoken)
                add(selected)
    uses = []
    for o in report["operations"]:
        add(o["moving"])
        run = []
        for token in o["support"] + o["moving"]:
            if run and token[0] != run[-1][0]:
                add(run)
                run = []
            run.append(token)
        add(run)
        add(o["support"])
        if o["landing"] and len(o["landing"]) + len(o["moving"]) <= 8:
            add(o["landing"] + o["moving"])
        if o["length"] > 1:
            uses.append((set(o["support"]), set(o["moving"]), o["k"] + 1 - o["length"], o["length"]))
    expanded = set()
    for seed in candidates:
        selected = set(seed)
        while len(selected) < 8:
            options = []
            for index, (lower, flight, slack, length) in enumerate(uses):
                added = flight - selected
                if len(lower & selected) > slack and added and len(selected | added) <= 8:
                    options.append((len(added), -length, index, added))
            if not options:
                break
            selected |= min(options, key=lambda x: x[:3])[3]
        expanded.add(frozenset(selected))
    result = {"scope": "v039完成解での通常除去集合と依存拡張。優先度、時間予算、2塔近傍は含めない。",
              "ordinary_count": len(candidates), "expanded_distinct_count": len(expanded)}
    for label, required in [("a2_and_e1", {"a2", "e1"}),
                            ("southeast_six", {"a1", "a2", "c7", "e1", "h10", "j1"})]:
        result[label] = {"required": sorted(required),
                         "ordinary_supersets": sum(required <= c for c in candidates),
                         "expanded_supersets": sum(required <= c for c in expanded)}
    result["sets_containing_a2"] = [sorted(c, key=id_order.get) for c in sorted(candidates | expanded, key=lambda x: tuple(sorted(x))) if "a2" in c]
    return result


def main():
    DEST.mkdir(parents=True, exist_ok=True)
    reports = {name: analyze(name) for name in BINS}
    (DEST / "case0060_pair.json").write_text(json.dumps(reports, ensure_ascii=False, indent=2) + "\n")
    candidates = ordinary_candidates(reports["v039"])
    (DEST / "case0060_v039_candidates.json").write_text(json.dumps(candidates, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(candidates, ensure_ascii=False, indent=2))
    print("ID start shortest | v039 moves,W,cost | v022 moves,W,cost | difference(cost)")
    for token, a in reports["v039"]["tokens"].items():
        b = reports["v022"]["tokens"][token]
        print(f"{token:3} {str(a['start']):8} {a['shortest']:2} | "
              f"{a['moves']:2},{a['W']:2},{a['cost']:5.2f} | "
              f"{b['moves']:2},{b['W']:2},{b['cost']:5.2f} | {a['cost']-b['cost']:+6.2f}")


if __name__ == "__main__":
    main()
