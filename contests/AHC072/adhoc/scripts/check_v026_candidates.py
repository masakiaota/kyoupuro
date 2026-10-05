#!/usr/bin/env python3
"""Independently derive first/later cuts from saved output; run no solver."""
from collections import Counter, defaultdict
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / "adhoc/v026_audit"
DIRS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}
MASK = (1 << 64) - 1


def replay_candidates(case_name):
    lines = (ROOT / "tools/in" / case_name).read_text().splitlines()
    N, K = map(int, lines[0].split())
    grid = "".join(lines[1:N + 1])
    positions = [p for p, char in enumerate(grid) if char != "#"]
    cell_id = {p: i for i, p in enumerate(positions)}
    colors = {cell_id[p]: ord(c) - ord("a") + 1 for p, c in enumerate(grid) if c.islower()}
    nests = {cell_id[p]: ord(c) - ord("A") + 1 for p, c in enumerate(grid) if c.isupper()}
    assert len(nests) == K
    tower = [[i] if i in colors else [] for i in range(len(positions))]
    occurrences = defaultdict(dict)
    flights = []
    returned = set()
    output = (ROOT / "results/out/v022_dependency_lns" / case_name).read_text().splitlines()
    for time, line in enumerate(output):
        row, col, keep, direction, distance = line.split()
        row, col, keep, distance = map(int, (row, col, keep, distance))
        origin = row * N + col
        p = cell_id[origin]
        h = len(tower[p])
        assert 0 <= keep < h and 1 <= distance <= keep + 1
        di, dj = DIRS[direction]
        for step in range(1, distance + 1):
            nr, nc = row + step * di, col + step * dj
            assert 0 <= nr < N and 0 <= nc < N and grid[nr * N + nc] != "#"
        q = cell_id[(row + distance * di) * N + col + distance * dj]
        assert len(tower[q]) + h - keep <= 8
        for cut_keep in (0, keep, h - 2):
            if cut_keep < 0 or h - cut_keep < 2:
                continue
            if cut_keep and colors[tower[p][cut_keep - 1]] == nests.get(p):
                continue
            ids = tuple(sorted(tower[p][cut_keep:]))
            record = {"time": time, "keep": cut_keep, "size": len(ids), "split": int(cut_keep < keep)}
            if time in occurrences[ids]:
                assert occurrences[ids][time] == record
            occurrences[ids][time] = record
        flying = tower[p][keep:]
        flights.append(set(flying))
        tower[p] = tower[p][:keep]
        tower[q].extend(reversed(flying))
        for cell in (p, q):
            while tower[cell] and colors[tower[cell][-1]] == nests.get(cell):
                token = tower[cell].pop()
                assert token not in returned
                returned.add(token)
    assert not any(tower) and len(returned) == len(colors)
    expected = {}
    counts = Counter(groups=len(occurrences), multi_time_groups=0, later_split_groups=0,
                     later_nonsplit_groups=0, split_preferred_over_later_nonsplit=0,
                     delay_sum=0, occurrences=sum(len(v) for v in occurrences.values()))
    for ids, by_time in occurrences.items():
        ordered = sorted(by_time.values(), key=lambda r: r["time"])
        selected = [ordered[0]]
        if len(ordered) > 1:
            counts["multi_time_groups"] += 1
            later = max(ordered[1:], key=lambda r: (r["split"], r["time"]))
            selected.append(later)
            counts["later_split_groups" if later["split"] else "later_nonsplit_groups"] += 1
            counts["split_preferred_over_later_nonsplit"] += later["time"] != ordered[-1]["time"]
            counts["delay_sum"] += later["time"] - ordered[0]["time"]
        base_hash = 0x51ed270b3a4dc107
        for token in ids:
            base_hash = ((base_hash ^ (token + 1)) * 0x9e3779b97f4a7c15) & MASK
        base_hash ^= 0xd1b54a32d192ed03
        for slot, item in enumerate(selected):
            saved = sum(flight.issubset(ids) for flight in flights[item["time"]:])
            code = base_hash ^ (((item["time"] + 1) * 0x94d049bb133111eb) & MASK) if slot else base_hash
            expected[(ids, slot)] = {**item, "ids": list(ids), "saved": saved, "later": slot, "hash": code}
        if len(selected) == 2:
            assert expected[(ids, 0)]["hash"] != expected[(ids, 1)]["hash"]
    return expected, counts


def main():
    rows = [json.loads(line) for line in (AUDIT / "generated_cuts.jsonl").read_text().splitlines()]
    assert len(rows) == 100 and len({r["case"] for r in rows}) == 100
    summaries = []
    for row in rows:
        expected, counts = replay_candidates(row["case"])
        actual = {}
        for cut in row["cuts"]:
            key = (tuple(cut["ids"]), cut["later"])
            assert key not in actual
            actual[key] = cut
        assert expected == actual, row["case"]
        assert counts["groups"] <= len(actual) <= 2 * counts["groups"]
        summaries.append({"case": row["case"], **counts, "cuts": len(actual)})
    total = Counter()
    for row in summaries:
        total.update({k: v for k, v in row.items() if k != "case"})
    result = {"verified_cases": 100, "total": dict(total), "cases_with_later": sum(r["multi_time_groups"] > 0 for r in summaries),
              "cases_with_split": sum(r["later_split_groups"] > 0 for r in summaries), "cases": summaries}
    (AUDIT / "generation_check.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "cases"}, indent=2))


if __name__ == "__main__":
    main()
