#!/usr/bin/env python3
"""Inspect saved solutions only: no move generation and no solver invocation."""

from collections import Counter, defaultdict
import hashlib
import heapq
import json
from pathlib import Path

from replay_slime_output import replay


ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "adhoc/cooperative_plan_review_20260928"
BASE = "v039_exact_board_lns"
PAIRS = {
    "0005": "v026_late_start_lns",
    "0019": "v025_fast_math",
    "0060": "v022_dependency_lns",
    "0068": "v041_joint_window",
    "0097": "v041_joint_window",
}


def write_json(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


def inspect(case, bin_name):
    input_path = ROOT / "tools/in" / f"{case}.txt"
    output_path = ROOT / "results/out" / bin_name / f"{case}.txt"
    checked = replay(input_path, output_path)
    assert checked["metrics"]["E"] == 0
    lines = input_path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    towers, nests, tokens = {}, {}, {}
    numbers = Counter()
    for i, row in enumerate(lines[1:N + 1]):
        for j, ch in enumerate(row):
            if ch == "#":
                continue
            towers[i, j] = []
            if ch.islower():
                numbers[ch] += 1
                token = f"{ch}{numbers[ch]}"
                towers[i, j].append(token)
                tokens[token] = dict(color=ch, start=[i, j], moves=0, W=0,
                                     cost_units=0, support_uses=0, rides=[])
            elif ch.isupper():
                nests[i, j] = ch.lower()

    def snapshot():
        return {f"{p[0]},{p[1]}": list(ids) for p, ids in towers.items() if ids}

    snapshots, operations = [snapshot()], []
    supports = defaultdict(list)
    signatures = Counter()
    metrics = dict(checked["metrics"])
    metrics.update(N=N, K=K, floor_count=len(towers), wall_count=N*N-len(towers))
    metrics["flight_histogram"] = {}
    flight_histogram = Counter()
    for t, op in enumerate(checked["operations"]):
        p, q = tuple(op["source"]), tuple(op["destination"])
        _, _, k, _, length = op["action"].split()
        k, length = int(k), int(length)
        before_p, before_q = towers[p][:], towers[q][:]
        moving, support = before_p[k:], before_p[:k]
        for token in moving:
            info = tokens[token]
            info["moves"] += 1
            info["W"] += length
            info["cost_units"] += 840 // len(moving)
            info["rides"].append(t)
        if length > 1:
            for token in support:
                tokens[token]["support_uses"] += 1
            supports[p, tuple(support)].append(t)
        flight_histogram[len(moving), length] += 1
        signatures[(p, q, k, ''.join(x[0] for x in before_p),
                    ''.join(x[0] for x in before_q))] += 1
        towers[p] = support[:]
        towers[q].extend(reversed(moving))
        returned = []
        for cell in (p, q):
            while towers[cell] and towers[cell][-1][0] == nests.get(cell):
                token = towers[cell].pop()
                tokens[token]["returned"] = t
                returned.append(token)
            assert ''.join(x[0] for x in towers[cell]) == op["after"][str(cell)]
        assert ''.join(x[0] for x in moving) == op["moving_bottom_to_top"]
        operations.append(dict(turn=t, source=p, destination=q, k=k, length=length,
                               moving=moving, support=support, landing=before_q,
                               returned=returned, touched=before_p+before_q))
        snapshots.append(snapshot())
    assert not any(towers.values())
    assert sum(x["cost_units"] for x in tokens.values()) == 840*len(operations)
    assert sum(x["W"] for x in tokens.values()) == metrics["W"]
    metrics["flight_histogram"] = {f"{a}x{b}": n for (a, b), n in sorted(flight_histogram.items())}
    support_groups = [dict(cell=list(p), support=list(ids), turns=ts,
                           moving_total=sum(len(operations[t]["moving"]) for t in ts),
                           W=sum(len(operations[t]["moving"])*operations[t]["length"] for t in ts))
                      for (p, ids), ts in supports.items()]
    support_groups.sort(key=lambda a: (-len(a["turns"]), -a["W"]))
    return dict(case=case, bin=bin_name, metrics=metrics, tokens=tokens,
                operations=operations, snapshots=snapshots, support_groups=support_groups,
                hashes={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()
                        for p in (input_path, output_path)}), signatures


def components(report, boundaries):
    """Exactly the all-future contact closure used by v043, with size diagnostics."""
    answer = []
    for time in boundaries:
        parent = {x:x for x in report["tokens"]}
        def root(x):
            while parent[x] != x:
                parent[x] = parent[parent[x]]
                x = parent[x]
            return x
        for op in report["operations"][time:]:
            ids = op["touched"]
            r = root(ids[0])
            for x in ids[1:]:
                parent[root(x)] = r
        groups = defaultdict(lambda:dict(ids=[], cells=[], commands=0))
        for p, ids in report["snapshots"][time].items():
            group = groups[root(ids[0])]
            group["cells"].append(p)
            group["ids"].extend(ids)
        for op in report["operations"][time:]:
            groups[root(op["touched"][0])]["commands"] += 1
        groups = sorted(groups.values(), key=lambda x: -len(x["ids"]))
        eligible = [g for g in groups if 2 <= len(g["ids"]) <= 16 and len(g["cells"]) <= 6 and g["commands"] >= 2]
        answer.append(dict(time=time, remaining=sum(len(g["ids"]) for g in groups),
                           component_sizes=[len(g["ids"]) for g in groups],
                           eligible_ids=sum(len(g["ids"]) for g in eligible),
                           eligible_commands=sum(g["commands"] for g in eligible),
                           eligible_groups=len(eligible)))
    return answer


def transcript(report):
    rows=[]
    for op in report["operations"]:
        rows.append(f"{op['turn']:3} {str(op['source']):9} -> {str(op['destination']):9} "
                    f"k={op['k']} l={op['length']} move={','.join(op['moving']):38} "
                    f"support={','.join(op['support']):28} land={','.join(op['landing']):28} "
                    f"home={','.join(op['returned'])}")
    return '\n'.join(rows)+'\n'


def relaxed_distances(case):
    """A bound only: freely available supports, cost 840/(9-l) per slime."""
    lines = (ROOT/'tools/in'/f'{case}.txt').read_text().splitlines()
    N, K = map(int, lines[0].split())
    floor = {(i,j) for i,row in enumerate(lines[1:N+1])
             for j,ch in enumerate(row) if ch != '#'}
    nests = {ch.lower():(i,j) for i,row in enumerate(lines[1:N+1])
             for j,ch in enumerate(row) if ch.isupper()}
    assert len(nests) == K
    edges = defaultdict(list)
    for p in floor:
        for di,dj in ((1,0),(-1,0),(0,1),(0,-1)):
            for length in range(1,9):
                q = p[0]+di*length,p[1]+dj*length
                if q not in floor:
                    break
                edges[p].append((q,840//(9-length)))
    result = {}
    for color,nest in nests.items():
        distance,queue = {nest:0},[(0,nest)]
        while queue:
            value,p = heapq.heappop(queue)
            if value != distance[p]:
                continue
            for q,cost in edges[p]:
                if value+cost < distance.get(q,10**9):
                    distance[q] = value+cost
                    heapq.heappush(queue,(value+cost,q))
        assert len(distance) == len(floor)
        result[color] = {f'{i},{j}':value for (i,j),value in distance.items()}
    return result


def graph_components_check(report, expected):
    """Check union-find results using a separate adjacency graph and DFS."""
    for row in expected:
        time = row['time']
        adjacency = defaultdict(set)
        for op in report['operations'][time:]:
            first = op['touched'][0]
            for token in op['touched'][1:]:
                adjacency[first].add(token)
                adjacency[token].add(first)
        alive = {token:p for p,ids in report['snapshots'][time].items() for token in ids}
        unseen,groups = set(alive),[]
        while unseen:
            seen,queue = set(),[next(iter(unseen))]
            while queue:
                token = queue.pop()
                if token in seen:
                    continue
                seen.add(token)
                queue.extend(adjacency[token]-seen)
            assert seen <= set(alive)
            unseen -= seen
            commands = sum(op['touched'][0] in seen for op in report['operations'][time:])
            groups.append((len(seen),len({alive[x] for x in seen}),commands))
        eligible = [g for g in groups if 2 <= g[0] <= 16 and g[1] <= 6 and g[2] >= 2]
        assert sorted(g[0] for g in groups) == sorted(row['component_sizes'])
        assert sum(g[0] for g in eligible) == row['eligible_ids']
        assert sum(g[2] for g in eligible) == row['eligible_commands']
        assert len(eligible) == row['eligible_groups']
        assert sum(g[2] for g in groups) == len(report['operations'])-time


def main():
    DEST.mkdir(parents=True, exist_ok=True)
    pairs,checked_reports=[],{}
    for case, other in PAIRS.items():
        reports=[]
        signatures=[]
        for name in (BASE, other):
            report, sig = inspect(case, name)
            checked_reports[case,name] = report
            report["components"] = components(report, sorted({0, len(report['operations'])//4,
                len(report['operations'])//2, 3*len(report['operations'])//4}))
            reports.append(report)
            signatures.append(sig)
            write_json(DEST/f"{case}_{name[:4]}.json", report)
            (DEST/f"{case}_{name[:4]}_operations.txt").write_text(transcript(report))
        a,b=reports
        diff=[]
        for token, original in a['tokens'].items():
            reference=b['tokens'][token]
            diff.append(dict(id=token, start=original['start'],
                             cost_difference=(original['cost_units']-reference['cost_units'])/840,
                             old_moves=original['moves'], new_moves=reference['moves'],
                             old_W=original['W'], new_W=reference['W'],
                             old_support=original['support_uses'], new_support=reference['support_uses']))
        diff.sort(key=lambda x:-x['cost_difference'])
        shared=sum((signatures[0]&signatures[1]).values())
        pairs.append(dict(case=case, reference=other, baseline=a['metrics'], reference_metrics=b['metrics'],
                          gap=a['metrics']['T']-b['metrics']['T'], common_local_color_operations=shared,
                          token_cost_differences=diff, components=[r['components'] for r in reports],
                          support_groups=[r['support_groups'][:12] for r in reports]))
    write_json(DEST/'pair_summary.json',pairs)
    population=[]
    for file in sorted((ROOT/'tools/in').glob('*.txt')):
        if file.stem == '0000':
            continue
        report,_=inspect(file.stem,BASE)
        checked_reports[file.stem,BASE] = report
        T=report['metrics']['T']
        population.append(dict(case=file.stem,M=report['metrics']['M'],T=T,
                               components=components(report,[0,T//4,T//2,3*T//4])))
    write_json(DEST/'component_population.json',population)
    distances = {row['case']:relaxed_distances(row['case']) for row in population}
    bounds,checked_moves,checked_states = [],0,0
    for (case,name),report in checked_reports.items():
        values = [sum(distances[case][token[0]][p] for p,ids in state.items() for token in ids)
                  for state in report['snapshots']]
        occupied = [len(state) for state in report['snapshots']]
        # A real move transports at most 9-l slimes, so it decreases the
        # summed relaxed distance by at most one. Home returns have distance 0.
        for t in range(len(values)-1):
            assert values[t]-values[t+1] <= 840
            assert occupied[t]-occupied[t+1] <= 1
        for t,value in enumerate(values):
            assert max((value+839)//840,occupied[t]) <= len(report['operations'])-t
        checked_moves += len(values)-1
        checked_states += len(values)
        if name == BASE:
            bounds.append(dict(case=case,M=report['metrics']['M'],T=report['metrics']['T'],
                               fractional_transport=values[0]/840,
                               lower_bound=max((values[0]+839)//840,occupied[0])))
    bounds.sort(key=lambda row:row['case'])
    write_json(DEST/'transport_lower_bounds.json',bounds)
    write_json(DEST/'relaxed_distance_units.json',distances)
    for row in population:
        graph_components_check(checked_reports[row['case'],BASE],row['components'])
    sources = [Path(__file__),ROOT/'results/score_detail.csv',
               ROOT/'src/bin/v039_exact_board_lns.cpp',
               ROOT/'src/bin/v043_cooperative_events.cpp',ROOT/'src/bin/v012_pruned.cpp']
    hashes = {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
    for report in checked_reports.values():
        hashes.update(report['hashes'])
    write_json(DEST/'verification.json',dict(independent_graph_checks=len(population)*4,
               distinct_saved_solutions_replayed=len(checked_reports),
               lower_bound_checked_moves=checked_moves,lower_bound_checked_states=checked_states,
               solver_invocations=0,hashes=hashes))
    print('verification',len(checked_reports),'saved solutions;',checked_moves,'moves;',
          checked_states,'states;',len(population)*4,'component checks')
    for pair in pairs:
        print(pair['case'],pair['baseline']['T'],pair['reference_metrics']['T'],
              'M',pair['baseline']['M'],'common_operations',pair['common_local_color_operations'])
        print('baseline components',pair['components'][0])
        print('top cost differences',pair['token_cost_differences'][:8])
    for i in range(4):
        print('component_population',i,dict(
            eligible_cases=sum(r['components'][i]['eligible_groups']>0 for r in population),
            eligible_commands=sum(r['components'][i]['eligible_commands'] for r in population),
            remaining_commands=sum(r['T']-r['components'][i]['time'] for r in population),
            largest_over_16=sum(r['components'][i]['component_sizes'][0]>16 for r in population)))


if __name__ == '__main__':
    main()
