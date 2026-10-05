#!/usr/bin/env python3
"""Verify the registered v067 diagnostics and one normal evaluation."""
import csv
import hashlib
import json
from pathlib import Path
import sys

from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v067_audit'
BIN = 'v067_repair_joint_priority'
PARENT = 'v059_repair_priority'
PARENT_RUN = '20260929T235217+0900_v059_repair_priority_6879ca'
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
    result = json.loads((OUT / 'mechanism.json').read_text())
    assert result['calls'] == 24 and result['complete_cycles'] == 2
    assert result['skipped'] == 4 and result['rng_consumption'] == 0
    assert result['validated_outputs'] == 24 and result['pool_free'] == 4
    assert result['condition_checks'] == 4 and result['budget_checks'] == 3
    assert result['by_kind'] == {'joint': 18, 'strict': 2, 'slack': 2, 'finite': 2}
    (OUT / 'diagnostic_verification.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


def evaluation():
    audit = unchanged()
    assert (OUT / 'diagnostic_verification.json').exists()
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
            assert 0 <= c['search_reductions_'+k+'_best_created_with'] <= c['search_reductions_best_created']
            assert c['search_reductions_'+k+'_best_created_with'] <= c['search_reductions_'+k+'_accepted_saved']
        assert sum(c['search_reductions_'+k+'_best_created_with'] for k in KINDS) >= c['search_reductions_best_created']
        schedule = ('joint', 'joint', 'joint', 'strict', 'joint', 'joint', 'joint', 'slack', 'joint', 'joint', 'joint', 'finite')
        rounds, remaining = divmod(c['search_reductions_heavy_calls'], len(schedule))
        for k in ('joint', 'strict', 'slack', 'finite'):
            expected = rounds*schedule.count(k)+schedule[:remaining].count(k)
            assert c['search_reductions_'+k+'_calls'] == expected, (r['case_name'], k)
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
              if k.startswith(('search_reductions_', 'final_reductions_', 'lns_repair_priority_', 'lns_dependency_'))}
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
              'mean_elapsed_ms': sum(r['elapsed'] for r in cases)/100,
              'counts': counts, 'errors': errors, 'diagnostics': sum(len(r['diagnostics']) for r in cases),
              'accepted_candidate_saved': sum(r['accepted_saved'] for r in cases),
              'accepted_shortening_cases': sum(r['accepted_saved'] > 0 for r in cases),
              'mean_extra_ms': sum(r['extra_ms'] for r in cases)/100,
              'max_extra_ms': max(r['extra_ms'] for r in cases),
              'mean_ms_by_kind': {k: sum(r['times_ms']['search_reductions_'+k] for r in cases)/100 for k in KINDS},
              'attempts': attempts, 'parent_attempts': parent_attempts,
              'attempts_delta_percent': 100*(attempts/parent_attempts-1),
              'pre_pair_delta_T': sum(r['counts']['pre_pair_ops']-r['parent_counts']['pre_pair_ops'] for r in cases),
              'pre_lns_changed_cases': sum(r['counts']['pre_lns_ops']!=r['parent_counts']['pre_lns_ops'] for r in cases),
              'pre_lns_delta_T': sum(r['counts']['pre_lns_ops']-r['parent_counts']['pre_lns_ops'] for r in cases),
              'pre_final_delta_T': sum(r['counts']['pre_final_reductions_ops']-r['parent_counts']['pre_final_reductions_ops'] for r in cases)}
    assert all(counts['search_reductions_'+k+'_calls'] > 0 for k in ('joint', 'strict', 'slack', 'finite'))
    assert counts['lns_repair_priority_penalized'] > 0
    assert counts['lns_repair_priority_selected_broken_jumps'] > 0
    assert counts['lns_repair_priority_penalized_completed'] > 0
    result['schedule_verified_cases'] = len(cases)
    result['adopt'] = (total < baseline and counts['search_reductions_joint_accepted_saved'] > 0
                       and not any(errors.values()) and result['diagnostics'] == 0
                       and result['case0000_T'] <= 43 and result['max_elapsed_ms'] <= 2000)
    long_cases = {'0014.txt', '0015.txt', '0030.txt', '0034.txt', '0042.txt', '0047.txt'}
    result['long6_delta_T'] = sum(r['delta_T'] for r in cases if r['case_name'] in long_cases)
    result['long6_cases'] = sorted(long_cases)
    result['baseline_totals'] = {}
    for name in ('v057_search_reductions', 'v058_joint_priority'):
        other = [r for r in records if r['bin'] == name]
        assert len(other) == 100 and len({r['run_id'] for r in other}) == 1
        result['baseline_totals'][name] = {'total_T': sum(r['score'] for r in other),
                                         'delta_T': total-sum(r['score'] for r in other)}
    dest = ROOT / 'results/analysis/v067'
    dest.mkdir(parents=True, exist_ok=True)
    with (dest / 'comparison.csv').open('w') as f:
        writer = csv.writer(f)
        writer.writerow(('case', 'parent_T', 'T', 'delta_T', 'accepted_saved', 'extra_ms',
                         *(k+'_accepted_saved' for k in KINDS)))
        for r in cases:
            writer.writerow((r['case_name'], r['parent_T'], r['score'], r['delta_T'],
                             r['accepted_saved'], r['extra_ms'],
                             *(r['counts']['search_reductions_'+k+'_accepted_saved'] for k in KINDS)))
    mechanism = {}
    for prefix, field in (('parent', 'parent_counts'), ('current', 'counts')):
        times_field = 'parent_times_ms' if prefix == 'parent' else 'times_ms'
        mechanism[prefix] = {
            'by_kind': {k: {
                'calls': sum(r[field]['search_reductions_'+k+'_calls'] for r in cases),
                'saved': sum(r[field]['search_reductions_'+k+'_saved'] for r in cases),
                'accepted_saved': sum(r[field]['search_reductions_'+k+'_accepted_saved'] for r in cases),
                'best_created_with': (sum(r[field]['search_reductions_'+k+'_best_created_with'] for r in cases)
                                      if prefix == 'current' else None),
                'mean_ms': sum(r[times_field]['search_reductions_'+k] for r in cases)/100,
            } for k in KINDS},
            'best_created': sum(r[field]['search_reductions_best_created'] for r in cases),
            'mean_heavy_ms': sum(r[times_field]['search_reductions_heavy'] for r in cases)/100,
            'mean_extra_ms': sum(sum(r[times_field]['search_reductions_'+k] for k in KINDS) for r in cases)/100,
            'mean_make_candidates_ms': sum(r[times_field]['lns_candidate_build'] for r in cases)/100,
            'repair_priority': {k: sum(r[field].get(k, 0) for r in cases)
                                for k in keys if k.startswith('lns_repair_priority_')},
        }
    (dest / 'mechanism_summary.json').write_text(json.dumps(mechanism, indent=2)+'\n')
    (dest / 'summary.json').write_text(json.dumps({**result, 'cases': cases}, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    {'diagnostic': diagnostic, 'evaluation': evaluation, 'unchanged': unchanged}[sys.argv[1]]()
