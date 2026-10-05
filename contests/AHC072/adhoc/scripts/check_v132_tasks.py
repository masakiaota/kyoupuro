#!/usr/bin/env python3
"""Validate frozen diagnostic outputs only; never generate solution moves."""
import copy
import csv
import hashlib
import json
from pathlib import Path
import subprocess

from replay_slime_output import replay

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "results/nn_rank/v132/20261005_temporal_tasks_studio/diagnostic"
DIRECTIONS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}


def read_moves(path):
    result = []
    for line in path.read_text().splitlines():
        if line.strip():
            i, j, k, d, l = line.split()
            result.append((int(i), int(j), int(k), d, int(l)))
    return result


def identity_input(case):
    lines = (ROOT / f"tools/in/{case}.txt").read_text().splitlines()
    N, K = map(int, lines[0].split())
    state, nests, colors, origins = {}, {}, {}, {}
    for i, row in enumerate(lines[1:N+1]):
        for j, c in enumerate(row):
            if c == "#":
                continue
            state[i, j] = []
            if c.islower():
                token = len(colors)
                colors[token] = c
                origins[i, j] = token
                state[i, j].append(token)
            if c.isupper():
                nests[i, j] = c.lower()
    assert len(nests) == K
    return state, nests, colors, origins


def apply(state, nests, colors, move):
    i, j, k, d, length = move
    p = i, j
    assert p in state and 0 <= k < len(state[p])
    assert 1 <= length <= k+1 and d in DIRECTIONS
    di, dj = DIRECTIONS[d]
    for step in range(1, length+1):
        assert (i+di*step, j+dj*step) in state
    q = i+di*length, j+dj*length
    flight = state[p][k:]
    assert len(flight)+len(state[q]) <= 8
    state[p] = state[p][:k]
    state[q].extend(reversed(flight))
    for cell in (p, q):
        while state[cell] and colors[state[cell][-1]] == nests.get(cell):
            state[cell].pop()
    return flight


def prepare(case):
    state, nests, colors, origins = identity_input(case)
    moves = read_moves(ROOT / f"results/out/v039_exact_board_lns/{case}.txt")
    snapshots, flights = [copy.deepcopy(state)], []
    for move in moves:
        flights.append(apply(state, nests, colors, move))
        snapshots.append(copy.deepcopy(state))
    assert not any(state.values())
    return dict(nests=nests, colors=colors, origins=origins, snapshots=snapshots,
                moves=moves, flights=flights)


def admit(data, trial, path):
    selected = {data["origins"][tuple(point)] for point in trial["origins"]}
    boundary = trial["boundary"]
    state = copy.deepcopy(data["snapshots"][boundary])
    schedule = []
    for move, flight in zip(data["moves"][boundary:], data["flights"][boundary:]):
        fixed = [token for token in flight if token not in selected]
        if fixed:
            schedule.append((move, fixed))
    index = 0
    result = dict(accepted=0, operations=0, first_failure=-1, schedule_cursor=0,
                  reason="ok", selected_moves=0, background_moves=0, rides=0,
                  supports=0, skipped=0)
    for step, move in enumerate(path):
        p = move[:2]
        result["schedule_cursor"] = index
        try:
            after = copy.deepcopy(state)
            flight = apply(after, data["nests"], data["colors"], move)
        except (AssertionError, KeyError):
            result.update(first_failure=step, reason="illegal")
            return result
        fixed = [token for token in flight if token not in selected]
        if fixed:
            alive = {x for tower in state.values() for x in tower}
            allowed = False
            if index < len(schedule):
                scheduled_move, scheduled_ids = schedule[index]
                expected = [x for x in scheduled_ids if x in alive]
                allowed = fixed == expected and move[:2] == scheduled_move[:2] and move[3:] == scheduled_move[3:]
            if not allowed:
                result.update(first_failure=step, reason="fixed_schedule")
                return result
            result["background_moves"] += 1
            result["rides"] += sum(x in selected for x in flight)
            if move[4] > 1:
                result["supports"] += sum(x in selected for x in state[p][:move[2]])
            index += 1
        else:
            result["selected_moves"] += 1
        state = after
        alive = {x for tower in state.values() for x in tower}
        while index < len(schedule) and not any(x in alive for x in schedule[index][1]):
            index += 1
            result["skipped"] += 1
        result["operations"] += 1
    result["schedule_cursor"] = index
    result["accepted"] = int(not any(state.values()) and index == len(schedule))
    if not result["accepted"]:
        result["reason"] = "unfinished"
    return result


