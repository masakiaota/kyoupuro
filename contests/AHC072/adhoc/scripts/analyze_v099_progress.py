#!/usr/bin/env python3
"""保存済み学習ログだけから手数の推移と同じ入力集合の差を整理する。"""
import argparse
import json
from pathlib import Path
import numpy as np
from v089_data import ROOT, save, sha, now

RUN = ROOT / 'results/nn_rank/v099/20261003_complete_studio'


def analyze(destination, group_limit):
    destination.mkdir(parents=True, exist_ok=False)
    reports = {}; cohorts = {}; histories = {}
    for method in ('mc_ppo', 'group'):
        path = RUN / method / 'training/metrics.jsonl'
        rows = [json.loads(line) for line in path.read_text().splitlines()]
        if method == 'group' and group_limit: rows = [r for r in rows if r['iteration'] <= group_limit]
        histories[method] = rows; boards = []
        for row in rows:
            data = json.loads((RUN / method / 'training/episodes' / f"{row['iteration']:04d}.json").read_text())
            for item in data['inputs']:
                episodes = [r for r in data['rows'] if r['case'] == item['id']]
                assert len(episodes) == 4
                complete = [r['T'] for r in episodes if r['E'] == 0]
                boards.append(dict(iteration=row['iteration'], seed=item['seed'], sha256=item['sha256'],
                                   T=float(np.mean(complete)) if complete else None,
                                   successes=len(complete), **item['statistics']))
        cohorts[method] = boards; windows = {}
        for label, selected_rows in [('first5', rows[:5]), ('last5', rows[-5:])]:
            intervals = {r['iteration'] for r in selected_rows}
            selected = [b for b in boards if b['iteration'] in intervals]; good = [b for b in selected if b['successes'] == 4]
            weight = sum(r['successes'] for r in selected_rows)
            windows[label] = dict(iterations=sorted(intervals),
                mean_T_completed=sum(r['mean_T_completed'] * r['successes'] for r in selected_rows) / weight,
                input_means={k: float(np.mean([b[k] for b in selected])) for k in ('N', 'K', 'M', 'walls')},
                T_per_individual=sum(b['T'] for b in good)/sum(b['M'] for b in good),
                mean_entropy=float(np.mean([r['entropy'] for r in selected_rows])),
                successes=weight, episodes=sum(r['episodes'] for r in selected_rows))
        report = dict(iterations=len(rows), windows=windows, episodes=sum(r['episodes'] for r in rows),
                      successes=sum(r['successes'] for r in rows))
        validation = RUN / method / 'evaluation/validation/greedy/result.json'
        if validation.exists(): report['validation'] = json.loads(validation.read_text())['metrics']
        reports[method] = report
    shared = min(map(len, histories.values()))
    a = [r['mean_T_completed'] for r in histories['mc_ppo'][:shared]]
    b = [r['mean_T_completed'] for r in histories['group'][:shared]]
    baseline = {b['seed']: b for b in cohorts['mc_ppo']}
    pairs = [(baseline[b['seed']], b) for b in cohorts['group'] if b['seed'] in baseline]
    assert all(a['sha256'] == b['sha256'] for a, b in pairs)
    complete = [(a,b) for a,b in pairs if a['successes'] == b['successes'] == 4]
    result = dict(reports=reports, same_input_cohorts=shared, cohort_correlation=float(np.corrcoef(a,b)[0,1]),
                  paired_boards=len(pairs), all_eight_complete_boards=len(complete),
                  group_minus_ppo_on_paired_boards=float(np.mean([b['T']-a['T'] for a,b in complete])),
                  initial_validation_pending=not (RUN/'baseline/evaluation/validation/greedy/result.json').exists(),
                  generated_at=now(), solver_runs=0, gradient_updates=0)
    save(destination/'result.json', result); save(destination/'metrics_snapshot.json', histories)
    save(destination/'board_statistics.json', cohorts)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--group-limit',type=int,default=0)
    a=p.parse_args();analyze(a.out.resolve(),a.group_limit)
