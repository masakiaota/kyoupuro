#!/usr/bin/env python3
"""同一入力・時点・乱数の計画間で、保持する完成解の費用差を学ぶ。"""
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

from train_v124_allocation import model_new
from v089_data import ROOT, save, sha, now, status

RUN = ROOT / 'results/nn_rank/v126/20261004_pair_allocation_studio'
PREVIOUS = ROOT / 'results/nn_rank/v124/20261004_plan_allocation_studio'


def load(path):
    return json.loads(path.read_text())


def grouped_loss(predicted, target, valid):
    count = predicted.shape[1]
    pair_mask = torch.triu(torch.ones((count, count), dtype=torch.bool), diagonal=1)
    pair_mask = pair_mask[None] & valid[:, :, None] & valid[:, None, :]
    pair_errors = nn.functional.smooth_l1_loss(
        predicted[:, :, None] - predicted[:, None, :],
        target[:, :, None] - target[:, None, :], reduction='none')
    pair_loss = ((pair_errors * pair_mask).sum((1, 2)) / pair_mask.sum((1, 2))).mean()
    point_errors = nn.functional.smooth_l1_loss(predicted, target, reduction='none')
    point_loss = ((point_errors * valid).sum(1) / valid.sum(1)).mean()
    return pair_loss + 0.1 * point_loss, pair_loss, point_loss


def dataset():
    assert load(PREVIOUS / 'collection/exit.json')['exit_code'] == 0
    expected = load(PREVIOUS / 'training/data_identity.json')
    rows = []
    identities = {}
    for case in load(PREVIOUS / 'input_manifest.json'):
        if case['role'] != 'train':
            continue
        for repeat in [124101, 124102, 124103, 124104]:
            path = PREVIOUS / 'data' / f"{case['index']:04d}" / str(repeat) / 'labels.jsonl'
            key = str(path.relative_to(PREVIOUS))
            identities[key] = sha(path)
            assert identities[key] == expected[key]
            for line in path.read_text().splitlines():
                row = json.loads(line)
                row.pop('path')
                rows.append(dict(row, case=case['index'], repeat=repeat))
    assert identities == expected
    save(RUN / 'training/data_identity.json', identities)
    folds = np.array(load(PREVIOUS / 'training/folds.json'), dtype=np.int64)
    assert len(folds) == 128 and set(folds) == set(range(4))
    save(RUN / 'training/folds.json', folds.tolist())
    group_indices = defaultdict(list)
    for i, row in enumerate(rows):
        group_indices[row['case'], row['repeat'], row['phase']].append(i)
    groups = list(group_indices.values())
    features = torch.zeros((len(groups), 4, 18))
    targets = torch.zeros((len(groups), 4))
    valid = torch.zeros((len(groups), 4), dtype=torch.bool)
    group_folds = []
    for g, indices in enumerate(groups):
        assert 3 <= len(indices) <= 4
        part = [rows[i] for i in indices]
        assert len({(r['case'], r['repeat'], r['phase']) for r in part}) == 1
        assert len({r['candidate'] for r in part}) == len(part)
        shortest = part[0]['shortest']
        assert all(r['shortest'] == shortest for r in part)
        assert shortest == min(r['before'] for r in part)
        group_folds.append(folds[part[0]['case']])
        for j, row in enumerate(part):
            assert row['after'] <= row['before']
            features[g, j] = torch.tensor(row['features'])
            # 現在の全体最短解を保持するので、候補が追いつかない場合も悪化しない。
            cost = min(shortest, row['after'])
            targets[g, j] = (row['before'] - cost) / 20
            valid[g, j] = True
    assert torch.isfinite(features).all() and torch.isfinite(targets).all()
    return rows, groups, features, targets, valid, np.array(group_folds), folds


def mechanism(x, y, valid):
    marker = RUN / 'mechanism/result.json'
    if marker.exists():
        assert load(marker)['passed']
        return
    test = torch.zeros((1, 2), requires_grad=True)
    desired = torch.tensor([[0.5, 0.1]])
    mask = torch.ones((1, 2), dtype=torch.bool)
    loss, pair, _ = grouped_loss(test, desired, mask)
    loss.backward()
    assert test.grad[0, 0] < 0 < test.grad[0, 1]
    exact, _, _ = grouped_loss(desired, desired, mask)
    assert exact.item() == 0
    _, shifted, _ = grouped_loss(test.detach() + 2, desired, mask)
    assert abs(shifted.item() - pair.item()) < 1e-7
    torch.manual_seed(124004)
    model = model_new()
    optimizer = torch.optim.AdamW(model.parameters(), lr=.001, weight_decay=.0001)
    batch = (x[:32], y[:32], valid[:32])
    losses = []
    for _ in range(51):
        loss, _, _ = grouped_loss(model(batch[0]).squeeze(-1), batch[1], batch[2])
        losses.append(loss.item())
        optimizer.zero_grad(set_to_none=True)
        loss.backward()
        optimizer.step()
    assert losses[-1] < losses[0]
    # この診断の更新状態は本学習へ持ち込まず、各分割で初期化し直す。
    save(marker, dict(passed=True, grouped_records=len(x), teacher_rows=int(valid.sum()),
                     batch_initial_loss=losses[0], batch_final_loss=losses[-1],
                     gradient_direction=True, common_offset_pair_invariant=True,
                     completed_at=now()))


