#!/usr/bin/env python3
"""固定済み0060を対象に、事前登録した再挿入の診断を1回実行して検証する。"""

import csv
from collections import Counter, defaultdict
from datetime import datetime
import fcntl
from hashlib import sha256
import json
from math import factorial
from pathlib import Path
import subprocess
import time

from replay_slime_output import replay


ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "adhoc/v042_three_tower"
RUN = DEST / "run"
INPUT = ROOT / "tools/in/0060.txt"
ORIGINAL = ROOT / "results/out/v039_exact_board_lns/0060.txt"
OLDER = ROOT / "results/out/v022_dependency_lns/0060.txt"
TARGETS = DEST / "targets.txt"
EXE = ROOT / "target/release/v042_three_tower_probe"
SOURCES = [ROOT / "src/bin/v039_exact_board_lns.cpp",
           ROOT / "adhoc/bin/v042_three_tower_probe.cpp", Path(__file__).resolve(),
           ROOT / "notes/experiments/v042.md", INPUT, ORIGINAL, OLDER, TARGETS]


def hashes():
    return {str(p.relative_to(ROOT)): sha256(p.read_bytes()).hexdigest() for p in SOURCES}


def motif(path):
    """A2匹を残して対象の別色を北へ長距離輸送した操作を、個体IDで確認する。"""
    rows = INPUT.read_text().splitlines()[1:]
    board = {(i, j): [(i, j)] if c.islower() else []
             for i, row in enumerate(rows) for j, c in enumerate(row) if c != "#"}
    color = {(i, j): c for i, row in enumerate(rows) for j, c in enumerate(row) if c.islower()}
    nest = {(i, j): c.lower() for i, row in enumerate(rows) for j, c in enumerate(row) if c.isupper()}
    target_rows = [line.split() for line in TARGETS.read_text().splitlines()]
    selected = {(int(i), int(j)) for _, i, j, _ in target_rows}
    a_ids = {p for p in selected if color[p] == "a"}
    directions = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}
    found = []
    for turn, line in enumerate(path.read_text().splitlines()):
        i, j, k, d, length = line.split()
        i, j, k, length = map(int, (i, j, k, length))
        p = i, j
        di, dj = directions[d]
        q = i + di * length, j + dj * length
        support, moving = board[p][:k], board[p][k:]
        if d == "U" and length >= 2 and a_ids <= set(support) and (set(moving) & (selected - a_ids)):
            found.append({"turn": turn, "source": p, "destination": q,
                          "support": "".join(color[t] for t in support),
                          "moving": "".join(color[t] for t in moving)})
        board[p] = list(support)
        board[q].extend(reversed(moving))
        for cell in (p, q):
            while board[cell] and color[board[cell][-1]] == nest.get(cell):
                board[cell].pop()
    assert not any(board.values())
    return found


