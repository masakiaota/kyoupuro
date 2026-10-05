#!/usr/bin/env python3
"""Analyze saved scores for input-based selection; never run a solver."""
from collections import defaultdict
import csv
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'results/analysis/v058_v059_selection/20260930'
BINS = ('v058_joint_priority', 'v059_repair_priority', 'v067_repair_joint_priority')


def relative(minimum, score):
    # Match Math.round(1e9 * MIN / YOUR) for positive integer scores.
    return (2 * 10**9 * minimum + score) // (2 * score)


def features(path):
    tokens = path.read_text().split()
    N, K = map(int, tokens[:2])
    grid = tokens[2:]
    assert len(grid) == N and all(len(row) == N for row in grid)
    M = sum('a' <= c <= 'z' for row in grid for c in row)
    walls = sum(row.count('#') for row in grid)
    return {'N': N, 'K': K, 'M': M, 'wall_fraction': walls / (N * N),
            'slime_density': M / (N * N - walls - K)}


def summarize(rows):
    n = len(rows)
    names = ('v058', 'v059', 'v067', 'select_M_lt_80', 'best_of_two_after_results')
    return {
        'cases': n,
        'totals': {name: sum(r[name] for r in rows) for name in names},
        'relative_avg': {name: sum(relative(r['reference_min'], r[name]) for r in rows) / n
                         for name in names},
        'v058_minus_v059': sum(r['v058'] - r['v059'] for r in rows),
        'v058_wins_draws_losses': [sum(r['v058'] < r['v059'] for r in rows),
                                  sum(r['v058'] == r['v059'] for r in rows),
                                  sum(r['v058'] > r['v059'] for r in rows)],
        'selection_minus_v059': sum(r['select_M_lt_80'] - r['v059'] for r in rows),
    }


def main():
    by_run = defaultdict(list)
    for line in (ROOT / 'results/eval_records.jsonl').open():
        r = json.loads(line)
        if r['input_dir'] in ('tools/in', 'tools/validation1'):
            by_run[r['run_id']].append(r)
    # The viewer excludes a whole run if any case failed.
    valid = {rid: rs for rid, rs in by_run.items()
             if rs and all(r['status'] == 'ok' for r in rs)}
    OUT.mkdir(parents=True, exist_ok=True)
    result = {'solver_executions': 0, 'threshold': 80,
              'threshold_status': 'exploratory: selected after inspecting validation1 groups',
              'relative_reference': 'minimum per case across valid runs in the same evaluation set',
              'selection_status': 'saved-result calculation, not a compiled selector evaluation',
              'datasets': {}}
    for dataset, expected in (('tools/in', 100), ('tools/validation1', 500)):
        references = {rid: rs for rid, rs in valid.items() if rs[0]['input_dir'] == dataset}
        selected = {}
        for name in BINS:
            matches = [(rid, rs) for rid, rs in references.items() if rs[0]['bin'] == name]
            assert len(matches) == 1, (dataset, name)
            rid, rs = matches[0]
            assert len(rs) == expected and len({r['case_name'] for r in rs}) == expected
            selected[name] = (rid, {r['case_name']: r for r in rs})
        names = sorted(selected[BINS[0]][1])
        assert all(set(rs) == set(names) for _, rs in selected.values())
        minima = {}
        for rs in references.values():
            for r in rs:
                key = r['case_name']
                minima[key] = min(minima.get(key, r['score']), r['score'])
        rows, hashes = [], {}
        for case in names:
            path = ROOT / dataset / case
            hashes[case] = hashlib.sha256(path.read_bytes()).hexdigest()
            scores = {f'v{name[1:4]}': selected[name][1][case]['score'] for name in BINS}
            row = {'case': case, **features(path), **scores, 'reference_min': minima[case]}
            row['select_M_lt_80'] = row['v058'] if row['M'] < 80 else row['v059']
            row['best_of_two_after_results'] = min(row['v058'], row['v059'])
            row['v058_minus_v059'] = row['v058'] - row['v059']
            row['selection_minus_v059'] = row['select_M_lt_80'] - row['v059']
            rows.append(row)
        summary = summarize(rows)
        summary['runs'] = {name: rid for name, (rid, _) in selected.items()}
        summary['reference_runs'] = list(references)
        summary['M_groups'] = {
            'below_80': summarize([r for r in rows if r['M'] < 80]),
            'at_least_80': summarize([r for r in rows if r['M'] >= 80]),
        }
        summary['consecutive_100_case_blocks'] = [
            {'first_case': rows[start]['case'], **summarize(rows[start:start + 100])}
            for start in range(0, len(rows), 100)
        ]
        summary['current_input_sha256'] = hashes
        result['datasets'][dataset] = summary
        with (OUT / (Path(dataset).name + '_cases.csv')).open('w', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
    result['current_source_sha256'] = {
        name: hashlib.sha256((ROOT / f'src/bin/{name}.cpp').read_bytes()).hexdigest()
        for name in BINS
    }
    (OUT / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
    for dataset, summary in result['datasets'].items():
        print(dataset, json.dumps({k: v for k, v in summary.items()
                                  if k in ('cases', 'totals', 'relative_avg', 'selection_minus_v059')},
                                 ensure_ascii=False))


if __name__ == '__main__':
    main()
