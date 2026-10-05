#!/usr/bin/env python3
"""Analyze fixed-answer start-time diagnostics and independently replay results."""

from collections import Counter, defaultdict
import csv
import hashlib
import json
from pathlib import Path
from statistics import mean, median

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v026_behavior_analysis"
KINDS = ("packet", "flexible")
DIRS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}
MASK = (1 << 64) - 1


def set_hash(ids):
    value = 0x51ed270b3a4dc107
    for token in ids:
        value = ((value ^ (token + 1)) * 0x9e3779b97f4a7c15) & MASK
    return str(value)


def replay(case_name, moves, collect=False):
    lines = (ROOT / "tools/in" / case_name).read_text().splitlines()
    N, K = map(int, lines[0].split())
    grid = "".join(lines[1:N + 1])
    positions = [p for p, c in enumerate(grid) if c != "#"]
    cell_id = {p: i for i, p in enumerate(positions)}
    colors = {cell_id[p]: ord(c) - ord("a") + 1 for p, c in enumerate(grid) if c.islower()}
    nests = {cell_id[p]: ord(c) - ord("A") + 1 for p, c in enumerate(grid) if c.isupper()}
    assert len(nests) == K
    tower = [[i] if i in colors else [] for i in range(len(positions))]
    groups = defaultdict(dict)
    flights, returned = [], set()
    for time, move in enumerate(moves):
        row, col, keep, direction, distance = move
        p = cell_id[row * N + col]
        h = len(tower[p])
        assert 0 <= keep < h and 1 <= distance <= keep + 1
        di, dj = DIRS[direction]
        for step in range(1, distance + 1):
            nr, nc = row + step * di, col + step * dj
            assert 0 <= nr < N and 0 <= nc < N and grid[nr * N + nc] != "#"
        q = cell_id[(row + distance * di) * N + col + distance * dj]
        assert len(tower[q]) + h - keep <= 8
        if collect:
            for cut_keep in (0, keep, h - 2):
                if cut_keep < 0 or h - cut_keep < 2:
                    continue
                if cut_keep and colors[tower[p][cut_keep - 1]] == nests.get(p):
                    continue
                ids = tuple(sorted(tower[p][cut_keep:]))
                item = {"time": time, "keep": cut_keep, "source": p, "size": len(ids),
                        "split": int(cut_keep < keep), "colors": len({colors[i] for i in ids})}
                if time in groups[ids]:
                    assert groups[ids][time] == item
                groups[ids][time] = item
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
    return groups, flights


def saved_moves(case_name):
    lines = (ROOT / "results/out/v026_late_start_lns" / case_name).read_text().splitlines()
    moves = []
    for line in lines:
        row, col, keep, direction, distance = line.split()
        moves.append([int(row), int(col), int(keep), direction, int(distance)])
    return moves


def describe(rows):
    if not rows:
        return {"attempts": 0}
    successful = [r for r in rows if r["extracted"]]
    result = {
        "attempts": len(rows), "cases": len({r["case"] for r in rows}),
        "extracted": len(successful), "extraction_failed": len(rows) - len(successful),
        "split": sum(r["split"] for r in rows),
        "saved_mean": mean(r["saved"] for r in rows),
        "saved_le_1": sum(r["saved"] <= 1 for r in rows),
        "repaired": sum(r["repaired_jumps"] > 0 for r in successful),
        "net_removed_mean": mean(r["net_removed"] for r in successful) if successful else None,
        "net_removed_negative": sum(r["net_removed"] < 0 for r in successful),
    }
    for kind in KINDS:
        ok = [r for r in rows if r[kind + "_ok"]]
        improved = [r for r in ok if r[kind + "_delta"] < 0]
        result[kind] = {
            "completed": len(ok), "insertion_failed_after_extraction": sum(not r[kind + "_ok"] for r in successful),
            "raw_duplicates": sum(r[kind + "_raw_duplicate"] for r in ok),
            "duplicates": sum(r[kind + "_duplicate"] for r in ok),
            "equal_different": sum(r[kind + "_delta"] == 0 and not r[kind + "_duplicate"] for r in ok),
            "worse": sum(r[kind + "_delta"] > 0 for r in ok),
            "improved": len(improved), "improved_cases": len({r["case"] for r in improved}),
            "gain_sum_overlapping": sum(-r[kind + "_delta"] for r in improved),
            "raw_improved": sum(r[kind + "_raw_delta"] < 0 for r in ok),
            "total_us": sum(r[kind + "_us"] for r in rows),
            "mean_us": mean(r[kind + "_us"] for r in rows),
            "median_us": median(r[kind + "_us"] for r in rows),
        }
    return result


