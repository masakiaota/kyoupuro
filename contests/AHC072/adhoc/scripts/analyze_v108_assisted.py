#!/usr/bin/env python3
"""保存済み評価を、NN・補完・LNSの各段階へ分けて集計する。"""
import json
from pathlib import Path
import statistics

from v089_data import ROOT, save, now

RUN = ROOT/'results/nn_rank/v108/20261004_assisted_weights_studio'


def analyze(root):
    results = {label: json.loads((root/label/'evaluation/validation/result.json').read_text())
               for label in ('base', 'small', 'wide', 'multistart')}
    baseline = {r['case']: r for r in results['base']['rows']}
    rows = {}
    for label, result in results.items():
        values = result['rows']; constructions = [r['construction'] for r in values]
        def mean(key):
            collected = [c[key] for c in constructions if key in c]
            return statistics.mean(collected) if collected else None
        current = dict(result['metrics'], lns_attempts=mean('lns_attempts'), mean_initial_T=mean('pre_lns'))
        current['mean_LNS_and_final_reduction'] = statistics.mean(r['construction']['pre_lns']-r['T'] for r in values)
        if label != 'multistart':
            count = len(values)
            current.update(
                nn_completed_inside_construction=sum(c['v312_nn_E'] == 0 for c in constructions),
                nn_incomplete_but_answer_completed=sum(r['construction']['v312_nn_E'] > 0 and r['E'] == 0 for r in values),
                nn_deadlines=sum(c['v312_nn_deadline'] for c in constructions),
                initial_from_tree=sum(c['v312_winner'] == -1 for c in constructions),
                initial_from_DP=sum(0 <= c['v312_winner'] < 5 for c in constructions),
                initial_from_NN=sum(c['v312_winner'] == 5 for c in constructions),
                DP_attempts=sum(c['v312_dp_attempts'] for c in constructions),
                DP_completed=sum(c['v312_dp_completed'] for c in constructions),
                DP_blocked=sum(c['v312_dp_blocked'] for c in constructions),
                DP_deadlines=sum(c['v312_dp_deadlines'] for c in constructions),
                NN_milliseconds=1000*mean('v312_nn_seconds'), DP_milliseconds=1000*mean('v312_dp_seconds'),
                construction_milliseconds=1000*mean('v312_construction_seconds'),
                longer_initial_but_shorter_final=sum(r['construction']['pre_lns'] > baseline[r['case']]['construction']['pre_lns']
                    and r['T'] < baseline[r['case']]['T'] for r in values),
                shorter_initial_but_longer_final=sum(r['construction']['pre_lns'] < baseline[r['case']]['construction']['pre_lns']
                    and r['T'] > baseline[r['case']]['T'] for r in values),
                mean_initial_difference=statistics.mean(r['construction']['pre_lns']-baseline[r['case']]['construction']['pre_lns'] for r in values),
                mean_final_difference=statistics.mean(r['T']-baseline[r['case']]['T'] for r in values))
            assert current['initial_from_tree']+current['initial_from_DP']+current['initial_from_NN'] == count
        rows[label] = current
    result = dict(conditions=rows, note='NN完走は構築22%の枠内の観測。単体solverの全時間での完走率ではない。', completed_at=now())
    save(root/'analysis.json', result)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    analyze(RUN)
