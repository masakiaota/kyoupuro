#!/usr/bin/env python3
"""Audit an existing 43-move witness against v014's constraints; no solving."""

from collections import deque
import json
from pathlib import Path

from replay_slime_output import replay

ROOT = Path(__file__).resolve().parents[2]


def main():
    source = ROOT / "tools/in/0000.txt"
    witness = ROOT / "adhoc/x_uta_case0000/tsukammo_43.txt"
    data = replay(source, witness)
    lines = source.read_text().splitlines()
    N, K = map(int, lines[0].split())
    grid = lines[1:N + 1]
    board = {(i, j): (ch if ch.islower() else "")
             for i, row in enumerate(grid) for j, ch in enumerate(row) if ch != "#"}
    nest = {(i, j): ch.lower() for i, row in enumerate(grid)
            for j, ch in enumerate(row) if ch.isupper()}
    homes = {c: v for v, c in nest.items()}
    states = [dict(board)]
    directions = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}
    for op in data["operations"]:
        u, v = tuple(op["source"]), tuple(op["destination"])
        k = int(op["action"].split()[2])
        word = board[u][k:]
        board[u] = board[u][:k]
        board[v] += word[::-1]
        for at in (u, v):
            while board[at] and board[at][-1] == nest.get(at):
                board[at] = board[at][:-1]
        states.append(dict(board))
    distance = {}
    for source_cell in board:
        values = {source_cell: 0}
        queue = deque([source_cell])
        while queue:
            u = queue.popleft()
            for di, dj in directions.values():
                v = u[0] + di, u[1] + dj
                if v in board and v not in values:
                    values[v] = values[u] + 1
                    queue.append(v)
        distance[source_cell] = values

    def estimate(pending, at, word):
        points = [at] if word else []
        colors = set(word)
        for v, w in pending:
            points.append(v)
            colors.update(w)
        points += [homes[c] for c in sorted(colors)]
        reached, left = {0}, set(range(1, len(points)))
        total = 0
        while left:
            cost, j = min((distance[points[i]][points[j]], j) for i in reached for j in left)
            total += cost
            reached.add(j)
            left.remove(j)
        return 0.65 * total

    pending_blue = [(v, w) for v, w in states[29].items() if w == "b"]
    landing = []
    u = (9, 9)
    for di, dj in directions.values():
        for length in range(1, 6):
            v = u[0] + di * length, u[1] + dj * length
            if v not in board:
                break
            if len(states[29][v]) + 2 <= 8:
                landing.append({"cell": v, "estimate": estimate(pending_blue, v, "bb")})
    landing.sort(key=lambda a: (a["estimate"], a["cell"]))
    for rank, choice in enumerate(landing, 1):
        choice["rank"] = rank
    blue_before_green_home = [(v, w) for v, w in states[22].items() if w == "b"]
    assert states[14][9, 1] == "ccccd" and states[14][11, 1] == "d"
    assert states[16][9, 1] == "ccccdd"
    assert states[28][9, 8] == "bbdddd" and states[29][9, 9] == "ddddbb"
    assert states[39][5, 0] == states[4][5, 0] == "aaa"
    assert sum(len(w) for w in states[39].values()) == 4
    report = {
        "witness_metrics": data["metrics"],
        "closed_delivery": {"prefix_moves": 4, "delivery_moves": 35, "suffix_moves": 4,
                            "colors": "bcd", "slimes": 12, "max_packet": 8},
        "incoming_merge": {"turns": [15, 16], "parked": [9, 1], "incoming_start": [11, 1],
                           "before": "ccccd", "after": "ccccdd", "v014_transition_exists": False},
        "empty_split_site": {"transfer_turn": 29, "split_turn": 30, "last_pickup": [9, 8],
                             "split_site": [9, 9], "before": "bbdddd", "after": "ddddbb",
                             "v014_transfer_without_pickup_or_home_exists": False},
        "split_landing_ranks_if_state_were_available": landing,
        "order_blind_estimate": {
            word: estimate(blue_before_green_home, (6, 4), word)
            for word in ("ccccdddd", "cdcdcdcd")},
        "saved_run": {"constructed": 10, "adopted": 0, "timeouts": 0,
                      "cooperative_phase_ms": 14.406, "T": 49},
        "limits": ["The old run did not save candidate membership, cost, or beam ranks.",
                   "This audit proves witness exclusions, not the optimum of v014's entire search space."]}
    target = ROOT / "adhoc/v014_case0000_constraint_audit.json"
    target.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