def diagnostics(rows):
    groups = defaultdict(list)
    for row in rows:
        groups[(row["case"], row["group"])].append(row)
    first, later, discarded, paired_first = [], [], [], []
    pairs, missed, ratios, late_only = [], [], [], []
    alternative_counts = {name: {kind: Counter() for kind in KINDS}
                          for name in ("second_occurrence", "earliest_later_split")}
    structure = Counter()
    for key, items in groups.items():
        items.sort(key=lambda r: r["time"])
        original = items[0]
        assert original["selected"] == 0
        first.append(original)
        discarded.extend(r for r in items if r["selected"] == 2)
        structure["groups"] += 1
        if len(items) < 2:
            continue
        structure["multi_time_groups"] += 1
        chosen = next(r for r in items if r["selected"] == 1)
        later.append(chosen); paired_first.append(original)
        structure["multiple_later_times"] += len(items) > 2
        structure["multiple_later_split_times"] += sum(r["split"] for r in items[1:]) > 1
        structure["later_split"] += chosen["split"]
        structure["later_saved_strictly_lower"] += chosen["saved"] < original["saved"]
        structure["later_cannot_outrank_first_with_10pct_noise"] += 1.1 * chosen["priority"] < 0.9 * original["priority"]
        structure["chosen_is_last_occurrence"] += chosen["time"] == items[-1]["time"]
        ratios.append(chosen["priority"] / original["priority"])
        for kind in KINDS:
            before = original[kind + "_delta"] if original[kind + "_ok"] else 100000
            after = chosen[kind + "_delta"] if chosen[kind + "_ok"] else 100000
            pairs.append({"kind": kind, "case": key[0], "early_only_success": before < 100000 and after == 100000,
                          "later_only_success": after < 100000 and before == 100000,
                          "later_only_improves": after < 0 <= before, "early_only_improves": before < 0 <= after,
                          "both_improve": before < 0 and after < 0, "later_better": after < before,
                          "first_better": before < after, "equal": before == after})
            if after < 0 <= before:
                late_only.append({"case": key[0], "group": key[1], "kind": kind, "size": chosen["size"],
                                  "colors": chosen["colors"], "split": chosen["split"],
                                  "saved": chosen["saved"], "delta": after})
            options_by_rule = {
                "second_occurrence": items[1],
                "earliest_later_split": next((r for r in items[1:] if r["split"]), items[1]),
            }
            original_best = min(0, before, after)
            for name, option in options_by_rule.items():
                option_delta = option[kind + "_delta"] if option[kind + "_ok"] else 100000
                alternative_best = min(0, before, option_delta)
                result = alternative_counts[name][kind]
                result["groups"] += 1
                result["better_groups"] += alternative_best < original_best
                result["worse_groups"] += alternative_best > original_best
                result["same_groups"] += alternative_best == original_best
                result["delta_sum_overlapping"] += alternative_best - original_best
            retained = min((r[kind + "_delta"] for r in items if r["selected"] != 2 and r[kind + "_ok"]), default=100000)
            options = [r for r in items if r["selected"] == 2 and r[kind + "_ok"] and r[kind + "_delta"] < min(retained, 0)]
            if options:
                best = min(options, key=lambda r: r[kind + "_delta"])
                missed.append({"case": key[0], "group": key[1], "kind": kind, "size": original["size"],
                               "first_time": original["time"], "chosen_time": chosen["time"], "chosen_split": chosen["split"],
                               "first_delta": before if before != 100000 else None,
                               "chosen_delta": after if after != 100000 else None,
                               "better_time": best["time"], "better_split": best["split"], "delta": best[kind + "_delta"]})
    ranking = {}
    for top in (8, 16, 32, 64):
        selected = [r for r in rows if 0 <= r["rank"] < top]
        ranking[str(top)] = {"total": len(selected), "later": sum(r["selected"] == 1 for r in selected)}
    time_overlap = Counter()
    for case_name in {r["case"] for r in rows}:
        primary_times = {r["time"] for r in first if r["case"] == case_name}
        extra = [r for r in later if r["case"] == case_name]
        time_overlap["later_cuts"] += len(extra)
        time_overlap["later_cuts_time_already_primary"] += sum(r["time"] in primary_times for r in extra)
        time_overlap["new_unique_times"] += len({r["time"] for r in extra} - primary_times)
        time_overlap["primary_unique_times"] += len(primary_times)
    pair_summary = {}
    for kind in KINDS:
        entries = [p for p in pairs if p["kind"] == kind]
        pair_summary[kind] = {k: sum(p[k] for p in entries) for k in entries[0] if k not in ("kind", "case")}
        pair_summary[kind]["pairs"] = len(entries)
    return {
        "structure": dict(structure), "priority_ratio_mean": mean(ratios), "priority_ratio_median": median(ratios),
        "ranking": ranking, "first_all": describe(first), "first_matched": describe(paired_first),
        "later": describe(later), "discarded": describe(discarded), "paired_outcomes": pair_summary,
        "later_split": describe([r for r in later if r["split"]]),
        "later_nonsplit": describe([r for r in later if not r["split"]]),
        "later_size_2_4": describe([r for r in later if r["size"] <= 4]),
        "later_size_5_8": describe([r for r in later if r["size"] >= 5]),
        "later_mono": describe([r for r in later if r["colors"] == 1]),
        "later_mixed": describe([r for r in later if r["colors"] > 1]),
        "missed_better": missed,
        "missed_cases": len({r["case"] for r in missed}),
        "missed_groups": len({(r["case"], r["group"]) for r in missed}),
        "late_only_improvements": late_only,
        "fixed_alternative_representatives": alternative_counts,
        "paired_anchor_time_overlap": dict(time_overlap),
    }


