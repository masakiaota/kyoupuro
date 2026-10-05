#!/usr/bin/env python3
"""保存した候補対モデルの診断を集計する。学習とsolverを再実行しない。"""
import csv
from collections import defaultdict
import json
import shutil

import numpy as np
import torch

from train_v126_allocation import RUN, PREVIOUS, load, model_new
from v089_data import ROOT, save, sha, now


def main():
    torch.set_num_threads(4)
    result = load(RUN / 'training/result.json')
    assert not result['passed'], 'passed diagnosis must continue to the registered solver comparison'
    rows = []
    for case in load(PREVIOUS / 'input_manifest.json'):
        if case['role'] != 'train':
            continue
        for repeat in [124101, 124102, 124103, 124104]:
            path = PREVIOUS / 'data' / f"{case['index']:04d}" / str(repeat) / 'labels.jsonl'
            for line in path.read_text().splitlines():
                row = json.loads(line)
                row.pop('path')
                rows.append(dict(row, case=case['index'], repeat=repeat))
    matrix = torch.tensor([r['features'] for r in rows])
    folds = load(RUN / 'training/folds.json')
    groups = defaultdict(list)
    for i, row in enumerate(rows):
        if row['phase'] == 0:
            groups[row['case'], row['repeat']].append(i)
    training = []
    histories = []
    for fold in range(4):
        directory = RUN / 'training' / f'fold_{fold}'
        model = model_new()
        fitted = load(directory / 'model.json')
        model.load_state_dict({k: torch.tensor(v) for k, v in fitted['parameters'].items()})
        with torch.no_grad():
            predictions = 20 * model(matrix).squeeze(1).numpy()
        predictions = np.clip(predictions, 0, [r['before'] for r in rows])
        differences = []
        changed = 0
        for (case, repeat), indices in groups.items():
            if folds[case] == fold:
                continue
            base = min(indices, key=lambda i: (rows[i]['before'], rows[i]['candidate']))
            chosen = min(indices, key=lambda i: (rows[i]['before'] - predictions[i], rows[i]['before'], rows[i]['candidate']))
            shortest = rows[base]['shortest']
            differences.append(min(shortest, rows[chosen]['after']) - min(shortest, rows[base]['after']))
            changed += chosen != base
        training.append(dict(fold=fold, groups=len(differences), changed=changed,
                             mean_difference=float(np.mean(differences))))
        history = load(directory / 'history.json')
        assert history[-1]['epoch'] == 500 and len(history) == 500
        histories.append(dict(fold=fold, first=history[0], last=history[-1], seconds=fitted['seconds'],
                              model_sha256=sha(directory / 'model.json')))
    base = ROOT / 'src/bin/v113_integrated_nn_lns.cpp'
    assert sha(base) == '037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
    summary = dict(decision='diagnostic_failed', diagnostic=result, mechanism=load(RUN / 'mechanism/result.json'),
                   training_selection=training, histories=histories, config=load(RUN / 'training/config.json'),
                   retained=dict(version='v113', source=str(base.relative_to(ROOT)), source_sha256=sha(base)),
                   cpp_integration_evaluated=False, new_256_evaluated=False, tools_in_evaluated=False,
                   validation1_opened=False, completed_at=now())
    save(RUN / 'result.json', summary)
    shared = ROOT / 'adhoc/v126'
    shared.mkdir(exist_ok=True)
    save(shared / 'result_summary.json', summary)
    with (shared / 'diagnostic_pairs.csv').open('w', newline='') as stream:
        pairs = load(RUN / 'training/diagnostic_pairs.json')
        writer = csv.DictWriter(stream, fieldnames=list(pairs[0]), lineterminator='\n')
        writer.writeheader()
        writer.writerows(pairs)
    for fold in range(4):
        shutil.copy2(RUN / 'training' / f'fold_{fold}' / 'model.json', shared / f'model_fold_{fold}.json')
    print(json.dumps({k: v for k, v in summary.items() if k != 'config'}, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
