#!/usr/bin/env python3
"""固定された親状態の複数乱数平均を教師にし、別の乱数群で診断する。"""
import argparse
from collections import defaultdict
import json
from pathlib import Path
import shutil

import numpy as np
import torch

import train_v126_allocation as learning
from build_v127_allocation import ROOT, RUN
from v089_data import save, sha, now, status

model_new = learning.model_new


def load(path):
    return json.loads(path.read_text())


def dataset():
    assert load(RUN / 'collection/exit.json')['exit_code'] == 0
    rows, identity = [], {}
    for case in load(RUN / 'input_manifest.json'):
        if case['role'] != 'train':
            continue
        directory = RUN / 'data' / f"{case['index']:04d}" / '127101'
        report = load(directory / 'result.json')
        assert report['identity']['input_sha256'] == case['sha256']
        path = directory / 'labels.jsonl'
        identity[str(path.relative_to(RUN))] = sha(path)
        by_candidate = defaultdict(list)
        for line in path.read_text().splitlines():
            row = json.loads(line)
            by_candidate[row['candidate']].append(row)
        for candidate, part in sorted(by_candidate.items()):
            part.sort(key=lambda row: row['rollout'])
            assert len(part) == 8 and [r['rollout'] for r in part] == list(range(8))
            assert len({r['prefix_hash'] for r in part}) == 1
            assert len({tuple(r['features']) for r in part}) == 1
            first = part[0]
            costs = [min(r['shortest'], r['after']) for r in part]
            rows.append(dict(case=case['index'], candidate=candidate, before=first['before'],
                             shortest=first['shortest'], features=first['features'],
                             teacher_cost=float(np.mean(costs[:4])), after=float(np.mean(costs[4:])),
                             costs=costs, raw_after=[r['after'] for r in part], prefix_hash=first['prefix_hash']))
    save(RUN / 'training/data_identity.json', identity)
    order = np.random.default_rng(127003).permutation(512)
    folds = np.empty(512, dtype=np.int64)
    for i, group in enumerate(np.array_split(order, 4)):
        folds[group] = i
    save(RUN / 'training/folds.json', folds.tolist())
    by_case = defaultdict(list)
    for i, row in enumerate(rows):
        by_case[row['case']].append(i)
    groups = list(by_case.values())
    x = torch.zeros((len(groups), 4, 18))
    y = torch.zeros((len(groups), 4))
    valid = torch.zeros((len(groups), 4), dtype=torch.bool)
    group_folds = []
    for g, indices in enumerate(groups):
        assert 3 <= len(indices) <= 4
        assert len({rows[i]['case'] for i in indices}) == 1
        shortest = rows[indices[0]]['shortest']
        assert shortest == min(rows[i]['before'] for i in indices)
        group_folds.append(folds[rows[indices[0]]['case']])
        for j, i in enumerate(indices):
            row = rows[i]
            assert row['teacher_cost'] <= shortest and row['after'] <= shortest
            x[g, j] = torch.tensor(row['features'])
            y[g, j] = (row['before'] - row['teacher_cost']) / 20
            valid[g, j] = True
    save(RUN / 'training/averaged_labels.json', rows)
    return rows, groups, x, y, valid, np.array(group_folds), folds


def quality(rows, groups):
    comparisons, pair_signs = [], []
    for indices in groups:
        base = min(indices, key=lambda i: (rows[i]['before'], rows[i]['candidate']))
        selected = min(indices, key=lambda i: (rows[i]['teacher_cost'], rows[i]['before'], rows[i]['candidate']))
        comparisons.append(dict(case=rows[base]['case'], selected=rows[selected]['candidate'],
                                baseline=rows[base]['after'], selected_cost=rows[selected]['after'],
                                difference=rows[selected]['after'] - rows[base]['after']))
        for position, i in enumerate(indices):
            for j in indices[position + 1:]:
                first = rows[i]['teacher_cost'] - rows[j]['teacher_cost']
                second = rows[i]['after'] - rows[j]['after']
                if abs(first) >= 1 and abs(second) >= 1:
                    pair_signs.append(first * second > 0)
    mean = float(np.mean([r['difference'] for r in comparisons])) if comparisons else 0.
    agreement = float(np.mean(pair_signs)) if pair_signs else 0.
    result = dict(passed=len(groups) >= 384 and mean <= -.25 and len(pair_signs) >= 128 and agreement >= .6,
                  eligible_inputs=len(groups), transfer_mean_difference=mean, pairs=len(pair_signs),
                  sign_agreement=agreement, measured_completions=8 * len(rows), completed_at=now())
    save(RUN / 'quality/result.json', result)
    save(RUN / 'quality/input_comparisons.json', comparisons)
    return result


