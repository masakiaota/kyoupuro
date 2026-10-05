#!/usr/bin/env python3
"""Verify the combined v031 experiment against its preregistered baseline."""
import csv
import hashlib
import json
from pathlib import Path

from summarize_v021 import latest_run, read_case
from summarize_v026 import group as late_start_group

ROOT = Path(__file__).resolve().parents[2]
BIN = 'v031_support_pair_fast_math'
PARENT = 'v026_late_start_lns'
LABEL = 'support_pair_fast_math'
PREFIX = 'lns_pair_support_'
BASELINES = {PARENT: 'late_start_lns', 'v028_two_order_lns': 'two_order_lns', 'v025_fast_math': 'fast_math'}


def group(cases, baselines):
    result = late_start_group(cases, baselines)
    counts = result['counts_sum']
    result['support_pair_mechanism_passed'] = all(counts.get(PREFIX+k, 0) > 0 for k in ('selected', 'completed', 'accepted'))
    result['support_pair_improved_cases'] = sum(c['counts'][PREFIX+'improvements'] > 0 for c in cases)
    result['support_pair_selected_cases'] = sum(c['counts'][PREFIX+'selected'] > 0 for c in cases)
    for case in cases:
        c, t = case['counts'], case['times_ms']
        z = {k.removeprefix(PREFIX): v for k, v in c.items() if k.startswith(PREFIX)}
        assert 0 <= z['priority_picks'] <= z['selected_before_cut'] <= z['available'] <= z['queries'] <= c['lns_paired_attempts']
        assert z['available'] <= z['related'] <= z['partners']
        assert 0 <= z['improvements'] <= z['accepted'] <= z['completed'] <= z['extracted'] <= z['attempted'] <= z['selected'] <= z['selected_before_cut']
        assert z['saved'] <= c['lns_paired_saved'] and z['saved'] >= z['improvements']
        assert z['accepted'] <= c['lns_paired_accepted']
        assert t[PREFIX+'mark'] <= t['lns_paired_search']+0.01
    return result