def main():
    frozen = json.loads((OUT / "sources_before.json").read_text())
    for name, digest in frozen.items():
        assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == digest, name
    settings = json.loads((OUT / "settings.json").read_text())
    trials = {row["tag"]: row for row in settings["trials"]}
    data = {trial["case"]: prepare(trial["case"]) for trial in trials.values()}
    admissions = list(csv.DictReader((OUT / "admissions.csv").open()))
    assert len(admissions) == 32
    checked_admissions = 0
    for row in admissions:
        if row["tag"].startswith("fixture_"):
            assert row["accepted"] == "1"
            continue
        trial = trials[row["tag"]]
        sample = data[trial["case"]]
        if row["type"] == "parent":
            path = sample["moves"][trial["boundary"]:]
        else:
            full = read_moves(ROOT / trial["reference"])
            assert full[:trial["boundary"]] == sample["moves"][:trial["boundary"]]
            path = full[trial["boundary"]:]
        independent = admit(sample, trial, path)
        for key, value in independent.items():
            actual = row[key] if isinstance(value, str) else int(row[key])
            assert value == actual, (row["tag"], row["type"], key, value, actual)
        checked_admissions += 1
    fixtures = list(csv.DictReader((OUT / "fixtures.csv").open()))
    assert len(fixtures) == 3 and all(3 <= int(row["best"]) <= limit for row, limit in zip(fixtures, (4, 3, 5)))
    rows = list(csv.DictReader((ROOT / "results/nn_rank/v132/20261005_temporal_tasks_studio/trials.csv").open()))
    assert len(rows) == len(trials) and {row["tag"] for row in rows} == set(trials)
    checked_plans = []
    scorer = ROOT / "tools/target/release/vis"
    assert scorer.is_file()
    for row in rows:
        trial = trials[row["tag"]]
        assert int(row["base_tail"])-int(row["best_tail"]) == int(row["saved"])
        assert int(row["labels"]) <= settings["label_limit"]
        if row["artifact"] == "-":
            assert int(row["saved"]) == 0
            continue
        output = ROOT / row["artifact"]
        input_path = ROOT / f"tools/in/{trial['case']}.txt"
        report = replay(input_path, output)
        assert report["metrics"]["E"] == 0
        assert report["metrics"]["T"] == trial["boundary"]+int(row["best_tail"])
        full = read_moves(output)
        assert full[:trial["boundary"]] == data[trial["case"]]["moves"][:trial["boundary"]]
        independent = admit(data[trial["case"]], trial, full[trial["boundary"]:])
        assert independent["accepted"]
        for key in ("selected_moves", "background_moves", "rides", "supports", "skipped"):
            assert independent[key] == int(row[key]), (row["tag"], key)
        scored = subprocess.run([str(scorer), str(input_path), str(output), "--no-vis"],
                                check=True, capture_output=True, text=True, cwd=OUT)
        score = int(scored.stdout.strip().splitlines()[-1].split("=")[-1])
        assert score == report["metrics"]["S"]
        checked_plans.append(dict(tag=row["tag"], score=score,
                                  output_sha256=hashlib.sha256(output.read_bytes()).hexdigest()))
    main_rows = [row for row in rows if row["control"] == "0"]
    summary = dict(trials=len(rows), main_trials=len(main_rows),
                   source_hashes_checked=len(frozen), admissions_independently_checked=checked_admissions,
                   fixtures=len(fixtures), plans=checked_plans,
                   improved_main_trials=sum(int(row["saved"]) > 0 for row in main_rows),
                   capped_main_trials=sum(row["status"] in ("label_cap", "expansion_cap") for row in main_rows),
                   best_saved_by_case={case: max(int(row["saved"]) for row in main_rows if row["case"] == case)
                                       for case in sorted(data)},
                   transition_checks=sum(int(row["transition_checks"]) for row in rows),
                   physical_checks=sum(int(row["physical_checks"]) for row in rows),
                   bound_checks=sum(int(row["bound_checks"]) for row in rows),
                   total_search_ms=sum(float(row["ms"]) for row in rows),
                   max_search_ms=max(float(row["ms"]) for row in rows))
    (OUT / "verification.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
