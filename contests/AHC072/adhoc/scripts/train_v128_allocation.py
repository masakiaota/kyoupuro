#!/usr/bin/env python3
"""v127の教師と更新条件を保持し、計画選択器の容量だけを増やす。"""
import argparse
from collections import defaultdict
import json
import math
from pathlib import Path
import shutil
import time

import numpy as np
import torch
from torch import nn

from train_v126_allocation import grouped_loss
from v089_data import ROOT, save, sha, now, status

RUN = ROOT / 'results/nn_rank/v128/20261004_capacity_allocation_studio'
PREVIOUS = ROOT / 'results/nn_rank/v127/20261004_repeated_allocation_studio'
BASE = ROOT / 'src/bin/v113_integrated_nn_lns.cpp'
BASE_SHA = '037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'


def load(path):
    return json.loads(path.read_text())


def model_new():
    return nn.Sequential(nn.Linear(18, 64), nn.ReLU(), nn.Linear(64, 64),
                         nn.ReLU(), nn.Linear(64, 1))


def dataset():
    assert sha(BASE) == BASE_SHA
    assert load(PREVIOUS / 'quality/result.json')['passed']
    assert load(PREVIOUS / 'pipeline/exit.json')['exit_code'] == 0
    files = ['training/averaged_labels.json', 'training/folds.json',
             'training/data_identity.json', 'input_manifest.json']
    identity = {name: sha(PREVIOUS / name) for name in files}
    identity['source'] = sha(Path(__file__))
    identity['loss_source'] = sha(ROOT / 'adhoc/scripts/train_v126_allocation.py')
    identity['base'] = sha(BASE)
    identity['architecture'] = [18, 64, 64, 1]
    identity['epochs'] = 500
    identity['seed'] = 124004
    identity['threads'] = 4
    identity['batch_groups'] = 32
    identity['lr'] = [.001, .0001]
    identity['weight_decay'] = .0001
    identity['point_loss_weight'] = .1
    identity['scale'] = 20
    where = RUN / 'training/config.json'
    if where.exists():
        assert load(where) == identity
    else:
        save(where, identity)
        frozen = RUN / 'training_frozen'
        frozen.mkdir(exist_ok=True)
        for p in [Path(__file__), ROOT / 'adhoc/scripts/train_v126_allocation.py']:
            shutil.copy2(p, frozen / p.name)
    for name in files[:3]:
        target = RUN / name
        target.parent.mkdir(parents=True, exist_ok=True)
        if target.exists():
            assert sha(target) == identity[name]
        else:
            shutil.copy2(PREVIOUS / name, target)
    # 生成済みの入力を参照する。確認用256入力は評価工程まで読み込まない。
    manifest = [dict(row, path=str(PREVIOUS / row['path']))
                for row in load(PREVIOUS / 'input_manifest.json')]
    manifest_path = RUN / 'input_manifest.json'
    if manifest_path.exists():
        assert load(manifest_path) == manifest
    else:
        save(manifest_path, manifest)
    rows = load(RUN / 'training/averaged_labels.json')
    folds = np.array(load(RUN / 'training/folds.json'), dtype=np.int64)
    assert len(rows) == 2039 and len(folds) == 512
    by_case = defaultdict(list)
    for i, row in enumerate(rows):
        by_case[row['case']].append(i)
    groups = list(by_case.values())
    assert len(groups) == 511
    x = torch.zeros((len(groups), 4, 18))
    y = torch.zeros((len(groups), 4))
    valid = torch.zeros((len(groups), 4), dtype=torch.bool)
    group_folds = []
    for g, indices in enumerate(groups):
        assert 3 <= len(indices) <= 4
        assert len({rows[i]['case'] for i in indices}) == 1
        group_folds.append(folds[rows[indices[0]]['case']])
        for j, i in enumerate(indices):
            row = rows[i]
            assert row['teacher_cost'] <= row['shortest'] <= row['before']
            x[g, j] = torch.tensor(row['features'])
            y[g, j] = (row['before'] - row['teacher_cost']) / 20
            valid[g, j] = True
    assert torch.isfinite(x).all() and torch.isfinite(y).all()
    return rows, groups, x, y, valid, np.array(group_folds), folds


def mechanism(x, y, valid):
    marker = RUN / 'mechanism/result.json'
    if marker.exists():
        assert load(marker)['passed']
        return
    torch.manual_seed(124004)
    model = model_new()
    assert sum(p.numel() for p in model.parameters()) == 5441
    assert model(x[:1]).shape == (1, 4, 1)
    prediction = torch.zeros((1, 2), requires_grad=True)
    target = torch.tensor([[.5, .1]])
    mask = torch.ones((1, 2), dtype=torch.bool)
    loss, _, _ = grouped_loss(prediction, target, mask)
    loss.backward()
    assert prediction.grad[0, 0] < 0 < prediction.grad[0, 1]
    optimizer = torch.optim.AdamW(model.parameters(), lr=.001, weight_decay=.0001)
    losses = []
    for _ in range(51):
        loss, _, _ = grouped_loss(model(x[:32]).squeeze(-1), y[:32], valid[:32])
        losses.append(loss.item())
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        assert all(torch.isfinite(p.grad).all() for p in model.parameters())
        optimizer.step()
    assert losses[-1] < losses[0]
    save(marker, dict(passed=True, parameters=5441, gradient_direction=True,
                     initial_batch_loss=losses[0], final_batch_loss=losses[-1],
                     diagnostic_weights_discarded=True, completed_at=now()))


