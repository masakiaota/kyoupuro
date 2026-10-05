#!/usr/bin/env python3
"""Inspect saved v026 plans for dependency-aware paired suffix candidates.

No solver runs or reconstruction calls. This measures structural availability,
not candidate improvements or the distribution seen during the actual search.
"""
from collections import Counter, deque
import csv
import hashlib
import json
from pathlib import Path

from analyze_v026_late_starts import replay, saved_moves

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/merge_candidate_analysis"
DIRS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}


def inspect(case):
    moves = saved_moves(case)
    groups, _ = replay(case, moves, collect=True)
    primary, later = set(), set()
    for appearances in groups.values():
        items = sorted(appearances.values(), key=lambda a: a["time"])
        primary.add(items[0]["time"])
        if len(items) > 1:
            late = max(items[1:], key=lambda a: (a["split"], a["time"]))
            later.add(late["time"])
    anchors = primary | later
    lines = (ROOT / "tools/in" / case).read_text().splitlines()
    N, K = map(int, lines[0].split())
    grid = "".join(lines[1:N+1])
    floor = [p for p, c in enumerate(grid) if c != "#"]
    colors = {p: ord(c)-ord("a") for p, c in enumerate(grid) if "a" <= c <= "z"}
    homes = {ord(c)-ord("A"): p for p, c in enumerate(grid) if "A" <= c <= "Z"}
    nest = {p: c for c, p in homes.items()}
    assert len(homes) == K
    board = {p: [p] for p in colors}
    states, supports = {}, []
    for t, (row, col, keep, direction, length) in enumerate(moves):
        p = row*N+col
        if t in anchors:
            states[t] = {cell: tuple(ids) for cell, ids in board.items() if ids}
        stack = board[p]
        if length > 1:
            supports.append((t, set(stack[:keep]), set(stack[keep:]), keep+1-length))
        dr, dc = DIRS[direction]
        q = (row+dr*length)*N+col+dc*length
        flying = stack[keep:]
        board[p] = stack[:keep]
        board.setdefault(q, []).extend(reversed(flying))
        for cell in (p, q):
            while board[cell] and colors[board[cell][-1]] == nest.get(cell, -1):
                board[cell].pop()
    assert not any(board.values())

    adjacency = {}
    for p in floor:
        r, c = divmod(p, N)
        adjacency[p] = [(r+dr)*N+c+dc for dr, dc in DIRS.values()
                        if 0 <= r+dr < N and 0 <= c+dc < N and grid[(r+dr)*N+c+dc] != "#"]
    distances = {}

    def dist(source):
        if source not in distances:
            result = {source: 0}
            queue = deque([source])
            while queue:
                u = queue.popleft()
                for v in adjacency[u]:
                    if v not in result:
                        result[v] = result[u]+1
                        queue.append(v)
            distances[source] = result
        return distances[source]

    count, rows = Counter(), []
    for limit in (6, 8):
        prefix = f"limit_{limit}_"
        for time, state in sorted(states.items()):
            row, col, _, direction, length = moves[time]
            p = row*N+col
            hp = len(state[p])

            def keep_for(cell, maximum):
                ids = state[cell]
                return next((k for k in range(max(0, len(ids)-maximum), len(ids))
                             if k == 0 or colors[ids[k-1]] != nest.get(cell, -1)), None)

            kp = keep_for(p, min(hp, limit-1))
            if kp is None:
                continue
            removed = set(state[p][kp:])
            positions = {token: cell for cell, ids in state.items() for token in ids}
            count[prefix+"anchors"] += 1
            if time in later:
                count[prefix+"anchors_in_late_slots"] += 1
            dr, dc = DIRS[direction]
            first_destination = (row+dr*length)*N+col+dc*length
            pc = {colors[token] for token in removed}
            partners = []
            for q in state:
                if q == p or dist(p)[q] > 10:
                    continue
                kq = keep_for(q, min(len(state[q]), limit-len(removed)))
                if kq is None:
                    continue
                qc = {colors[token] for token in state[q][kq:]}
                home_distance = min(dist(homes[a])[homes[b]] for a in pc for b in qc)
                score = dist(p)[q]+0.22*home_distance-0.45*(q == first_destination)
                partners.append((score, q, kq))
            partners.sort()
            rank = {q: i+1 for i, (_, q, _) in enumerate(partners)}
            matched = {}
            for use_time, lower, flight, slack in supports:
                if use_time < time or len(lower & removed) <= slack:
                    continue
                need = flight-removed
                if not need:
                    continue
                count[prefix+"broken_surviving_flights"] += 1
                places = {positions[token] for token in need}
                if len(places) != 1:
                    continue
                q = next(iter(places))
                if q == p:
                    continue
                kq = keep_for(q, min(len(state[q]), limit-len(removed)))
                if kq is None or not need <= set(state[q][kq:]):
                    continue
                if q not in matched:
                    matched[q] = {"first_use": use_time, "keep_q": kq, "uses": 0}
                matched[q]["uses"] += 1
            nearby = {q: item for q, item in matched.items() if q in rank}
            if matched:
                count[prefix+"anchors_with_two_tower_dependency"] += 1
            if nearby:
                count[prefix+"anchors_with_current_eligible_partner"] += 1
                if time in later:
                    count[prefix+"late_anchors_with_current_eligible_partner"] += 1
            for q, item in matched.items():
                count[prefix+"dependency_pairs"] += 1
                if q in rank:
                    count[prefix+"current_eligible_pairs"] += 1
                    count[prefix+"rank_above_12_pairs"] += rank[q] > 12
                    count[prefix+"rank_above_3_pairs"] += rank[q] > 3
                rows.append({"case": case, "limit": limit, "time": time, "p_row": row, "p_col": col,
                             "keep_p": kp, "q_row": q//N, "q_col": q%N, "keep_q": item["keep_q"],
                             "first_use_time": item["first_use"], "supported_flights": item["uses"],
                             "distance": dist(p)[q], "current_partner_rank": rank.get(q, -1),
                             "late_slot": int(time in later)})
    return count, rows


def main():
    OUT.mkdir(exist_ok=True)
    total, normal, per_case, rows, hashes = Counter(), Counter(), {}, [], {}
    for path in sorted((ROOT / "tools/in").glob("*.txt")):
        count, pairs = inspect(path.name)
        per_case[path.name] = count
        rows.extend(pairs)
        total.update(count)
        if path.name != "0000.txt":
            normal.update(count)
        for p in (path, ROOT / "results/out/v026_late_start_lns" / path.name):
            hashes[str(p.relative_to(ROOT))] = hashlib.sha256(p.read_bytes()).hexdigest()
    with (OUT / "dependency_pairs.csv").open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    source = ROOT / "src/bin/v026_late_start_lns.cpp"
    result = {"scope": "Replay only. No optimization, candidate reconstruction or score evaluation.",
              "bin": source.stem, "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
              "verified_cases": len(per_case), "all_100": total, "generated_99": normal,
              "eligible_cases_99": {str(limit): sum(c[f"limit_{limit}_anchors_with_current_eligible_partner"] > 0
                                   for name, c in per_case.items() if name != "0000.txt") for limit in (6, 8)},
              "cases": per_case, "input_hashes": hashes}
    (OUT / "summary.json").write_text(json.dumps(result, indent=2)+"\n")
    print(json.dumps({k: v for k, v in result.items() if k not in ("cases", "input_hashes")}, indent=2))


if __name__ == "__main__":
    main()
