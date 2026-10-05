#!/usr/bin/env python3
"""保存済み評価から修復方式の発動と入力別の差分を共有する。"""
import csv
import json
import re
import statistics

from build_v120_adaptive import ROOT, RUN
from v089_data import save, sha


def main():
    result = json.loads((RUN / 'result.json').read_text())
    assessment = result['assessment']
    current = assessment['validation']['adaptive']
    baseline = assessment['validation']['base']
    rows = current['rows']
    previous = {row['case']: row for row in baseline['rows']}
    assert len(rows) == len(previous) == 256
    assert all(row['complete'] and row['E'] == 0 for row in rows)
    assert all(row['trace']['state_pool_free_at_end'] == 4 for row in rows)
    errors = {key: sum(row['trace'].get(key, 0) for row in rows)
              for key in ['construction_errors', 'lns_errors', 'lns_invalid_candidates',
                          'baseline_recovery', 'final_recovery']}
    assert not any(errors.values())
    times = []
    for path in sorted((RUN / 'adaptive/evaluation/validation/outputs').glob('*.err')):
        times.append({key: float(value) for key, value in
                      re.findall(r'\[summary.time_ms\] (\w+)=([\d.]+)', path.read_text())})
    assert len(times) == 256
    methods = []
    total_trials = sum(row['trace']['allocation_observations'] for row in rows)
    for i, name in enumerate(['通常', '依存拡張', '束', '分解', '二塔', '並べ替え']):
        prefix = f'allocation_{i}'
        trials = sum(row['trace'][prefix + '_trials'] for row in rows)
        methods.append(dict(name=name, trials=trials, fraction=trials / total_trials,
                            saved_across_all_seeds=sum(row['trace'][prefix + '_saved'] for row in rows),
                            changed=sum(row['trace'][prefix + '_changes'] for row in rows),
                            mean_time_ms=statistics.mean(t[prefix] for t in times),
                            mean_case_min_probability=statistics.mean(row['trace'][prefix + '_min_ppm'] / 1e6 for row in rows),
                            mean_case_max_probability=statistics.mean(row['trace'][prefix + '_max_ppm'] / 1e6 for row in rows)))
    observed = dict(
        mean_lns_attempts=statistics.mean(row['trace']['lns_attempts'] for row in rows),
        baseline_mean_lns_attempts=statistics.mean(row['trace']['lns_attempts'] for row in baseline['rows']),
        all_methods_active_cases=sum(all(row['trace'][f'allocation_{i}_trials'] > 0 for i in range(6)) for row in rows),
        adaptive_active_cases=sum(row['trace']['allocation_draws'] > 0 for row in rows),
        initial_nn_same_hash=sum(row['trace']['nn_initial_hash'] == previous[row['case']]['trace']['nn_initial_hash'] for row in rows),
        pool_returned_cases=sum(row['trace']['state_pool_free_at_end'] == 4 for row in rows),
        errors=errors, methods=methods)
    save(RUN / 'observations.json', observed)
    final = result['final']
    source = ROOT / final['candidate']['source']
    assert sha(source) == final['candidate']['source_sha256']
    summary = dict(
        selected=assessment['selected'], retained=final['retained'], candidate=final['candidate'],
        validation={key: value['metrics'] for key, value in assessment['validation'].items()},
        differences=assessment['differences'], activation=assessment['activation'], observations=observed,
        mechanism={key: value for key, value in assessment['mechanism'].items() if key != 'reports'},
        retained_final={key: final[key]['metrics'] for key in ['test', 'tools_in']},
        reused_final=final['reused_control'], goal_190=final['tools_in']['metrics']['mean_T_completed'] < 190,
        source_hash_verified=True, official_source=str(RUN.relative_to(ROOT) / 'result.json'),
        completed_at=result['completed_at'])
    shared = ROOT / 'adhoc/v120'
    shared.mkdir(parents=True, exist_ok=True)
    save(shared / 'result_summary.json', summary)
    with (shared / 'validation_paired.csv').open('w', newline='') as stream:
        writer = csv.writer(stream, lineterminator='\n')
        writer.writerow(['case', 'v115_T', 'v120_T', 'difference', 'v120_E', 'elapsed_ms',
                         'v115_lns_attempts', 'v120_lns_attempts', 'allocation_observations', 'allocation_draws'])
        for row in rows:
            parent = previous[row['case']]
            writer.writerow([row['case'], parent['T'], row['T'], row['T'] - parent['T'], row['E'],
                             row['elapsed_ms'], parent['trace']['lns_attempts'], row['trace']['lns_attempts'],
                             row['trace']['allocation_observations'], row['trace']['allocation_draws']])
    print(json.dumps(dict(differences=summary['differences'], observations=observed,
                          goal_190=summary['goal_190']), ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
