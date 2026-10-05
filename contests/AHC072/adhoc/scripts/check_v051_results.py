#!/usr/bin/env python3
"""Verify frozen-plan diagnostics and the single v051 evaluation independently."""
import csv
import hashlib
import json
from pathlib import Path
import sys

from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v051_audit'
BIN = 'v051_joint_polish'
PARENT = 'v050_joint_towers'
PARENT_RUN = '20260929T104058+0900_v050_joint_towers_71ae82'
MODES = ('route_pair', 'joint_only', 'combined')


def verify_sources():
    audit = json.loads((AUDIT / 'static_verification.json').read_text())
    for name, key in ((BIN, 'solver_sha256'), (PARENT, 'parent_sha256')):
        assert hashlib.sha256((ROOT / f'src/bin/{name}.cpp').read_bytes()).hexdigest() == audit[key]
    assert len(audit['input_sha256']) == len(audit['parent_output_sha256']) == 100
    for folder, key in ((ROOT / 'tools/in', 'input_sha256'),
                        (ROOT / 'results/out' / PARENT, 'parent_output_sha256')):
        for case, digest in audit[key].items():
            assert hashlib.sha256((folder / case).read_bytes()).hexdigest() == digest
    return audit


def diagnostic():
    audit = verify_sources()
    rows = list(csv.DictReader((AUDIT / 'diagnostic.csv').open()))
    assert len(rows) == len({(r['case'], r['mode']) for r in rows}) == 300
    for r in rows:
        for key in r:
            if key not in ('case', 'mode'):
                r[key] = float(r[key]) if key == 'ms' else int(r[key])
        assert r['mode'] in MODES
        actual = verify_output(r['case'], AUDIT / r['mode'] / r['case'])
        original = (ROOT / 'results/out' / PARENT / r['case']).read_bytes()
        assert r['before'] == len(original.splitlines())
        assert actual == r['after'] <= r['before']
        r['saved'] = r['before'] - actual
        assert r['saved'] == r['route_saved'] + r['pair_saved'] + r['joint_saved']
        assert r['deadlines'] == r['joint_deadlines'] == 0 and r['converged'] == 1
        r['byte_identical'] = original == (AUDIT / r['mode'] / r['case']).read_bytes()
    assert verify_output('0060.txt', AUDIT / 'witness_0060.txt') <= 159
    mechanism = json.loads((AUDIT / 'mechanism.json').read_text())
    assert mechanism['fixed_checks'] == 7 and mechanism['cases'] == 100 and mechanism['rng_consumption'] == 0
    summaries = {}
    for mode in MODES:
        cases = [r for r in rows if r['mode'] == mode]
        assert len(cases) == 100
        summaries[mode] = {
            'saved': sum(r['saved'] for r in cases),
            'shortened_cases': sum(r['saved'] > 0 for r in cases),
            'byte_identical_cases': sum(r['byte_identical'] for r in cases),
            'mean_ms': sum(r['ms'] for r in cases) / 100,
            'max_ms': max(r['ms'] for r in cases),
            'max_rounds': max(r['rounds'] for r in cases),
            'counts': {key: sum(r[key] for r in cases) for key in (
                'route_saved', 'pair_saved', 'joint_saved', 'rounds', 'route_calls', 'pair_calls', 'joint_calls',
                'candidates', 'searches', 'expanded', 'state_caps', 'replacements', 'independent',
                'changed_first_split', 'large_candidates', 'large_searches', 'large_replacements', 'large_saved')},
        }
        assert summaries[mode]['saved'] == mechanism[f'{mode}_saved']
    by_case = {(r['case'], r['mode']): r for r in rows}
    contrasts = {}
    for mode in MODES[:2]:
        deltas = [by_case[case, 'combined']['after'] - by_case[case, mode]['after']
                  for case in audit['input_sha256']]
        contrasts[mode] = {'delta_T': sum(deltas), 'wins': sum(d < 0 for d in deltas),
                           'draws': sum(d == 0 for d in deltas), 'losses': sum(d > 0 for d in deltas)}
    result = {'solver_sha256': audit['solver_sha256'], 'verified_outputs': 301,
              'mechanism': mechanism, 'modes': summaries, 'combined_vs_controls': contrasts,
              'proceed_to_evaluation': summaries['combined']['saved'] > 0}
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
        assert c['pre_joint_ops'] - c['pre_polish_ops'] == c['joint_window_saved']
        assert c['pre_polish_ops'] - T == c['joint_polish_saved']
        assert c['joint_polish_saved'] == sum(c['joint_polish_' + kind + '_saved'] for kind in ('route', 'pair', 'joint'))
        assert c['pre_pair_ops'] - c['pre_joint_ops'] == c['pair_transfer_saved'] - c['joint_polish_pair_saved']
        assert c['pre_lns_ops'] - c['pre_pair_ops'] == c['lns_saved']
        baseline = parent[record['case_name']]
        old, _, _ = log_values((ROOT / baseline['stdout_path']).with_suffix('.txt.err'))
        cases.append({**record, 'parent_T': baseline['score'], 'delta_T': T - baseline['score'],
                      'pre_lns_delta': c['pre_lns_ops'] - old['pre_lns_ops'],
                      'parent_attempts': old['lns_attempts'], 'counts': c, 'times_ms': times, 'diagnostics': diagnostics})
    total = sum(r['score'] for r in cases)
    baseline = sum(r['score'] for r in parent.values())
    counts = {key: sum(r['counts'][key] for r in cases)
              for key in cases[0]['counts'] if key.startswith(('joint_window_', 'joint_polish_'))}
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
        'mean_polish_ms': sum(r['times_ms']['joint_polish'] for r in cases) / 100,
        'max_polish_ms': max(r['times_ms']['joint_polish'] for r in cases),
        'shortened_cases': sum(r['counts']['joint_polish_saved'] > 0 for r in cases),
        'attempts': attempts, 'parent_attempts': old_attempts,
        'attempts_delta_percent': 100 * (attempts / old_attempts - 1),
        'pre_lns_delta_T': sum(r['pre_lns_delta'] for r in cases),
        'pre_lns_changed_cases': sum(r['pre_lns_delta'] != 0 for r in cases),
        'pre_polish_delta_T': sum(r['counts']['pre_polish_ops'] - r['parent_T'] for r in cases),
    }
    result['adopt'] = (not any(errors.values()) and result['diagnostic_count'] == 0
                       and result['max_elapsed_ms'] <= 2000 and result['case0000_T'] <= 43
                       and total < baseline and counts['joint_polish_saved'] > 0)
    out = ROOT / 'results/analysis/v051'
    out.mkdir(parents=True, exist_ok=True)
    with (out / 'comparison.csv').open('w') as file:
        writer = csv.writer(file)
        writer.writerow(('case', 'parent_T', 'T', 'delta_T', 'pre_polish_T', 'polish_saved', 'route_saved',
                         'pair_saved', 'joint_saved', 'rounds', 'polish_ms'))
        for r in cases:
            c = r['counts']
            writer.writerow((r['case_name'], r['parent_T'], r['score'], r['delta_T'], c['pre_polish_ops'],
                             c['joint_polish_saved'], c['joint_polish_route_saved'], c['joint_polish_pair_saved'],
                             c['joint_polish_joint_saved'], c['joint_polish_rounds'], r['times_ms']['joint_polish']))
    (out / 'summary.json').write_text(json.dumps({**result, 'cases': cases}, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    {'diagnostic': diagnostic, 'evaluation': evaluation}[sys.argv[1]]()
