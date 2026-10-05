#!/usr/bin/env python3
"""保存手順の個体追跡と、同じ全盤面を結ぶ短縮区間の抽出。探索は行わない。"""

from collections import Counter, defaultdict
import csv
from difflib import SequenceMatcher
import json
from pathlib import Path
import sys

from replay_slime_output import replay

ROOT = Path(__file__).resolve().parents[2]
DIRECTIONS = {"U": (-1, 0), "D": (1, 0), "L": (0, -1), "R": (0, 1)}


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def write_csv(path, rows):
    with path.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def inspect(inp, output):
    checked = replay(inp, output)
    assert checked["metrics"]["E"] == 0
    rows = inp.read_text().splitlines()
    N, K = map(int, rows[0].split())
    board = rows[1:N+1]
    towers, nests, tokens, numbers = {}, {}, {}, Counter()
    for i, row in enumerate(board):
        for j, c in enumerate(row):
            if c == "#":
                continue
            towers[i, j] = []
            if c.isupper():
                nests[i, j] = c.lower()
            if c.islower():
                numbers[c] += 1
                token = f"{c}{numbers[c]}"
                towers[i, j] = [token]
                tokens[token] = dict(start=(i, j), rides=[], W=0, cost_units=0, support_uses=0)
    floor = sorted(towers)

    def state():
        return tuple(''.join(x[0] for x in towers[p]) for p in floor)

    states, operations = [state()], []
    metrics = Counter(checked["metrics"])
    hist, support_groups = Counter(), defaultdict(list)
    for t, item in enumerate(checked["operations"]):
        p, q = tuple(item["source"]), tuple(item["destination"])
        _, _, k, d, length = item["action"].split()
        k, length = int(k), int(length)
        before_p, before_q = towers[p][:], towers[q][:]
        moving, support = before_p[k:], before_p[:k]
        for token in moving:
            tokens[token]["rides"].append(t)
            tokens[token]["W"] += length
            tokens[token]["cost_units"] += 840 // len(moving)
        if length > 1:
            support_groups[p, tuple(support)].append(t)
            for token in support:
                tokens[token]["support_uses"] += 1
        towers[p] = support[:]
        towers[q].extend(reversed(moving))
        returned_p, returned_q = [], []
        for cell, returned in ((p, returned_p), (q, returned_q)):
            while towers[cell] and towers[cell][-1][0] == nests.get(cell):
                token = towers[cell].pop()
                returned.append(token)
                tokens[token]["returned_at"] = t
        metrics["return_operations"] += bool(returned_p or returned_q)
        metrics["whole_tower_moves"] += k == 0
        metrics["empty_landing"] += not before_q
        metrics["single_empty_landing"] += not before_q and len(moving) == 1
        metrics["source_return_operations"] += bool(returned_p)
        metrics["destination_return_operations"] += bool(returned_q)
        metrics["nest_touches"] += p in nests or q in nests
        metrics["mixed_support_jumps"] += length > 1 and len({x[0] for x in before_p}) > 1
        hist[len(moving), length] += 1
        operations.append(dict(turn=t, action=item["action"], source=p, destination=q, k=k, length=length,
                               moving=moving, support=support, landing=before_q,
                               before_source=before_p, before_destination=before_q,
                               after_source=towers[p][:], after_destination=towers[q][:],
                               returned_source=returned_p, returned_destination=returned_q))
        states.append(state())
    assert not any(states[-1])
    assert sum(v["cost_units"] for v in tokens.values()) == 840 * len(operations)
    groups = [dict(cell=p, support=ids, turns=times) for (p, ids), times in support_groups.items()]
    groups.sort(key=lambda v: -len(v["turns"]))
    metrics["reused_support_groups"] = sum(len(v["turns"]) > 1 for v in groups)
    metrics["reused_support_jumps"] = sum(len(v["turns"]) for v in groups if len(v["turns"]) > 1)
    return dict(N=N, K=K, board=board, floor=floor, metrics=dict(metrics), states=states, operations=operations,
                tokens=tokens, support_groups=groups,
                histogram={f"{a}x{l}": n for (a, l), n in sorted(hist.items())})


def transcript(report, start=0, end=None):
    result = []
    for op in report["operations"][start:end]:
        result.append(f"{op['turn']:3}: {op['action']:13} {str(op['source']):9}->{str(op['destination']):9} "
                      f"move={','.join(op['moving']):36} lower={','.join(op['support']):24} "
                      f"land={','.join(op['landing']):24} "
                      f"home={','.join(op['returned_source']+op['returned_destination'])}")
    return '\n'.join(result) + '\n'


def closed_segments(before, after):
    """Common complete color boards certify that a segment can be compared alone."""
    pairs = SequenceMatcher(a=before["states"], b=after["states"], autojunk=False).get_matching_blocks()
    result = []
    previous = None
    for block in pairs:
        if not block.size:
            continue
        if previous:
            a0, b0 = previous
            a1, b1 = block.a, block.b
            if a1-a0 > b1-b0:
                ops = before["operations"][a0:a1] + after["operations"][b0:b1]
                ids = {x for op in ops for x in op["before_source"]+op["before_destination"]}
                result.append(dict(before_start=a0, before_end=a1, after_start=b0, after_end=b1,
                                   saved=(a1-a0)-(b1-b0), before_moves=a1-a0, after_moves=b1-b0,
                                   tokens=len(ids), cells=len({tuple(op[k]) for op in ops for k in ("source", "destination")})))
        previous = block.a+block.size-1, block.b+block.size-1
    return result


