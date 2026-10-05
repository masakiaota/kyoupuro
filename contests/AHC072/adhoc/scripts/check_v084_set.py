#!/usr/bin/env python3
"""個体20特徴をPythonの独立再生と最短路計算で照合する。"""
from collections import deque
import json
import subprocess

import numpy as np

from check_v082_relation import replay, read_probe
from v084_data import ROOT, WIDTH, save, sha


def reference(text, state):
    lines = text.splitlines()
    N, _ = map(int, lines[0].split())
    grid = lines[1:N+1]
    cells = [(i, j) for i in range(N) for j in range(N) if grid[i][j] != "#"]
    cell_set = set(cells)
    colors, homes = {}, {}
    for identity, (r, c) in enumerate(cells):
        char = grid[r][c]
        if char.islower():
            colors[identity] = char
        if char.isupper():
            homes[char.lower()] = (r, c)
    directions = ((-1, 0), (1, 0), (0, -1), (0, 1))
    degree = lambda cell: sum((cell[0]+dr, cell[1]+dc) in cell_set for dr, dc in directions)
    distances = {}
    for color, home in homes.items():
        distance, queue = {home: 0}, deque([home])
        while queue:
            r, c = queue.popleft()
            for dr, dc in directions:
                point = (r+dr, c+dc)
                if point in cell_set and point not in distance:
                    distance[point] = distance[r, c]+1
                    queue.append(point)
        distances[color] = distance
    record = replay(text, state)
    T = max(1, record["T"])
    ids = sorted(colors)
    rows = []
    for identity in ids:
        color, cell = colors[identity], cells[identity]
        home = homes[color]
        times = record["history"][identity]
        x = np.zeros(WIDTH, np.float64)
        x[:4] = (distances[color][cell]/N, (abs(cell[0]-home[0])+abs(cell[1]-home[1]))/N,
                 degree(cell)/4, degree(home)/4)
        x[4:9] = (len(times)/T, record["travel"][identity]/N,
                  times[0]/T if times else 0, times[-1]/T if times else 0,
                  max((b-a for a, b in zip(times, times[1:])), default=0)/T)
        passenger_counts, ranks, lower_heights, support_slacks = [], [], [], []
        solo = same = mixed = long = supports = supported = others = 0
        for t, length, slack, passengers, supporters in record["events"]:
            if identity in passengers:
                peer_count = len(passengers)-1
                passenger_counts.append(peer_count)
                ranks.append(passengers.index(identity))
                lower_heights.append(state["current"][t][1])
                solo += peer_count == 0
                same += sum(p != identity and colors[p] == color for p in passengers)
                others += peer_count
                mixed += any(colors[p] != color for p in passengers)
                long += length > 1
            if length > 1 and identity in supporters:
                supports += 1
                supported += len(passengers)
                support_slacks.append(slack)
        count = max(1, len(times))
        x[9:] = (sum(passenger_counts)/count/7, max(passenger_counts, default=0)/7,
                 solo/count, same/others if others else 0, mixed/count, long/count,
                 supports/T, supported/T, min(support_slacks, default=8)/8,
                 sum(lower_heights)/count/8, sum(ranks)/count/7)
        rows.append(x)
    return ids, np.asarray(rows)


def self_test():
    text = "3 1\n.aA\n...\n...\n"
    ids, raw = reference(text, {"current": [[1, 0, 3, 1]]})
    expected = [1/3, 1/3, 3/4, 2/4, 1, 1/3, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0]
    assert ids == [1]
    np.testing.assert_allclose(raw[0], expected, rtol=0, atol=1e-14)
    text = "5 1\n.....\n.aaa.\n.a...\n.....\n....A\n"
    moves = [[7,0,2,1], [8,0,2,1], [7,0,2,1], [11,0,0,1], [6,2,3,3]]
    ids, raw = reference(text, {"current": moves})
    assert ids == [6, 7, 8, 11]
    np.testing.assert_allclose(raw[0, 15:18], [1/5, 2/5, 0])
    np.testing.assert_allclose(raw[2, 4:9], [3/5, 1, 1/5, 4/5, 2/5])
    np.testing.assert_allclose(raw[2, 9:15], [1/21, 1/7, 2/3, 1, 0, 1/3])
    np.testing.assert_allclose(raw[2, 18:], [1/12, 0])
    np.testing.assert_allclose(raw[3, 19], 1/14)
    return {"passed": True, "single_home": True, "support_and_bundle_flip": True}


def check(run, extractor, deadline):
    import time
    result = {"self_test": self_test(), "groups": 0, "items": 0, "max_error": 0., "files": {}}
    for item in json.loads((run / "probe_manifest.json").read_text()):
        path = run / "saved_probes" / f"{item['stem']}.input"
        text, states = read_probe(path)
        output = subprocess.check_output([str(extractor)], input=path.read_text(), text=True,
                                         timeout=max(1, min(30, deadline-time.time())), cwd=ROOT)
        actual = [json.loads(line) for line in output.splitlines()]
        assert len(actual) == len(states)
        for got, state in zip(actual, states):
            ids, expected = reference(text, state)
            assert got["phase"] == state["phase"] and got["item_ids"] == ids
            np.testing.assert_allclose(got["item_features"], expected, rtol=1e-12, atol=1e-10)
            result["max_error"] = max(result["max_error"], float(np.max(np.abs(np.asarray(got["item_features"])-expected))))
            result["groups"] += 1
            result["items"] += len(ids)
        result["files"][path.name] = sha(path)
    result["passed"] = True
    save(run / "individual_feature_check.json", result)
    return result
