#!/usr/bin/env python3
"""Independent verification of v052 saved-plan diagnostics and its evaluation."""
import csv
import hashlib
import json
from pathlib import Path
import sys

from analyze_finite_windows import windows
from analyze_v047_coordination import inspect
from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v052_audit'
BIN = 'v052_finite_cooperation'
PARENT = 'v050_joint_towers'
PARENT_RUN = '20260929T104058+0900_v050_joint_towers_71ae82'


def verify_sources():
    audit = json.loads((AUDIT / 'static_verification.json').read_text())
    for name, key in ((BIN, 'solver_sha256'), (PARENT, 'parent_sha256'),
                      ('v043_cooperative_events', 'planner_parent_sha256')):
        assert hashlib.sha256((ROOT / f'src/bin/{name}.cpp').read_bytes()).hexdigest() == audit[key]
    for folder, key in ((ROOT / 'tools/in', 'input_sha256'), (ROOT / 'results/out' / PARENT, 'parent_output_sha256')):
        assert len(audit[key]) == 100
        for case, digest in audit[key].items():
            assert hashlib.sha256((folder / case).read_bytes()).hexdigest() == digest
    return audit


def common_prefix(a, b):
    n = 0
    for x, y in zip(a, b):
        if x != y:
            break
        n += 1
    return n


def verify_candidates():
    actual = {}
    for row in csv.DictReader((AUDIT / 'candidates.csv').open()):
        cells = tuple(tuple(map(int, v.split(':'))) for v in row['cells'].strip(';').split(';'))
        operations = tuple(map(int, row['operations'].strip(';').split(';')))
        key = row['case'], int(row['first']), int(row['last'])
        assert key not in actual
        actual[key] = (int(row['pieces']), int(row['towers']), bool(int(row['has_nest'])), int(row['priority']), cells, operations)
    expected = {}
    for inp in sorted((ROOT / 'tools/in').glob('*.txt')):
        report = inspect(inp, ROOT / 'results/out' / PARENT / inp.name)
        index = {p: i for i, p in enumerate(report['floor'])}
        for w in windows(report):
            sources = destinations = 0
            for p in w['cells']:
                a, b = report['states'][w['first']][index[p]], report['states'][w['last'] + 1][index[p]]
                matched = common_prefix(a, b)
                sources += len(a) > matched
                destinations += len(b) > matched
            key = inp.name, w['first'], w['last']
            expected[key] = (w['pieces'], w['towers'], w['has_nest'], len(w['operations']) - max(sources, destinations),
                             tuple(w['cells']), tuple(w['operations']))
    assert actual == expected
    return len(actual)


def diagnostic():
    audit = verify_sources()
    candidates_checked = verify_candidates()
    rows = list(csv.DictReader((AUDIT / 'diagnostic.csv').open()))
    assert len(rows) == len({r['case'] for r in rows}) == 100
    for r in rows:
        for key in r:
            if key != 'case':
                r[key] = float(r[key]) if key == 'ms' else int(r[key])
        assert verify_output(r['case'], AUDIT / 'compressed_v050' / r['case']) == r['after'] <= r['before']
        assert r['before'] == len((ROOT / 'results/out' / PARENT / r['case']).read_text().splitlines())
        assert r['before'] - r['after'] == r['saved']
        assert r['deadlines'] == r['global_deadlines'] == 0 and r['attempts'] <= 32
    witnesses = list(csv.DictReader((AUDIT / 'witnesses.csv').open()))
    assert len(witnesses) == 4
    for w in witnesses:
        for key in w:
            if key not in ('label', 'case'):
                w[key] = float(w[key]) if key == 'ms' else int(w[key])
        if w['label'] in ('paired_orientation', 'nest_pickup'):
            assert w['found'] and w['after'] <= w['reference']
        if w['found']:
            verify_output(w['case'] + '.txt', AUDIT / 'witnesses' / (w['label'] + '_auto.txt'))
    mechanism = json.loads((AUDIT / 'mechanism.json').read_text())
    saved = sum(r['saved'] for r in rows)
    assert mechanism['rng_consumption'] == 0 and mechanism['saved'] == saved and mechanism['cases'] == 100
    counts = {key: sum(r[key] for r in rows) for key in rows[0] if key not in ('case', 'before', 'after', 'ms')}
    result = {'solver_sha256': audit['solver_sha256'], 'verified_cases': 100, 'candidates_checked': candidates_checked,
              'mechanism': mechanism, 'witnesses': witnesses, 'saved': saved, 'counts': counts,
              'shortened_cases': sum(r['saved'] > 0 for r in rows),
              'mean_ms': sum(r['ms'] for r in rows) / 100, 'max_ms': max(r['ms'] for r in rows),
              'proceed_to_evaluation': saved > 0}
    (AUDIT / 'diagnostic_verification.json').write_text(json.dumps({**result, 'cases': rows}, indent=2) + '\n')
    print(json.dumps(result, indent=2))


