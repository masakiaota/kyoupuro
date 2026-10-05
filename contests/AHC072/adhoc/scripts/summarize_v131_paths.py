#!/usr/bin/env python3
"""保存済みの途中経路二本の比較を集計する。solverを再実行しない。"""
import csv
import json
import statistics
import numpy as np
from run_v131_paths import ROOT, RUN, BASE, SOURCE, load
from v089_data import save, sha


def main():
    result = load(RUN / 'result.json')
    assert load(RUN / 'pipeline/exit.json')['exit_code'] == 0
    conditions = result['validation']
    base = {r['case']: r for r in conditions['base']['rows']}
    rows = conditions['paths']['rows']
    assert conditions['base']['source_sha256'] == sha(BASE)
    assert conditions['paths']['source_sha256'] == sha(SOURCE)
    differences = np.array([r['T'] - base[r['case']]['T'] for r in rows if r['E'] == base[r['case']]['E'] == 0])
    rng = np.random.default_rng(131006)
    means = rng.choice(differences, size=(4096, len(differences)), replace=True).mean(axis=1) if len(differences) else np.array([])
    diagnostics = {}
    for label, condition in conditions.items():
        current = condition['rows']
        diagnostics[label] = dict(
            mean_counts={k: statistics.mean(r['trace'].get(k, 0) for r in current)
                         for k in ('lns_attempts', 'lns_accepted', 'lns_uphill', 'lns_restarts')},
            nn_initial_hash_matches=sum(r['trace'].get('nn_initial_hash') == base[r['case']]['trace'].get('nn_initial_hash') for r in current),
            all_pool_returned=all(r['trace'].get('state_pool_free_at_end') == 4 for r in current),
            errors={k: sum(r['trace'].get(k, 0) for r in current) for k in (
                'lns_errors', 'construction_errors', 'lns_invalid_candidates', 'baseline_recovery', 'final_recovery')})
    summary = dict(assessment=result['assessment'], source_sha256=sha(SOURCE), parent_sha256=sha(BASE),
        validation={k: v['metrics'] for k, v in conditions.items()}, diagnostics=diagnostics,
        paired_bootstrap_95=np.quantile(means, [.025, .975]).tolist() if len(means) else None,
        bootstrap_note='Input-paired sampling only; runtime noise is not remeasured.',
        mechanism=load(RUN / 'mechanism/result.json')['passed'],
        official_source=str((RUN / 'result.json').relative_to(ROOT)), completed_at=result['completed_at'])
    if 'final' in result:
        f = result['final']
        summary['final'] = {k: v for k, v in f.items() if k != 'tools_in'}
        summary['final']['tools_in'] = f['tools_in']['metrics']
        summary['final']['below_190'] = f['tools_in']['metrics']['mean_T_completed'] < 190
    else:
        summary['retained'] = result['retained']
        previous = ROOT / 'results/nn_rank/v113/20261004_integrated_studio/final/result.json'
        stored = load(previous)
        assert stored['candidate']['source_sha256'] == stored['tools_in']['source_sha256'] == sha(BASE)
        summary['retained_final'] = dict(reused=True, source=str(previous.relative_to(ROOT)),
            tools_in=stored['tools_in']['metrics'], final_valid=stored['final_valid'])
    shared = ROOT / 'adhoc/v131'
    shared.mkdir(exist_ok=True)
    save(shared / 'result_summary.json', summary)
    save(RUN / 'summary.json', summary)
    with (shared / 'validation_paired.csv').open('w', newline='') as stream:
        writer = csv.writer(stream, lineterminator='\n')
        writer.writerow(['case', 'v113_T', 'v131_T', 'difference', 'E', 'v113_elapsed_ms', 'v131_elapsed_ms',
                         'path_pair_first_alternates', 'path_pair_completed', 'path_pair_saved'])
        for r in rows:
            old = base[r['case']]
            writer.writerow([r['case'], old['T'], r['T'], r['T']-old['T'], r['E'], old['elapsed_ms'], r['elapsed_ms']]
                            + [r['trace'].get(k, 0) for k in ('path_pair_first_alternates', 'path_pair_completed', 'path_pair_saved')])
    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
