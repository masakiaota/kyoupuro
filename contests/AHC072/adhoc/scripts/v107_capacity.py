#!/usr/bin/env python3
"""既存の関数を保持する幅拡張と、入力単位で固定した教師の抽出。"""
import argparse
import copy
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

from train_v090_board import Model
from train_v103_imitation import Teacher, TeacherEngine
from train_v077_rank import checkpoint
from v090_data import Dataset, RUN as BC_RUN
from v091_env import build_library, load
from v089_data import ROOT, save, sha, now, Geometry
from v101_compute import distribution
from check_v098_complete import action_line

RUN = ROOT / 'results/nn_rank/v107/20261004_capacity_studio'
INITIAL = ROOT / 'results/nn_rank/v105/20261004_nn_lns_studio/frozen/checkpoint.pt'
INITIAL_SHA = 'e9d615587a1a655b7b2818e2f19564619050e2ffd669050b015b7c46766b90d3'
SPECS = {'small': dict(width=48, depth=4, hidden=64), 'wide': dict(width=64, depth=4, hidden=112)}


def widen(old):
    """複製された入力接続を人数で割り、残差枝を含む関数を保つ。"""
    new = Model(SPECS['wide'])
    a = old.state_dict(); b = new.state_dict()
    ci = torch.arange(64) % 48; hi = torch.arange(112) % 64
    cc = torch.bincount(ci, minlength=48)[ci].float()
    hc = torch.bincount(hi, minlength=64)[hi].float()
    b['input.weight'] = a['input.weight'][ci].clone()
    b['input.bias'] = a['input.bias'][ci].clone()
    for i in range(4):
        prefix = f'blocks.{i}.'
        for name in ('depth.weight', 'depth.bias', 'point.bias'):
            b[prefix+name] = a[prefix+name][ci].clone()
        b[prefix+'point.weight'] = a[prefix+'point.weight'][ci][:, ci] / cc[None, :, None, None]
    w = a['actor.0.weight'][hi]
    b['actor.0.weight'] = torch.cat([w[:, j*48+ci]/cc for j in range(3)] + [w[:, 144:]], 1)
    b['actor.0.bias'] = a['actor.0.bias'][hi].clone()
    b['actor.2.weight'] = a['actor.2.weight'][:, hi] / hc
    b['actor.2.bias'] = a['actor.2.bias'].clone()
    b['critic.0.weight'] = a['critic.0.weight'][hi][:, ci] / cc
    b['critic.0.bias'] = a['critic.0.bias'][hi].clone()
    b['critic.2.weight'] = a['critic.2.weight'][:, hi] / hc
    b['critic.2.bias'] = a['critic.2.bias'].clone()
    # 同じ活性を持つ複製先の接続和は保ち、逆伝播を完全対称にしない。
    generator = torch.Generator().manual_seed(107099)
    for original in range(64):
        positions = torch.nonzero(hi == original).flatten()
        if len(positions) < 2:
            continue
        perturb = torch.randn(len(positions), generator=generator) * 1e-4
        perturb -= perturb.mean()
        b['actor.2.weight'][0, positions] += perturb
    new.load_state_dict(b)
    return new


def initial_model(capacity, device='cpu'):
    assert sha(INITIAL) == INITIAL_SHA
    model = Model(SPECS['small'])
    model.load_state_dict(torch.load(INITIAL, map_location='cpu', weights_only=False)['model'])
    if capacity == 'wide':
        model = widen(model)
    assert sum(p.numel() for p in model.parameters()) == (26866 if capacity == 'small' else 52738)
    for p in model.critic.parameters():
        p.requires_grad_(False)
    return model.to(device)


def prepare(root, phase='diagnostic'):
    directory = root / phase / 'data'
    if (directory/'dataset.json').exists():
        return load(directory/'dataset.json')
    data = Dataset(BC_RUN/'data')
    available = [c for c in data.cases if c['role'] == 'train']
    count = 64 if phase == 'diagnostic' else 1024
    cases = [available[i] for i in np.linspace(0, len(available)-1, count, dtype=int)]
    frames = np.concatenate([np.arange(c['frame_start'], c['frame_start']+c['frames']) for c in cases])
    directory.mkdir(parents=True, exist_ok=True)
    arrays = dict(states=np.asarray(data.states[frames]),
                  actions=np.asarray(data.codes[data.offsets[frames]+data.targets[frames]]),
                  changed=np.ones(len(frames), bool))
    for key, value in arrays.items():
        np.save(directory/(key+'.npy'), value)
    records = []; start = 0
    for case in cases:
        assert sha(BC_RUN/case['path']) == case['sha256']
        records.append(dict(case, frame_start=start, teacher_T=case['frames']))
        start += case['frames']
    result = dict(root=str(BC_RUN), variant='fixed high-quality teacher trajectories',
                  cases=records, frames=len(frames), source_dataset_sha256=sha(BC_RUN/'data/dataset.json'),
                  array_sha256={k: sha(directory/(k+'.npy')) for k in arrays},
                  sampling='8 input-uniform cases x 16 uniform states, random D4', created_at=now())
    save(directory/'dataset.json', result)
    return result