def main():
    audit = json.loads((ROOT/'adhoc/v031_audit/static_verification.json').read_text())
    diagnostic = json.loads((ROOT/'adhoc/v031_audit/diagnostic_check.json').read_text())
    for name, key in ((BIN, 'solver_sha256'), (PARENT, 'parent_sha256')):
        assert hashlib.sha256((ROOT/f'src/bin/{name}.cpp').read_bytes()).hexdigest() == audit[key]
    for name, digest in audit['input_sha256'].items():
        assert hashlib.sha256((ROOT/'tools/in'/name).read_bytes()).hexdigest() == digest
    records = [json.loads(line) for line in (ROOT/'results/eval_records.jsonl').read_text().splitlines()]
    current = latest_run(records, BIN, LABEL)
    assert len({r['run_id'] for r in records if r['bin'] == BIN}) == 1
    assert len(current) == len({r['case_name'] for r in current}) == 100
    assert all(r['local'] and r['input_dir'] == 'tools/in' for r in current)
    prior = {name: latest_run(records, name, label) for name, label in BASELINES.items()}
    assert prior[PARENT][0]['run_id'] == '20260927T172632+0900_v026_late_start_lns_973c99'
    baselines = {name: {r['case_name']: r for r in run} for name, run in prior.items()}
    for baseline in baselines.values():
        assert set(baseline) == {r['case_name'] for r in current}
        assert all(r['local'] and r['status'] == 'ok' for r in baseline.values())
    cases = [read_case(r) for r in current]
    parent_cases = {r['case_name']: read_case(r) for r in prior[PARENT]}
    result = {
        'bin': BIN, 'parents': ['v026', 'v025'], 'label': LABEL,
        'run_id': current[0]['run_id'], 'executed_at': current[0]['executed_at'],
        'solver_sha256': audit['solver_sha256'],
        'baseline_run_ids': {name: run[0]['run_id'] for name, run in prior.items()},
        'diagnostic_verification': diagnostic,
        'all_100': group(cases, baselines),
        'generated_99': group([c for c in cases if c['case_name'] != '0000.txt'], baselines),
        'case0000': group([c for c in cases if c['case_name'] == '0000.txt'], baselines),
        'parent_generated_99': late_start_group([c for name, c in parent_cases.items() if name != '0000.txt'], {}),
        'cases': cases,
    }
    all_cases, normal, parent_normal = (result[k] for k in ('all_100', 'generated_99', 'parent_generated_99'))
    result['adopt'] = (
        all_cases['verified_cases'] == all_cases['cases_under_2000_ms'] == 100
        and all_cases['error_count'] == all_cases['counts_sum'].get('lns_invalid_candidates', 0) == 0
        and diagnostic['mechanism_passed'] and audit['fast_math_attribute_lines'] > 0
        and normal['support_pair_mechanism_passed']
        and all_cases['comparisons'][PARENT]['delta_T'] < 0
        and result['case0000']['total_T'] <= 43
    )
    result['pre_lns_differences'] = {
        'cases': sum(c['counts']['pre_lns_ops'] != parent_cases[c['case_name']]['counts']['pre_lns_ops'] for c in cases),
        'total': sum(c['counts']['pre_lns_ops']-parent_cases[c['case_name']]['counts']['pre_lns_ops'] for c in cases),
    }
    n, p = normal['counts_sum'], parent_normal['counts_sum']
    result['relative_changes_percent'] = {
        'lns_attempts': 100*(n['lns_attempts']/p['lns_attempts']-1),
        'loops': 100*(normal['loops_including_dependency_skips']/parent_normal['loops_including_dependency_skips']-1),
        'paired_attempts': 100*(n['lns_paired_attempts']/p['lns_paired_attempts']-1),
        'paired_time': 100*(normal['mean_times_ms']['lns_paired_search']/parent_normal['mean_times_ms']['lns_paired_search']-1),
    }
    (ROOT/'adhoc/v031_evaluation_summary.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n')
    with (ROOT/'adhoc/v031_comparison.csv').open('w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(('case', 'v026_T', 'v028_T', 'v031_T', 'delta_vs_v026', 'pre_lns_delta', 'attempts',
                         'paired_attempts', 'related_selected', 'related_completed', 'related_accepted', 'related_best_saved', 'mark_ms', 'elapsed_ms'))
        for case in sorted(cases, key=lambda c: c['case_name']):
            parent, c = parent_cases[case['case_name']], case['counts']
            writer.writerow((case['case_name'], parent['score'], baselines['v028_two_order_lns'][case['case_name']]['score'], case['score'], case['score']-parent['score'],
                             c['pre_lns_ops']-parent['counts']['pre_lns_ops'], c['lns_attempts'], c['lns_paired_attempts'],
                             c[PREFIX+'selected'], c[PREFIX+'completed'], c[PREFIX+'accepted'], c[PREFIX+'saved'],
                             case['times_ms'][PREFIX+'mark'], case['elapsed']))
    print(json.dumps({
        'run_id': result['run_id'], 'adopt': result['adopt'],
        'mean_T': all_cases['mean_T'], 'total_T': all_cases['total_T'],
        'comparisons': all_cases['comparisons'], 'verified_cases': all_cases['verified_cases'],
        'max_elapsed_ms': all_cases['max_elapsed_ms'], 'error_count': all_cases['error_count'],
        'mechanism_passed': normal['support_pair_mechanism_passed'],
        'support_pair_improved_cases': normal['support_pair_improved_cases'],
        'pre_lns_differences': result['pre_lns_differences'],
        'relative_changes_percent': result['relative_changes_percent'],
        'support_pair_counts': {k: v for k, v in n.items() if k.startswith(PREFIX)},
        'support_pair_mark_ms_mean': normal['mean_times_ms'][PREFIX+'mark'],
    }, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
