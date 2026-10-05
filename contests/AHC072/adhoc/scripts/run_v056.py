#!/usr/bin/env python3
"""Run the preregistered v056 diagnostics, without changing solver code."""
import csv
import fcntl
import json
import re
import statistics
import subprocess
import sys

from audit_v056 import ROOT, OUT, NAMES, digest
from check_v037_results import verify_output

ADDED = {'trial_board_conversions', 'trial_undo_attempts', 'trial_undo_moves',
         'insert_incoming_seed_calls', 'insert_incoming_candidates', 'insert_incoming_height_updates'}


def unchanged():
    for name, expected in json.loads((OUT / 'frozen.json').read_text()).items():
        assert digest(ROOT / name) == expected, 'Frozen file changed: ' + name


def fixed():
    destination = OUT / 'fixed_clock'
    destination.mkdir()
    cases = []
    for i, path in enumerate(sorted((ROOT / 'tools/in').glob('*.txt'))):
        records = []
        for label in NAMES:
            target = destination / label
            target.mkdir(exist_ok=True)
            result = subprocess.run([str(OUT / (label + '_fixed'))], input=path.read_bytes(), capture_output=True, timeout=60)
            (target / path.name).write_bytes(result.stdout)
            (target / (path.name + '.err')).write_bytes(result.stderr)
            assert result.returncode == 0, (label, path.name, result.stderr[-2000:])
            log = result.stderr.decode()
            counts = {k: int(v) for k, v in re.findall(r'\[summary.count\] ([^=]+)=(-?\d+)', log)}
            rng = int(re.search(r'\[fixed.rng\] (\d+)', log)[1])
            ticks = int(re.search(r'\[fixed.ticks\] (\d+)', log)[1])
            assert counts['E'] == 0 and counts['board_pool_free_at_end'] == 4
            verify_output(path.name, target / path.name)
            records.append((result.stdout, {k: v for k, v in counts.items() if k not in ADDED}, rng, ticks))
        if records[0] != records[1]:
            wrong = [k for k, a, b in zip(('output', 'counts', 'rng', 'ticks'), records[0], records[1]) if a != b]
            (OUT / 'fixed_failure.json').write_text(json.dumps({'case': path.name, 'differences': wrong,
                'parent_counts': records[0][1], 'child_counts': records[1][1]}, indent=2) + '\n')
            raise RuntimeError('Fixed-clock mismatch: ' + path.name + ' ' + str(wrong))
        cases.append({'case': path.name, 'T': counts['T'], 'ticks': ticks, 'common_keys': len(records[0][1])})
        if (i + 1) % 10 == 0:
            print('fixed clock matched', i + 1, '/100', flush=True)
    (OUT / 'fixed_clock_summary.json').write_text(json.dumps({'matched': 100, 'cases': cases}, indent=2) + '\n')


def check():
    path = OUT / 'mechanism.csv'
    assert not path.exists(), 'Mechanism check already executed'
    with path.open('w') as output, (OUT / 'mechanism.log').open('w') as log:
        process = subprocess.Popen([str(OUT / 'check'), str(ROOT)], stdout=output, stderr=subprocess.PIPE, text=True, cwd=ROOT)
        for line in process.stderr:
            print(line, end='', flush=True)
            log.write(line)
        assert process.wait() == 0, 'Mechanism check failed'
    rows = list(csv.DictReader(path.open()))
    assert len(rows) == 200 and all(r['equal'] == '1' for r in rows)
    selected = [r for r in rows if r['kind'] == '0']
    keys = ('flexible_calls', 'flexible_success', 'paired_calls', 'paired_success', 'mask_checks', 'trial_moves', 'trial_restores')
    summary = {'cases': 100, 'all_outputs_and_rng_equal': True,
               'tower_checks': int(re.search(r'tower_checks=(\d+)', (OUT / 'mechanism.log').read_text())[1]),
               **{k: sum(int(r[k]) for r in selected) for k in keys},
               **{k: sum(int(r[k]) for r in rows) for k in ('extract_calls', 'insert_calls', 'insert_success')}}
    (OUT / 'mechanism_summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary, indent=2), flush=True)


def bench():
    assert not (OUT / 'fixed_work_samples.csv').exists(), 'Timing plan already executed'
    with (OUT / 'bench.log').open('w') as log:
        process = subprocess.Popen([str(OUT / 'bench'), str(ROOT), str(OUT)], stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, text=True, cwd=ROOT)
        for line in process.stdout:
            print(line, end='', flush=True)
            log.write(line)
        assert process.wait() == 0, 'Fixed-work measurement failed'
    summarize()


def summarize():
    rows = list(csv.DictReader((OUT / 'fixed_work_samples.csv').open()))
    assert len(rows) == len({(r['case'], r['round'], r['variant']) for r in rows}) == 1200
    for case in {r['case'] for r in rows}:
        selected = [r for r in rows if r['case'] == case]
        assert len({(r['candidates'], r['completed'], r['checksum']) for r in selected}) == 1
    metrics = ('build_ns', 'reconstruct_ns', 'smooth_ns', 'compress_ns', 'plan_pair_ns', 'final_ns', 'total_ns', 'total_wall_ns')
    result = {'cases': 100, 'all_signatures_equal': True, 'LOCAL': False,
              'candidates': sum(int(r['candidates']) for r in rows if r['round'] == '0' and r['variant'] == 'v055'),
              'normal99': {}}
    for metric in metrics:
        rounds = []
        for index in range(1, 6):
            selected = [r for r in rows if r['case'] != '0000.txt' and r['round'] == str(index)]
            totals = {v: sum(int(r[metric]) for r in selected if r['variant'] == v) for v in ('v055', 'v056')}
            rounds.append({'round': index, **totals, 'change_percent': 100 * (totals['v056'] / totals['v055'] - 1)})
        result['normal99'][metric] = {'rounds': rounds, 'median_change_percent': statistics.median(r['change_percent'] for r in rounds),
                                    'median_ms': {v: statistics.median(r[v] for r in rounds) / 1e6 for v in ('v055', 'v056')}}
    result['speed_passed'] = result['normal99']['total_ns']['median_change_percent'] <= -1
    (OUT / 'fixed_work_summary.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'speed_passed': result['speed_passed'], 'candidates': result['candidates'],
                      'median_change_percent': {k: v['median_change_percent'] for k, v in result['normal99'].items()}}, indent=2), flush=True)


if __name__ == '__main__':
    unchanged()
    print('Acquiring evaluation lock', flush=True)
    with (ROOT / 'results/.eval.lock').open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        {'fixed': fixed, 'check': check, 'bench': bench, 'summarize': summarize}[sys.argv[1]]()
    unchanged()
    print('Frozen sources and inputs unchanged', flush=True)
