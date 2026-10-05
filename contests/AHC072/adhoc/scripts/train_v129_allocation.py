#!/usr/bin/env python3
"""計画構造の24特徴と、同じ容量で追加特徴を0にした対照を比較する。"""
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
from build_v129_allocation import ROOT, RUN, BASE, BASE_SHA
from v089_data import save, sha, now, status


def load(path):
    return json.loads(path.read_text())


def model_new():
    return nn.Sequential(nn.Linear(42, 64), nn.ReLU(), nn.Linear(64, 64),
                         nn.ReLU(), nn.Linear(64, 1))


def dataset():
    assert load(RUN / 'collection/exit.json')['exit_code'] == 0
    rows, identity = [], {}
    for case in load(RUN / 'input_manifest.json'):
        if case['role'] != 'train':
            continue
        directory = RUN / 'data' / f"{case['index']:04d}" / '129101'
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
    order = np.random.default_rng(129003).permutation(512)
    folds = np.empty(512, dtype=np.int64)
    for i, group in enumerate(np.array_split(order, 4)):
        folds[group] = i
    save(RUN / 'training/folds.json', folds.tolist())
    by_case = defaultdict(list)
    for i, row in enumerate(rows):
        by_case[row['case']].append(i)
    groups = list(by_case.values())
    x = torch.zeros((len(groups), 4, 42))
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

def mechanism(x, y, valid):
    marker = RUN / 'loss_check/result.json'
    if marker.exists():
        assert load(marker)['passed']
        return
    torch.manual_seed(124004)
    model = model_new()
    assert sum(p.numel() for p in model.parameters()) == 6977
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
    save(marker, dict(passed=True, parameters=6977, gradient_direction=True,
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
                  checkpoint_sha256=sha(checkpoint), architecture=[42, 64, 64, 1], epochs=500,
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
    assert sha(BASE)==BASE_SHA
    config=dict(source_sha256=sha(Path(__file__)), loss_sha256=sha(ROOT/'adhoc/scripts/train_v126_allocation.py'),
                collection_sha256=sha(RUN/'collection_config.json'), architecture=[42,64,64,1], parameters=6977,
                epochs=500, seed=124004, fold_seed=129003, arms=['control_zero','structure'], threads=4,
                teacher_rollouts=[0,1,2,3], diagnosis_rollouts=[4,5,6,7])
    if (RUN/'training/config.json').exists(): assert load(RUN/'training/config.json')==config
    else:
        save(RUN/'training/config.json',config)
        frozen=RUN/'training_frozen';frozen.mkdir(exist_ok=True)
        for path in [Path(__file__),ROOT/'adhoc/scripts/train_v126_allocation.py']:
            shutil.copy2(path,frozen/path.name)
    rows,groups,x,y,valid,group_folds,folds=dataset()
    teacher=quality(rows,groups)
    if not teacher['passed']:
        result=dict(passed=False,reason='teacher_quality_failed',quality=teacher,completed_at=now())
        save(RUN/'training/result.json',result);status(RUN/'training','completed',**result);return
    mechanism(x,y,valid)
    if full:
        assert load(RUN/'cross_validation/result.json')['passed']
        train_one('full',x,y,valid);return
    if (RUN/'training/result.json').exists():return
    outcomes={};structured_predictions=None
    for arm in ['control_zero','structure']:
        current=x.clone()
        if arm=='control_zero':current[:,:,18:]=0
        predictions=np.zeros(len(rows));training=[]
        for fold in range(4):
            held=group_folds==fold
            assert not ({rows[groups[g][0]]['case'] for g in np.flatnonzero(held)} &
                        {rows[groups[g][0]]['case'] for g in np.flatnonzero(~held)})
            name=f'control_fold_{fold}' if arm=='control_zero' else f'fold_{fold}'
            trained=train_one(name,current[~held],y[~held],valid[~held])
            model=model_new();model.load_state_dict({k:torch.tensor(v) for k,v in trained['parameters'].items()})
            with torch.no_grad():output=20*model(current).squeeze(-1).numpy()
            all_predictions=np.zeros(len(rows))
            for g,indices in enumerate(groups):
                all_predictions[indices]=output[g,:len(indices)]
                if held[g]:predictions[indices]=all_predictions[indices]
            observed=[r for r in compare(rows,groups,all_predictions,folds) if r['fold']!=fold]
            history=load(RUN/'training'/name/'history.json')
            training.append(dict(fold=fold,seconds=trained['seconds'],train_inputs=len(observed),
                                 teacher_difference=float(np.mean([r['teacher_difference'] for r in observed])),
                                 other_seed_difference=float(np.mean([r['difference'] for r in observed])),
                                 first_loss=history[0]['loss'],final_loss=history[-1]['loss']))
        comparisons=compare(rows,groups,predictions,folds)
        differences=[float(np.mean([r['difference'] for r in comparisons if r['fold']==f])) for f in range(4)]
        mean=float(np.mean([r['difference'] for r in comparisons]))
        outcomes[arm]=dict(cases=len(groups),mean_difference=mean,fold_differences=differences,
                           changed=sum(r['changed'] for r in comparisons),training=training)
        save(RUN/'training'/f'{arm}_diagnostic_cases.json',comparisons)
        if arm=='structure':
            structured_predictions=predictions
            save(RUN/'training/diagnostic_cases.json',comparisons)
    selected=outcomes['structure'];control=outcomes['control_zero']
    passed=(selected['mean_difference']<=-.25 and sum(d<0 for d in selected['fold_differences'])>=3 and
            selected['mean_difference']<control['mean_difference'])
    result=dict(passed=passed,cases=len(groups),rows=len(rows),parameters=6977,quality=teacher,
                arms=outcomes,mean_difference=selected['mean_difference'],fold_differences=selected['fold_differences'],
                structure_vs_control=selected['mean_difference']-control['mean_difference'],completed_at=now())
    save(RUN/'training/result.json',result)
    np.savez_compressed(RUN/'training/predictions.npz',predictions=structured_predictions,
                        features=np.array([r['features'] for r in rows],dtype=np.float32))
    status(RUN/'training','completed',**result)
    print(json.dumps(result,ensure_ascii=False))


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--full',action='store_true');run(parser.parse_args().full)