def scan_motifs(out):
    """Count structural opportunities in saved plans; do not construct replacements."""
    rows = []
    for path in sorted(out.glob("????_*.json")):
        if path.stem.split('_')[-1] not in ("initial", "best"):
            continue
        report = json.loads(path.read_text())
        ops = report["operations"]
        pickup, orientation_legs = [], defaultdict(list)
        for a, first in enumerate(ops):
            p, q = first["source"], first["destination"]
            # Sending a parcel out to collect another and bringing the union
            # back is avoidable when both fit above the nest residents at once.
            if first["k"] and len(first["returned_source"]) == first["k"] and not first["returned_destination"]:
                later = [op for op in ops[a+1:] if p in (op["source"], op["destination"]) or q in (op["source"], op["destination"])]
                if len(later) >= 2:
                    second, third = later[:2]
                    landing = first["before_destination"]
                    condition = (second["source"] == q and second["destination"] == p and second["k"] == 0
                                 and third["source"] == p and third["destination"] not in (p, q) and third["k"] == 0
                                 and not second["returned_source"] and not second["returned_destination"]
                                 and landing and landing[0][0] != first["returned_source"][0][0]
                                 and len(first["before_source"])+len(landing) <= 8)
                    cells = (p, q, third["destination"])
                    chosen = {first["turn"], second["turn"], third["turn"]}
                    independent = all(op["turn"] in chosen or not any(cell in (op["source"], op["destination"]) for cell in cells)
                                      for op in ops[a:third["turn"]+1])
                    if condition and independent:
                        pickup.append(dict(turns=sorted(chosen), cells=cells))
            # A two-step straight transport with an available one-slime launch
            # support. A single shortcut reverses a non-palindromic parcel, so
            # these are opportunities for joint orientation planning, not proven savings.
            if a+1 < len(ops):
                second = ops[a+1]
                word = ''.join(x[0] for x in first["moving"])
                if (first["length"] == second["length"] == 1 and first["k"] >= 1
                        and first["action"].split()[3] == second["action"].split()[3]
                        and second["source"] == q and first["moving"] == second["moving"][::-1]
                        and not first["returned_source"] and not first["returned_destination"]
                        and not second["returned_source"] and not second["returned_destination"]
                        and word != word[::-1]):
                    orientation_legs[tuple(sorted(first["moving"]))].append(a)
        pairs = [dict(tokens=ids, starts=ts) for ids, ts in orientation_legs.items() if len(ts) >= 2]
        rows.append(dict(case=path.stem[:4], mode=path.stem.split('_')[-1],
                         nest_pickup_candidates=len(pickup), nest_pickup_details=pickup,
                         orientation_blocked_legs=sum(map(len, orientation_legs.values())),
                         repeated_parcel_groups=len(pairs), repeated_parcel_details=pairs))
    write_json(out / "motif_counts.json", rows)
    return rows


def main():
    run = Path(sys.argv[1]).resolve()
    out = run / "analysis/coordination"
    out.mkdir(exist_ok=True)
    manifest = json.loads((run / "manifest.json").read_text())
    metric_rows, differences, segments, events_out = [], [], [], []
    for case in manifest["cases"]:
        case_id = case["case"]
        inp = run / case["input"]
        reports = {}
        for mode, path in (("initial", run / case["seeds"][0]["path"]), ("best", run / "cases" / case_id / "best.txt")):
            report = inspect(inp, path)
            reports[mode] = report
            write_json(out / f"{case_id}_{mode}.json", report)
            (out / f"{case_id}_{mode}_operations.txt").write_text(transcript(report))
            metric_rows.append(dict(case=case_id, mode=mode, **report["metrics"]))
        a, b = reports["initial"], reports["best"]
        diffs = []
        for token, old in a["tokens"].items():
            new = b["tokens"][token]
            diffs.append(dict(token=token, start=old["start"], saved_cost=(old["cost_units"]-new["cost_units"])/840,
                              old_rides=len(old["rides"]), new_rides=len(new["rides"]),
                              old_W=old["W"], new_W=new["W"], old_support=old["support_uses"], new_support=new["support_uses"]))
        differences.append(dict(case=case_id, tokens=sorted(diffs, key=lambda v:-v["saved_cost"])))
        # One continuous run per case gives a consistent chain of recorded best solutions.
        folder = run / "cases" / case_id / "continuous/round_0000/search"
        for event in map(json.loads, (folder / "events.jsonl").read_text().splitlines()):
            if event["type"] != "improvement" or not event["before_plan"]:
                continue
            before = inspect(inp, folder / event["before_plan"])
            after = inspect(inp, folder / event["plan"])
            row = {k: event[k] for k in ("elapsed_sec", "reason", "iteration", "T", "previous_best_T", "before_T", "saved")}
            row.update(case=case_id, plan=str((folder / event["plan"]).relative_to(run)),
                       before_plan=str((folder / event["before_plan"]).relative_to(run)))
            events_out.append(row)
            for segment in closed_segments(before, after):
                segment.update(row)
                # Preserve the local saving separately from the best-record increment.
                segment["segment_saved"] = segment["before_moves"]-segment["after_moves"]
                segments.append(segment)
        print(f"{case_id}: 個体追跡と改善区間の照合が完了", flush=True)
    fields = list(dict.fromkeys(k for row in metric_rows for k in row))
    metric_rows = [{k: row.get(k, 0) for k in fields} for row in metric_rows]
    write_csv(out / "metrics.csv", metric_rows)
    write_json(out / "token_differences.json", differences)
    write_json(out / "closed_segments.json", sorted(segments, key=lambda s: (-s["segment_saved"], s["before_moves"])))
    write_csv(out / "continuous_events.csv", events_out)
    scan_motifs(out)
    print(f"{len(segments)}個の盤面境界が一致する短縮区間を保存: {out}")


if __name__ == "__main__":
    main()