def train_one(name, x, y, valid):
    where = RUN / 'training' / name
    where.mkdir(parents=True, exist_ok=True)
    if (where / 'model.json').exists():
        return load(where / 'model.json')
    torch.manual_seed(124004)
    model = model_new()
    optimizer = torch.optim.AdamW(model.parameters(), lr=.001, weight_decay=.0001)
    checkpoint = where / 'latest.pt'
    begin, history = 0, []
    if checkpoint.exists():
        state = torch.load(checkpoint, map_location='cpu', weights_only=False)
        model.load_state_dict(state['model'])
        optimizer.load_state_dict(state['optimizer'])
        begin, history = state['epoch'], state['history']
        torch.set_rng_state(state['rng'])
    tick = time.monotonic()
    for epoch in range(begin, 500):
        lr = .0001 + .5 * (.001 - .0001) * (1 + math.cos(math.pi * epoch / 499))
        for group in optimizer.param_groups:
            group['lr'] = lr
        total = np.zeros(3)
        for ids in torch.randperm(len(x)).split(32):
            loss, pair, point = grouped_loss(model(x[ids]).squeeze(-1), y[ids], valid[ids])
            optimizer.zero_grad(set_to_none=True)
            loss.backward()
            optimizer.step()
            total += np.array([loss.item(), pair.item(), point.item()]) * len(ids)
        history.append(dict(epoch=epoch + 1, loss=total[0] / len(x),
                            pair_loss=total[1] / len(x), point_loss=total[2] / len(x), lr=lr))
        if (epoch + 1) % 50 == 0:
            torch.save(dict(model=model.state_dict(), optimizer=optimizer.state_dict(),
                            epoch=epoch + 1, history=history, rng=torch.get_rng_state()), where / 'latest.tmp')
            (where / 'latest.tmp').replace(checkpoint)
            status(RUN / 'training', 'training', model=name, epoch=epoch + 1)
    result = dict(parameters={k: v.tolist() for k, v in model.state_dict().items()},
                  checkpoint_sha256=sha(checkpoint), architecture=[18, 64, 64, 1], epochs=500,
                  groups=len(x), rows=int(valid.sum()), seconds=time.monotonic() - tick,
                  completed_at=now())
    save(where / 'history.json', history)
    save(where / 'model.json', result)
    return result


def compare(rows, groups, predictions, folds):
    predictions = np.clip(predictions, 0, [row['before'] for row in rows])
    result = []
    for indices in groups:
        base = min(indices, key=lambda i: (rows[i]['before'], rows[i]['candidate']))
        chosen = min(indices, key=lambda i: (rows[i]['before'] - predictions[i],
                                             rows[i]['before'], rows[i]['candidate']))
        case = rows[base]['case']
        result.append(dict(case=case, fold=int(folds[case]), baseline=rows[base]['after'],
                           learned=rows[chosen]['after'], difference=rows[chosen]['after'] - rows[base]['after'],
                           teacher_difference=rows[chosen]['teacher_cost'] - rows[base]['teacher_cost'],
                           changed=chosen != base))
    return result


def run(full=False):
    RUN.mkdir(parents=True, exist_ok=True)
    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    rows, groups, x, y, valid, group_folds, folds = dataset()
    mechanism(x, y, valid)
    if full:
        assert load(RUN / 'cross_validation/result.json')['passed']
        train_one('full', x, y, valid)
        return
    if (RUN / 'training/result.json').exists():
        return
    predictions = np.zeros(len(rows))
    training = []
    for fold in range(4):
        held = group_folds == fold
        assert not ({rows[groups[g][0]]['case'] for g in np.flatnonzero(held)} &
                    {rows[groups[g][0]]['case'] for g in np.flatnonzero(~held)})
        trained = train_one(f'fold_{fold}', x[~held], y[~held], valid[~held])
        model = model_new()
        model.load_state_dict({k: torch.tensor(v) for k, v in trained['parameters'].items()})
        with torch.no_grad():
            output = 20 * model(x).squeeze(-1).numpy()
        all_predictions = np.zeros(len(rows))
        for g, indices in enumerate(groups):
            all_predictions[indices] = output[g, :len(indices)]
            if held[g]:
                predictions[indices] = all_predictions[indices]
        observed = [r for r in compare(rows, groups, all_predictions, folds) if r['fold'] != fold]
        history = load(RUN / 'training' / f'fold_{fold}' / 'history.json')
        training.append(dict(fold=fold, seconds=trained['seconds'], train_inputs=len(observed),
                             teacher_difference=float(np.mean([r['teacher_difference'] for r in observed])),
                             other_seed_difference=float(np.mean([r['difference'] for r in observed])),
                             first_loss=history[0]['loss'], final_loss=history[-1]['loss']))
    comparisons = compare(rows, groups, predictions, folds)
    differences = [float(np.mean([r['difference'] for r in comparisons if r['fold'] == f])) for f in range(4)]
    mean = float(np.mean([r['difference'] for r in comparisons]))
    result = dict(passed=mean <= -.25 and sum(d < 0 for d in differences) >= 3,
                  cases=len(groups), rows=len(rows), mean_difference=mean, fold_differences=differences,
                  changed=sum(r['changed'] for r in comparisons), parameters=5441, training=training,
                  parent_result_sha256=sha(PREVIOUS / 'training/result.json'),
                  parent_mean_difference=load(PREVIOUS / 'training/result.json')['mean_difference'],
                  completed_at=now())
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