def evaluation():
    audit = verify_sources()
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').read_text().splitlines()]
    current = [r for r in records if r['bin'] == BIN]
    parent = {r['case_name']: r for r in records if r['run_id'] == PARENT_RUN}
    assert len(current) == len(parent) == 100 and len({r['run_id'] for r in current}) == 1
    assert {r['case_name'] for r in current} == parent.keys()
    cases = []
    for record in sorted(current, key=lambda r: r['case_name']):
        assert record['status'] == 'ok' and record['local'] and record['input_dir'] == 'tools/in'
        path = ROOT / record['stdout_path']
        T = verify_output(record['case_name'], path)
        c, times, diagnostics = log_values(path.with_suffix('.txt.err'))
        assert T == record['score'] == c['T'] == c['final_ops'] == c['validated_moves']
        assert c['E'] == 0 and T <= 100000 and c['board_pool_free_at_end'] == c['board_pool_slots']
        assert c['pre_joint_ops'] - c['pre_finite_ops'] == c['joint_window_saved']
        assert c['pre_finite_ops'] - T == c['finite_saved']
        assert c['pre_pair_ops'] - c['pre_joint_ops'] == c['pair_transfer_saved']
        assert c['pre_lns_ops'] - c['pre_pair_ops'] == c['lns_saved']
        assert c['finite_attempts'] <= 32
        baseline = parent[record['case_name']]
        old, _, _ = log_values((ROOT / baseline['stdout_path']).with_suffix('.txt.err'))
        cases.append({**record, 'parent_T': baseline['score'], 'delta_T': T - baseline['score'],
                      'pre_lns_delta': c['pre_lns_ops'] - old['pre_lns_ops'],
                      'parent_attempts': old['lns_attempts'], 'counts': c, 'times_ms': times, 'diagnostics': diagnostics})
    total = sum(r['score'] for r in cases)
    baseline = sum(r['score'] for r in parent.values())
    counts = {key: sum(r['counts'][key] for r in cases) for key in cases[0]['counts'] if key.startswith(('joint_window_', 'finite_'))}
    errors = {key: sum(r['counts'][key] for r in cases) for key in ERRORS}
    attempts = sum(r['counts']['lns_attempts'] for r in cases)
    old_attempts = sum(r['parent_attempts'] for r in cases)
    result = {
        'run_id': current[0]['run_id'], 'parent_run_id': PARENT_RUN, 'solver_sha256': audit['solver_sha256'],
        'verified_cases': 100, 'total_T': total, 'parent_T': baseline, 'delta_T': total - baseline,
        'delta_percent': 100 * (total / baseline - 1),
        'wins': sum(r['delta_T'] < 0 for r in cases), 'draws': sum(r['delta_T'] == 0 for r in cases),
        'losses': sum(r['delta_T'] > 0 for r in cases), 'case0000_T': cases[0]['score'],
        'normal99_avg_T': (total - cases[0]['score']) / 99,
        'max_elapsed_ms': max(r['elapsed'] for r in cases), 'errors': errors,
        'diagnostic_count': sum(len(r['diagnostics']) for r in cases), 'counts': counts,
        'mean_finite_ms': sum(r['times_ms']['finite_window'] for r in cases) / 100,
        'max_finite_ms': max(r['times_ms']['finite_window'] for r in cases),
        'shortened_cases': sum(r['counts']['finite_saved'] > 0 for r in cases),
        'attempts': attempts, 'parent_attempts': old_attempts,
        'attempts_delta_percent': 100 * (attempts / old_attempts - 1),
        'pre_lns_delta_T': sum(r['pre_lns_delta'] for r in cases),
        'pre_lns_changed_cases': sum(r['pre_lns_delta'] != 0 for r in cases),
        'pre_finite_delta_T': sum(r['counts']['pre_finite_ops'] - r['parent_T'] for r in cases),
    }
    result['adopt'] = (not any(errors.values()) and result['diagnostic_count'] == 0
                       and result['max_elapsed_ms'] <= 2000 and result['case0000_T'] <= 43
                       and total < baseline and counts['finite_saved'] > 0)
    out = ROOT / 'results/analysis/v052'
    out.mkdir(parents=True, exist_ok=True)
    with (out / 'comparison.csv').open('w') as file:
        writer = csv.writer(file)
        writer.writerow(('case', 'parent_T', 'T', 'delta_T', 'pre_finite_T', 'finite_saved', 'nonempty_saved',
                         'nest_saved', 'attempts', 'expanded', 'deadlines', 'finite_ms'))
        for r in cases:
            c = r['counts']
            writer.writerow((r['case_name'], r['parent_T'], r['score'], r['delta_T'], c['pre_finite_ops'],
                             c['finite_saved'], c['finite_nonempty_saved'], c['finite_nest_saved'],
                             c['finite_attempts'], c['finite_expanded'], c['finite_deadlines'], r['times_ms']['finite_window']))
    (out / 'summary.json').write_text(json.dumps({**result, 'cases': cases}, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    {'diagnostic': diagnostic, 'evaluation': evaluation}[sys.argv[1]]()