def mechanism(root):
    marker = root/'mechanism/result.json'
    marker.parent.mkdir(parents=True, exist_ok=True)
    if marker.exists():
        return load(marker)
    prepare(root)
    data = Teacher(root/'diagnostic/data')
    engine = TeacherEngine(build_library(root/'environment'), data, workers=8)
    old, new = initial_model('small'), initial_model('wide')
    errors = dict(logits=0., value=0.); checks = mismatches = 0
    try:
        from train_v090_board import tensors
        for case in data.cases[:4]:
            frame = case['frame_start']+case['frames']//2
            for group in range(8):
                selection = dict(ids=np.array([case['index']], np.int32), frames=np.array([frame]), groups=np.array([group], np.int32))
                raw = data.batch(engine, selection); batch = tensors(raw, 'cpu')
                with torch.no_grad():
                    x, v = old(batch); y, w = new(batch)
                valid = batch['valid']
                errors['logits'] = max(errors['logits'], float((x[valid]-y[valid]).abs().max()))
                errors['value'] = max(errors['value'], float((v-w).abs().max()))
                if int(x.argmax(-1)) != int(y.argmax(-1)):
                    assert abs(float(x[0, x.argmax(-1)]-x[0, y.argmax(-1)])) < .002
                    mismatches += 1
                state = np.array(data.states[frame], copy=True); reference = state.copy()
                code = int(data.actions[frame]); engine.step([case['index']], state[None], np.array([code], np.uint32))
                Geometry(data.root/case['path']).apply(reference, action_line(code))
                assert np.array_equal(state, reference)
                checks += 1
        assert max(errors.values()) < .001, errors
        updates = {}
        for capacity in ('small', 'wide'):
            model = initial_model(capacity, 'mps')
            opt = torch.optim.AdamW((p for p in model.parameters() if p.requires_grad), lr=1e-4, weight_decay=1e-4)
            rng = np.random.default_rng(107099)
            critic = copy.deepcopy(model.critic.state_dict())
            def update(selection):
                raw = data.batch(engine, selection)
                logp, _ = distribution(model, raw, 'mps')
                loss = F.nll_loss(logp, torch.as_tensor(raw['target'], device='mps'))
                opt.zero_grad(set_to_none=True); loss.backward()
                grad = torch.nn.utils.clip_grad_norm_(model.parameters(), 1., error_if_nonfinite=True)
                opt.step()
                return float(loss.detach()), float(grad)
            first = update(data.choose(rng)); pending = data.choose(rng)
            path = root/'mechanism'/f'{capacity}.pt'
            checkpoint(path, dict(model={k:v.detach().cpu() for k,v in model.state_dict().items()}, optimizer=opt.state_dict(),
                                  rng=rng.bit_generator.state, pending=pending))
            second = update(pending); wanted = {k:v.detach().cpu().clone() for k,v in model.state_dict().items()}
            following = data.choose(rng)
            saved = torch.load(path, weights_only=False, map_location='cpu')
            model.load_state_dict(saved['model']); opt.load_state_dict(saved['optimizer'])
            rng.bit_generator.state = saved['rng']; replay = update(saved['pending'])
            next_selection = data.choose(rng)
            assert all(np.array_equal(following[k], next_selection[k]) for k in following)
            error = max(float((v.cpu()-wanted[k]).abs().max()) for k,v in model.state_dict().items())
            assert error < 2e-6 and abs(second[0]-replay[0]) < 2e-6
            assert all(torch.equal(v, model.critic.state_dict()[k]) for k,v in critic.items())
            assert any(not torch.equal(v, saved['model'][k]) for k,v in wanted.items())
            updates[capacity] = dict(first=first, second=second, replay=replay, weight_error=error, pending_identical=True)
            del model, opt
            torch.mps.empty_cache()
        result = dict(passed=True, checks=checks, errors=errors, tied_argmax_changes=mismatches,
                      updates=updates, initial_sha256=INITIAL_SHA, dataset_sha256=sha(data.directory/'dataset.json'), completed_at=now())
        save(marker, result)
        return result
    finally:
        engine.close()


if __name__ == '__main__':
    p=argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN)
    p.add_argument('--mode', choices=['prepare','check'], default='check'); p.add_argument('--phase', choices=['diagnostic','main'], default='diagnostic')
    a=p.parse_args(); torch.set_num_threads(2); torch.set_num_interop_threads(2)
    torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    result = prepare(a.run, a.phase) if a.mode=='prepare' else mechanism(a.run)
    print({k:v for k,v in result.items() if k not in ('cases',)}, flush=True)
