#!/usr/bin/env python3
"""Verify the registered v057 diagnostics and one normal evaluation."""
import csv
import hashlib
import json
from pathlib import Path
import sys

from check_v028_two_orders import validate
from check_v037_results import ERRORS, log_values, verify_output
from check_v055_results import verify_pass

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v057_audit'
BIN = 'v057_search_reductions'
PARENT = 'v055_final_reductions'
PARENT_RUN = '20260929T134646+0900_v055_final_reductions_63082d'
KINDS = ('route', 'pair', 'joint', 'mono', 'strict', 'slack', 'finite')


def unchanged():
    report = json.loads((OUT / 'static_verification.json').read_text())
    digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    for name, key in ((BIN, 'solver_sha256'), (PARENT, 'parent_sha256')):
        assert digest(ROOT / f'src/bin/{name}.cpp') == report[key]
    for folder, key in ((ROOT / 'tools/in', 'input_sha256'),
                        (ROOT / 'results/out' / PARENT, 'parent_output_sha256')):
        for name, expected in report[key].items():
            assert digest(folder / name) == expected, str(folder / name)
    for name, expected in report['protected_sha256'].items():
        assert digest(ROOT / name) == expected, name
    return report


def diagnostic():
    unchanged()
    mechanism = json.loads((OUT / 'mechanism.json').read_text())
    rows = list(csv.DictReader((OUT / 'diagnostic.csv').open()))
    assert len(rows) == mechanism['candidates'] == 4382
    assert mechanism['rng_consumption'] == 0 and mechanism['witness_checks'] == 8
    assert sum(int(r['saved']) for r in rows) == mechanism['summed_candidate_saved'] > 0
    assert sum(int(r['probability_changed']) for r in rows) == mechanism['probability_changed']
    output_count = 0
    for line in (OUT / 'outputs.jsonl').open():
        row = json.loads(line)
        validate(row['case'], row['moves'])
        output_count += 1
    assert output_count == len(rows) + 100
    passes = 0
    for line in (OUT / 'passes.jsonl').open():
        verify_pass(json.loads(line))
        passes += 1
    witnesses = list(csv.DictReader((OUT / 'witnesses.csv').open()))
    assert len(witnesses) == 4 and all(int(w['found']) == 1 for w in witnesses)
    for w in witnesses:
        verify_output(w['case']+'.txt', OUT / 'witnesses' / (w['label']+'_auto.txt'))
    result = {**mechanism, 'independent_outputs': output_count,
              'independent_shortened_passes': passes, 'independent_witnesses': len(witnesses)}
    (OUT / 'diagnostic_verification.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


def evaluation():
    audit = unchanged()
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').open()]
    current = sorted((r for r in records if r['bin'] == BIN), key=lambda r: r['case_name'])
    parent = {r['case_name']: r for r in records if r['run_id'] == PARENT_RUN}
    assert len(current) == len(parent) == 100 and len({r['run_id'] for r in current}) == 1
    cases = []
    for r in current:
        assert r['status'] == 'ok' and r['local'] and r['input_dir'] == 'tools/in'
        path = ROOT / r['stdout_path']
        T = verify_output(r['case_name'], path)
        c, t, diagnostics = log_values(path.with_suffix('.txt.err'))
        assert T == r['score'] == c['T'] == c['final_ops'] == c['validated_moves']
        assert c['E'] == 0 and T <= 100000
        assert c['board_pool_free_at_end'] == c['board_pool_slots']
        assert c['pre_joint_ops']-c['pre_final_reductions_ops'] == c['joint_window_saved']
        assert c['pre_final_reductions_ops']-T == c['final_reductions_saved']
        assert c['final_reductions_saved'] == sum(c['final_reductions_'+k+'_saved'] for k in KINDS)
        assert c['pre_pair_ops']-c['pre_joint_ops'] == c['pair_transfer_saved']-c['final_reductions_pair_saved']-c['search_reductions_pair_saved']
        assert c['pre_lns_ops']-c['pre_pair_ops'] == c['lns_saved']
        assert c['lns_saved'] == c['lns_initial_shortcut_saved']+c['lns_initial_reorder_saved']+c['lns_best_saved']
        assert t['search_limit'] == 1544.0
        accepted_saved = sum(c['search_reductions_'+k+'_accepted_saved'] for k in KINDS)
        for k in KINDS:
            assert 0 <= c['search_reductions_'+k+'_accepted_saved'] <= c['search_reductions_'+k+'_saved']
        p = parent[r['case_name']]
        old, old_t, _ = log_values((ROOT / p['stdout_path']).with_suffix('.txt.err'))
        cases.append({**r, 'parent_T': p['score'], 'delta_T': T-p['score'],
                      'counts': c, 'times_ms': t, 'diagnostics': diagnostics,
                      'accepted_saved': accepted_saved,
                      'extra_ms': sum(t['search_reductions_'+k] for k in KINDS),
                      'parent_counts': old, 'parent_times_ms': old_t})
    total = sum(r['score'] for r in cases)
    baseline = sum(r['score'] for r in parent.values())
    keys = set.union(*(set(r['counts']) for r in cases))
    counts = {k: sum(r['counts'].get(k, 0) for r in cases) for k in sorted(keys)
              if k.startswith(('search_reductions_', 'final_reductions_'))}
    errors = {k: sum(r['counts'][k] for r in cases) for k in ERRORS}
    attempts = sum(r['counts']['lns_attempts'] for r in cases)
    parent_attempts = sum(r['parent_counts']['lns_attempts'] for r in cases)
    result = {'run_id': current[0]['run_id'], 'parent_run_id': PARENT_RUN,
              'solver_sha256': audit['solver_sha256'], 'verified_cases': 100,
              'total_T': total, 'parent_T': baseline, 'delta_T': total-baseline,
              'delta_percent': 100*(total/baseline-1), 'average_T': total/100,
              'wins': sum(r['delta_T'] < 0 for r in cases),
              'draws': sum(r['delta_T'] == 0 for r in cases),
              'losses': sum(r['delta_T'] > 0 for r in cases),
              'case0000_T': cases[0]['score'], 'max_elapsed_ms': max(r['elapsed'] for r in cases),
              'counts': counts, 'errors': errors, 'diagnostics': sum(len(r['diagnostics']) for r in cases),
              'accepted_candidate_saved': sum(r['accepted_saved'] for r in cases),
              'accepted_shortening_cases': sum(r['accepted_saved'] > 0 for r in cases),
              'mean_extra_ms': sum(r['extra_ms'] for r in cases)/100,
              'max_extra_ms': max(r['extra_ms'] for r in cases),
              'mean_ms_by_kind': {k: sum(r['times_ms']['search_reductions_'+k] for r in cases)/100 for k in KINDS},
              'attempts': attempts, 'parent_attempts': parent_attempts,
              'attempts_delta_percent': 100*(attempts/parent_attempts-1),
              'pre_lns_delta_T': sum(r['counts']['pre_lns_ops']-r['parent_counts']['pre_lns_ops'] for r in cases),
              'pre_final_delta_T': sum(r['counts']['pre_final_reductions_ops']-r['parent_counts']['pre_final_reductions_ops'] for r in cases)}
    result['adopt'] = (total < baseline and result['accepted_candidate_saved'] > 0
                       and not any(errors.values()) and result['diagnostics'] == 0
                       and result['case0000_T'] <= 43 and result['max_elapsed_ms'] <= 2000)
    dest = ROOT / 'results/analysis/v057'
    dest.mkdir(parents=True, exist_ok=True)
    with (dest / 'comparison.csv').open('w') as f:
        writer = csv.writer(f)
        writer.writerow(('case', 'parent_T', 'T', 'delta_T', 'accepted_saved', 'extra_ms',
                         *(k+'_accepted_saved' for k in KINDS)))
        for r in cases:
            writer.writerow((r['case_name'], r['parent_T'], r['score'], r['delta_T'],
                             r['accepted_saved'], r['extra_ms'],
                             *(r['counts']['search_reductions_'+k+'_accepted_saved'] for k in KINDS)))
    (dest / 'summary.json').write_text(json.dumps({**result, 'cases': cases}, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    {'diagnostic': diagnostic, 'evaluation': evaluation, 'unchanged': unchanged}[sys.argv[1]]()