def verify():
    metadata = json.loads((RUN / "metadata.json").read_text())
    with (RUN / "scenarios.csv").open() as handle:
        scenarios = {row["scenario"]: row for row in csv.DictReader(handle)}
    lengths, motifs = {}, {}
    for path in sorted((RUN / "unique").glob("*.txt"), key=lambda p: int(p.stem)):
        checked = replay(INPUT, path)
        assert checked["metrics"]["E"] == 0
        lengths[int(path.stem)] = checked["metrics"]["T"]
        motifs[int(path.stem)] = motif(path)
    assert len(lengths) == metadata["unique_outputs"]
    groups = defaultdict(lambda: {"attempts": 0, "completed": 0, "statuses": Counter(),
                                  "raw_improved": 0, "polished_improved": 0,
                                  "improved_with_motif": 0, "best_T": None, "best_trials": [],
                                  "elapsed_us": 0.0})
    signatures = set()
    per_scenario = Counter()
    reference = metadata["reference_T"]
    with (RUN / "trials.csv").open() as handle:
        for row in csv.DictReader(handle):
            scenario = scenarios[row["scenario"]]
            kind = "initial" if scenario["kind"] == "initial" else f"{scenario['towers']}_tower"
            group = groups[kind]
            key = row["scenario"], row["support"], row["ride"], row["order"]
            assert key not in signatures
            signatures.add(key)
            assert sorted(row["order"].split("|")) == sorted(scenario["ids"].split("|"))
            per_scenario[row["scenario"]] += 1
            group["attempts"] += 1
            group["elapsed_us"] += float(row["elapsed_us"])
            group["statuses"][row["status"]] += 1
            if row["status"] != "ok":
                continue
            raw, polished = int(row["raw_id"]), int(row["polished_id"])
            raw_T, polished_T = int(row["raw_T"]), int(row["polished_T"])
            assert raw_T == lengths[raw] and polished_T == lengths[polished]
            assert polished_T <= raw_T <= int(scenario["time"]) + int(scenario["allowance"])
            assert int(row["inserted"]) == len(scenario["ids"].split("|"))
            group["completed"] += 1
            group["raw_improved"] += raw_T < reference
            group["polished_improved"] += polished_T < reference
            group["improved_with_motif"] += polished_T < reference and bool(motifs[polished])
            if group["best_T"] is None or polished_T < group["best_T"]:
                group["best_T"] = polished_T
                group["best_trials"] = []
            if polished_T == group["best_T"] and len(group["best_trials"]) < 5:
                group["best_trials"].append({**row, "motif": motifs[polished]})
    for name, scenario in scenarios.items():
        assert per_scenario[name] == 5 * factorial(len(scenario["ids"].split("|")))
    assert len(signatures) == metadata["expected_trials"] == metadata["actual_trials"]
    assert sum(g["completed"] for g in groups.values()) == metadata["completed"]
    triple = groups["3_tower"]["best_T"]
    controls = [g["best_T"] for name, g in groups.items() if name in {"1_tower", "2_tower"} and g["best_T"] is not None]
    strict_triple_effect = triple is not None and triple < reference and (not controls or triple < min(controls))
    official = {}
    ids = {metadata["original_id"], metadata["reference_id"]}
    for group in groups.values():
        ids.update(int(t["polished_id"]) for t in group["best_trials"])
    scorer = ROOT / "tools/target/release/vis"
    for id_ in sorted(ids):
        result = subprocess.run([str(scorer), str(INPUT), str(RUN / "unique" / f"{id_}.txt"), "--no-vis"],
                                cwd=ROOT, check=True, capture_output=True, text=True)
        assert result.stdout.strip() == f"Score = {lengths[id_]}", result.stdout
        official[id_] = lengths[id_]
    result = {"metadata": metadata, "groups": dict(groups), "strict_three_tower_effect": strict_triple_effect,
              "independently_validated_outputs": len(lengths), "official_best_scores": official,
              "v039_motif": motif(ORIGINAL), "v022_motif": motif(OLDER)}
    (DEST / "verification.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    csv_path = ROOT / "results/analysis/v042_three_tower_summary.csv"
    csv_path.parent.mkdir(parents=True, exist_ok=True)
    with csv_path.open("w") as handle:
        writer = csv.writer(handle)
        writer.writerow(["condition", "attempts", "completed", "best_T", "reference_T", "polished_improved", "improved_with_motif"])
        for name, group in groups.items():
            writer.writerow([name, group["attempts"], group["completed"], group["best_T"], reference,
                             group["polished_improved"], group["improved_with_motif"]])
    print(json.dumps({"groups": {name: {k: v for k, v in g.items() if k != "best_trials"} for name, g in groups.items()},
                      "strict_three_tower_effect": strict_triple_effect,
                      "unique_outputs": len(lengths)}, ensure_ascii=False, indent=2))


def main():
    assert EXE.is_file()
    assert not (DEST / "manifest.json").exists(), "This diagnostic has already been started."
    before = hashes()
    manifest = {"started_at": datetime.now().astimezone().isoformat(), "sha256": before,
                "binary_sha256": sha256(EXE.read_bytes()).hexdigest(),
                "command": [str(EXE), str(INPUT), str(ORIGINAL), str(TARGETS), str(RUN)]}
    with (ROOT / "results/.eval.lock").open("a+") as lock:
        print("Waiting for the evaluation lock", flush=True)
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        lock.seek(0);lock.truncate()
        lock.write(json.dumps({"bin": "v042_three_tower_probe", "started_at": manifest["started_at"]}) + "\n")
        lock.flush()
        (DEST / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
        begin = time.monotonic()
        with (DEST / "run.log").open("w") as stdout, (DEST / "run.err").open("w") as stderr:
            subprocess.run(manifest["command"], cwd=ROOT, stdout=stdout, stderr=stderr, check=True)
        manifest["elapsed_sec"] = time.monotonic() - begin
        assert hashes() == before, "A registered source changed during execution."
        manifest["hashes_unchanged_after_run"] = True
        (DEST / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
        print(f"Diagnostic completed in {manifest['elapsed_sec']:.3f} seconds", flush=True)
        verify()
    print("All registered trials and independent validation finished.")


if __name__ == "__main__":
    main()
