#!/usr/bin/env python3
"""保存した教師・選別・完成版の結果を要約する。計算を再実行しない。"""
import csv
import shutil

import numpy as np
import torch

from build_v127_allocation import ROOT, RUN, BASE, BASE_SHA
from train_v127_allocation import load, model_new
from v089_data import save, sha


def compact_evaluation(result):
    return dict(metrics=result['metrics'], all_legal=result.get('all_legal', True),
                source_sha256=result.get('source_sha256'))


def main():
    result = load(RUN / 'result.json')
    assert load(RUN / 'pipeline/exit.json')['exit_code'] == 0
    assert sha(BASE) == BASE_SHA
    shared = ROOT / 'adhoc/v127'
    shared.mkdir(exist_ok=True)
    summary = dict(decision=result['decision'], diagnostic=result['diagnostic'],
                   collector_mechanism=load(RUN / 'mechanism/result.json'),
                   source_result=str((RUN / 'result.json').relative_to(ROOT)),
                   completed_at=result['completed_at'], validation1_opened=False)
    reports = [load(RUN / 'data' / f"{c['index']:04d}" / '127101/result.json')
               for c in load(RUN / 'input_manifest.json') if c['role'] == 'train']
    assert len(reports) == 512
    assert all(r['parent_result']['E'] == 0 and r['cpp_replays'] == r['rows'] for r in reports)
    summary['collection'] = dict(inputs=len(reports), all_parent_complete=True,
                                 cpp_child_replays=sum(r['cpp_replays'] for r in reports),
                                 python_child_replays=sum(r['python_replays'] for r in reports),
                                 seconds=load(RUN / 'collection/exit.json')['seconds'])
    loss_check = RUN / 'loss_check/mechanism/result.json'
    if loss_check.exists():
        summary['loss_mechanism'] = load(loss_check)
    if 'cross_validation' in result:
        cv = result['cross_validation']
        summary['cross_validation'] = dict(passed=cv['passed'], difference=cv['difference'],
                                           base=compact_evaluation(cv['base']), learned=compact_evaluation(cv['learned']),
                                           active_longer_selections=cv['active_longer_selections'])
    if 'assessment' in result:
        a = result['assessment']
        summary['assessment'] = dict(accepted=a['accepted'], difference=a['difference'],
                                     base=compact_evaluation(a['base']), learned=compact_evaluation(a['learned']))
    if 'final' in result:
        f = result['final']
        assert sha(ROOT / f['candidate']['source']) == f['candidate']['source_sha256']
        summary['final'] = {k: v for k, v in f.items() if k != 'tools_in'}
        summary['final']['tools_in'] = compact_evaluation(f['tools_in'])
    else:
        summary['retained'] = dict(version='v113', source=str(BASE.relative_to(ROOT)), source_sha256=BASE_SHA)
        previous = load(ROOT / 'results/nn_rank/v113/20261004_integrated_studio/final/result.json')
        assert previous['candidate']['source_sha256'] == BASE_SHA
        summary['retained_tools_in'] = dict(reused=True, **compact_evaluation(previous['tools_in']))
    dispatch = load(RUN / 'dispatch.json')
    guard = RUN / dispatch['guard_directory'] / 'exit.json'
    if guard.exists():
        summary['guard'] = load(guard)
    summary['model_training'] = []
    labels = load(RUN / 'training/averaged_labels.json')
    features = torch.tensor([row['features'] for row in labels])
    folds = load(RUN / 'training/folds.json')
    torch.set_num_threads(4)
    groups = {}
    for i, row in enumerate(labels):
        groups.setdefault(row['case'], []).append(i)
    for directory in sorted((RUN / 'training').glob('fold_*')):
        if not (directory / 'model.json').exists():
            continue
        model = load(directory / 'model.json')
        history = load(directory / 'history.json')
        fold = int(directory.name.split('_')[-1])
        net = model_new()
        net.load_state_dict({k: torch.tensor(v) for k, v in model['parameters'].items()})
        with torch.no_grad():
            predicted = 20 * net(features).squeeze(1).numpy()
        predicted = np.clip(predicted, 0, [row['before'] for row in labels])
        train_first, train_second = [], []
        for case, indices in groups.items():
            if folds[case] == fold:
                continue
            base = min(indices, key=lambda i: (labels[i]['before'], labels[i]['candidate']))
            chosen = min(indices, key=lambda i: (labels[i]['before'] - predicted[i], labels[i]['before'], labels[i]['candidate']))
            train_first.append(labels[chosen]['teacher_cost'] - labels[base]['teacher_cost'])
            train_second.append(labels[chosen]['after'] - labels[base]['after'])
        summary['model_training'].append(dict(name=directory.name, seconds=model['seconds'],
                                               epochs=model['epochs'], first=history[0], last=history[-1],
                                               train_inputs=len(train_first), train_teacher_difference=float(np.mean(train_first)),
                                               train_other_seed_difference=float(np.mean(train_second))))
        shutil.copy2(directory / 'model.json', shared / (directory.name + '.json'))
    for name in ['quality/input_comparisons.json', 'training/diagnostic_cases.json']:
        path = RUN / name
        if path.exists():
            rows = load(path)
            with (shared / (path.parent.name + '_' + path.stem + '.csv')).open('w', newline='') as stream:
                writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator='\n')
                writer.writeheader()
                writer.writerows(rows)
    save(RUN / 'summary.json', summary)
    save(shared / 'result_summary.json', summary)
    print({k: v for k, v in summary.items() if k not in ['collector_mechanism', 'model_training']})


if __name__ == '__main__':
    main()