def runtime_summary():
    saved = json.loads((ROOT / "adhoc/v026_evaluation_summary.json").read_text())["generated_99"]
    counts, times = saved["counts_sum"], saved["mean_times_ms"]
    rows = {}
    for kind in (*KINDS, "paired"):
        if kind == "packet":
            attempts = counts["lns_block_attempts"] - counts["lns_flexible_attempts"]
            accepted = counts["lns_block_accepted"] - counts["lns_flexible_accepted"]
            improvements = counts["lns_block_improvements"] - counts["lns_flexible_improvements"]
        else:
            attempts = counts[f"lns_{kind}_attempts"]
            accepted = counts[f"lns_{kind}_accepted"]
            improvements = counts[f"lns_{kind}_improvements"]
        late = {key: counts[f"lns_late_start_{kind}_{key}"]
                for key in ("attempts", "completed", "accepted", "improvements", "saved")}
        late["total_ms"] = times[f"lns_late_start_{kind}"] * 99
        other = {"attempts": attempts - late["attempts"], "accepted": accepted - late["accepted"],
                 "improvements": improvements - late["improvements"], "saved": counts[f"lns_{kind}_saved"] - late["saved"]}
        rows[kind] = {"later": late, "other": other}
    return {"neighborhoods": rows,
            "later_generation_share": counts["lns_late_start_generated"] / (counts["lns_late_start_primary"] + counts["lns_late_start_generated"]),
            "later_direct_attempt_share": sum(rows[k]["later"]["attempts"] for k in KINDS) / counts["lns_block_attempts"]}


def main():
    hashes = json.loads((OUT / "input_hashes.json").read_text())
    for name, digest in hashes.items():
        assert hashlib.sha256((ROOT / name).read_bytes()).hexdigest() == digest, name
    rows = list(csv.DictReader((OUT / "candidates.csv").open()))
    strings = {"case", "group", "ids"}
    floats = {"priority", "packet_us", "flexible_us"}
    for row in rows:
        for key in set(row) - strings:
            row[key] = float(row[key]) if key in floats else int(row[key])
    per_case = defaultdict(list)
    for row in rows:
        per_case[row["case"]].append(row)
    assert len(per_case) == 100
    output_lengths = {}
    for case_name, case_rows in per_case.items():
        moves = saved_moves(case_name);output_lengths[case_name] = len(moves)
        groups, flights = replay(case_name, moves, collect=True)
        expected = {}
        for ids, times in groups.items():
            ordered = sorted(times.values(), key=lambda r: r["time"])
            kept_time = max(ordered[1:], key=lambda r: (r["split"], r["time"]))["time"] if len(ordered) > 1 else -1
            for index, item in enumerate(ordered):
                role = 0 if index == 0 else 1 if item["time"] == kept_time else 2
                expected[(set_hash(ids), item["time"])] = {
                    **item, "ids": " ".join(map(str, ids)), "selected": role,
                    "saved": sum(flight.issubset(ids) for flight in flights[item["time"]:]),
                }
        actual = {(r["group"], r["time"]): r for r in case_rows}
        assert len(actual) == len(case_rows) == len(expected)
        assert actual.keys() == expected.keys()
        for key, item in expected.items():
            for field, value in item.items():
                assert actual[key][field] == value, (case_name, key, field)
    verified_improvements = 0
    for line in (OUT / "improved.jsonl").read_text().splitlines():
        item = json.loads(line)
        replay(item["case"], item["moves"])
        assert len(item["moves"]) - output_lengths[item["case"]] == item["delta"] < 0
        verified_improvements += 1
    assert verified_improvements == sum(r[k + "_ok"] and r[k + "_delta"] < 0 for r in rows for k in KINDS)
    result = {"source_sha256": hashes['src/bin/v026_late_start_lns.cpp'], "verified_cases": 100,
              "verified_improvements": verified_improvements, "rows": len(rows),
              "neighborhood_calls": 2 * len(rows), "runtime_logs_99": runtime_summary(),
              "baseline_smoothing_deltas": dict(Counter(r["baseline_smoothing_delta"] for r in rows)),
              "generated_99": diagnostics([r for r in rows if r["case"] != "0000.txt"]),
              "case0000": diagnostics([r for r in rows if r["case"] == "0000.txt"])}
    (OUT / "summary.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    concise = {k: v for k, v in result.items() if k not in ("generated_99", "case0000")}
    normal = result["generated_99"]
    concise.update({k: normal[k] for k in ("structure", "ranking", "priority_ratio_mean", "priority_ratio_median", "paired_outcomes", "missed_cases", "missed_groups")})
    print(json.dumps(concise, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