def run(full=False):
    RUN.mkdir(parents=True, exist_ok=True)
    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    # v126の学習手順をそのまま利用し、このプロセスの保存先だけをv127にする。
    learning.RUN = RUN
    config = dict(script_sha256=sha(Path(__file__)), learning_sha256=sha(ROOT / 'adhoc/scripts/train_v126_allocation.py'),
                  collection_config_sha256=sha(RUN / 'collection_config.json'),
                  model=[18, 16, 1], epochs=500, seed=124004, fold_seed=127003,
                  teacher_rollouts=[0, 1, 2, 3], diagnosis_rollouts=[4, 5, 6, 7], threads=4)
    if (RUN / 'training/config.json').exists():
        assert load(RUN / 'training/config.json') == config
    else:
        save(RUN / 'training/config.json', config)
        frozen = RUN / 'training_frozen'
        frozen.mkdir(exist_ok=True)
        for path in [Path(__file__), ROOT / 'adhoc/scripts/train_v126_allocation.py']:
            shutil.copy2(path, frozen / path.name)
    rows, groups, x, y, valid, group_folds, folds = dataset()
    teacher = quality(rows, groups)
    if not teacher['passed']:
        result = dict(passed=False, reason='teacher_quality_failed', quality=teacher, completed_at=now())
        save(RUN / 'training/result.json', result)
        status(RUN / 'training', 'completed', **result)
        return
    learning.RUN = RUN / 'loss_check'
    learning.mechanism(x, y, valid)
    learning.RUN = RUN
    if full:
        assert load(RUN / 'cross_validation/result.json')['passed']
        learning.train_one('full', x, y, valid)
        return
    if (RUN / 'training/result.json').exists():
        return
    predictions = np.zeros(len(rows))
    for fold in range(4):
        held = group_folds == fold
        trained = learning.train_one(f'fold_{fold}', x[~held], y[~held], valid[~held])
        model = model_new()
        model.load_state_dict({k: torch.tensor(v) for k, v in trained['parameters'].items()})
        with torch.no_grad():
            output = 20 * model(x[held]).squeeze(-1).numpy()
        for j, g in enumerate(np.flatnonzero(held)):
            predictions[groups[g]] = output[j, :len(groups[g])]
    predictions = np.clip(predictions, 0, [r['before'] for r in rows])
    comparisons = []
    for indices in groups:
        base = min(indices, key=lambda i: (rows[i]['before'], rows[i]['candidate']))
        chosen = min(indices, key=lambda i: (rows[i]['before'] - predictions[i], rows[i]['before'], rows[i]['candidate']))
        case = rows[base]['case']
        comparisons.append(dict(case=case, fold=int(folds[case]), baseline=rows[base]['after'],
                                learned=rows[chosen]['after'], difference=rows[chosen]['after'] - rows[base]['after'],
                                changed=chosen != base))
    differences = [float(np.mean([r['difference'] for r in comparisons if r['fold'] == f])) for f in range(4)]
    mean = float(np.mean([r['difference'] for r in comparisons]))
    result = dict(passed=mean <= -.25 and sum(d < 0 for d in differences) >= 3,
                  cases=len(groups), rows=len(rows), mean_difference=mean, fold_differences=differences,
                  changed=sum(r['changed'] for r in comparisons), quality=teacher, parameters=321, completed_at=now())
    save(RUN / 'training/diagnostic_cases.json', comparisons)
    save(RUN / 'training/result.json', result)
    np.savez_compressed(RUN / 'training/predictions.npz', predictions=predictions,
                        features=np.array([r['features'] for r in rows], dtype=np.float32))
    status(RUN / 'training', 'completed', **result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--full', action='store_true')
    run(parser.parse_args().full)