def train_one(name, x, y, valid):
    where = RUN / 'training' / name
    where.mkdir(parents=True, exist_ok=True)
    if (where / 'model.json').exists():
        return load(where / 'model.json')
    torch.manual_seed(124004)
    model = model_new()
    optimizer = torch.optim.AdamW(model.parameters(), lr=.001, weight_decay=.0001)
    checkpoint = where / 'latest.pt'
    begin = 0
    history = []
    if checkpoint.exists():
        saved = torch.load(checkpoint, map_location='cpu', weights_only=False)
        model.load_state_dict(saved['model'])
        optimizer.load_state_dict(saved['optimizer'])
        begin, history = saved['epoch'], saved['history']
        torch.set_rng_state(saved['rng'])
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
                  checkpoint_sha256=sha(checkpoint), architecture=[18, 16, 1], epochs=500,
                  groups=len(x), rows=int(valid.sum()), seconds=time.monotonic() - tick,
                  completed_at=now())
    save(where / 'history.json', history)
    save(where / 'model.json', result)
    return result


def run(full=False):
    RUN.mkdir(parents=True, exist_ok=True)
    torch.set_num_threads(4)
    torch.set_num_interop_threads(1)
    config = dict(script_sha256=sha(Path(__file__)), parent_teacher_identity_sha256=sha(PREVIOUS / 'training/data_identity.json'),
                  folds_sha256=sha(PREVIOUS / 'training/folds.json'), model=[18, 16, 1], epochs=500,
                  seed=124004, batch_groups=32, auxiliary=.1, scale=20, threads=4)
    if (RUN / 'training/config.json').exists():
        assert load(RUN / 'training/config.json') == config
    else:
        save(RUN / 'training/config.json', config)
        frozen = RUN / 'frozen'
        frozen.mkdir(exist_ok=True)
        shutil.copy2(__file__, frozen / Path(__file__).name)
    rows, groups, x, y, valid, group_folds, folds = dataset()
    mechanism(x, y, valid)
    if full:
        assert load(RUN / 'cross_validation/result.json')['passed']
        train_one('full', x, y, valid)
        return
    if (RUN / 'training/result.json').exists():
        return
    predictions = np.zeros(len(rows))
    for fold in range(4):
        held = group_folds == fold
        assert not ({rows[groups[g][0]]['case'] for g in np.flatnonzero(held)} &
                    {rows[groups[g][0]]['case'] for g in np.flatnonzero(~held)})
        trained = train_one(f'fold_{fold}', x[~held], y[~held], valid[~held])
        model = model_new()
        model.load_state_dict({k: torch.tensor(v) for k, v in trained['parameters'].items()})
        with torch.no_grad():
            output = 20 * model(x[held]).squeeze(-1).numpy()
        for row, g in enumerate(np.flatnonzero(held)):
            predictions[groups[g]] = output[row, :len(groups[g])]
    predictions = np.clip(predictions, 0, np.array([r['before'] for r in rows]))
    pairs = []
    for indices in groups:
        row = rows[indices[0]]
        if row['phase'] != 0:
            continue
        base = min(indices, key=lambda i: (rows[i]['before'], rows[i]['candidate']))
        chosen = min(indices, key=lambda i: (rows[i]['before'] - predictions[i], rows[i]['before'], rows[i]['candidate']))
        shortest = row['shortest']
        before = min(shortest, rows[base]['after'])
        after = min(shortest, rows[chosen]['after'])
        pairs.append(dict(case=row['case'], repeat=row['repeat'], fold=int(folds[row['case']]),
                          baseline=before, learned=after, difference=after - before,
                          oracle=min(shortest, min(rows[i]['after'] for i in indices)),
                          changed=chosen != base))
    cases = []
    for case in range(128):
        values = [r['difference'] for r in pairs if r['case'] == case]
        assert len(values) == 4
        cases.append(dict(case=case, fold=int(folds[case]), difference=float(np.mean(values))))
    fold_differences = [float(np.mean([r['difference'] for r in cases if r['fold'] == f])) for f in range(4)]
    difference = float(np.mean([r['difference'] for r in cases]))
    result = dict(passed=difference <= -.25 and sum(d < 0 for d in fold_differences) >= 3,
                  cases=128, rows=len(rows), groups=len(pairs), mean_difference=difference,
                  fold_differences=fold_differences, changed=sum(r['changed'] for r in pairs),
                  oracle_mean_difference=float(np.mean([r['oracle'] - r['baseline'] for r in pairs])),
                  parameters=sum(p.numel() for p in model_new().parameters()), completed_at=now())
    save(RUN / 'training/diagnostic_pairs.json', pairs)
    save(RUN / 'training/diagnostic_cases.json', cases)
    save(RUN / 'training/result.json', result)
    np.savez_compressed(RUN / 'training/predictions.npz', predictions=predictions,
                        features=np.array([r['features'] for r in rows], dtype=np.float32))
    status(RUN / 'training', 'completed', **result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--full', action='store_true')
    run(parser.parse_args().full)
