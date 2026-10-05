#!/usr/bin/env python3
"""実操作の方策と、保存完成列の残り手数を固定40周学習する。"""
import argparse
import json
import math
from pathlib import Path
import signal
import time

import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from v089_data import Dataset, RUN, SEED, BATCH, EPOCHS, save, sha, status, now


class Block(nn.Module):
    def __init__(self):
        super().__init__()
        self.depth = nn.Conv2d(24, 24, 3, padding=1, groups=24)
        self.point = nn.Conv2d(24, 24, 1)

    def forward(self, x, floor):
        return F.relu(x + self.point(F.relu(self.depth(x)))) * floor


class Model(nn.Module):
    def __init__(self):
        super().__init__()
        self.input = nn.Conv2d(40, 24, 1)
        self.blocks = nn.ModuleList([Block() for _ in range(3)])
        self.actor = nn.Sequential(nn.Linear(88, 32), nn.ReLU(), nn.Linear(32, 1))
        self.critic = nn.Sequential(nn.Linear(24, 32), nn.ReLU(), nn.Linear(32, 1))

    def forward(self, batch):
        floor = batch['x'][:, :1]
        h = F.relu(self.input(batch['x'])) * floor
        for block in self.blocks: h = block(h, floor)
        pooled = h.sum((2, 3)) / floor.sum((2, 3))
        cells = h.flatten(2).transpose(1, 2)
        # 同じ出発点・着地点の行列積を合法手ごとに繰り返さない。
        weight, bias = self.actor[0].weight, self.actor[0].bias
        source = F.linear(cells, weight[:, :24])
        dest = F.linear(cells, weight[:, 24:48])
        hidden = torch.gather(source, 1, batch['src'][..., None].expand(-1, -1, 32))
        hidden = hidden + torch.gather(dest, 1, batch['dst'][..., None].expand(-1, -1, 32))
        hidden = hidden + F.linear(pooled, weight[:, 48:72], bias)[:, None]
        hidden = hidden + F.linear(batch['features'], weight[:, 72:])
        logits = self.actor[2](F.relu(hidden)).squeeze(-1)
        logits = logits.masked_fill(~batch['valid'], -1e9)
        value = F.softplus(self.critic(pooled).squeeze(-1)) * 100.
        return logits, value


def model_new(device):
    torch.manual_seed(SEED)
    if device == 'mps': torch.mps.manual_seed(SEED)
    return Model().to(device)


def tensors(raw, device):
    return {key: torch.from_numpy(np.ascontiguousarray(value)).to(device) for key, value in raw.items()}


def update(model, optimizer, batch):
    model.train(); logits, value = model(batch)
    policy = F.cross_entropy(logits, batch['target'], reduction='none')
    critic = F.smooth_l1_loss(value / 100., batch['value'] / 100., reduction='none')
    loss = ((policy + critic) * batch['weight']).mean()
    optimizer.zero_grad(set_to_none=True); loss.backward()
    nn.utils.clip_grad_norm_(model.parameters(), 5., error_if_nonfinite=True)
    optimizer.step()
    return loss.detach()


@torch.no_grad()
def evaluate(model, data, ids, device):
    model.eval(); count = 0; sums = np.zeros(4, np.float64)
    for start in range(0, len(ids), BATCH):
        batch = tensors(data.batch(ids[start:start + BATCH]), device)
        logits, value = model(batch)
        rows = torch.stack([F.cross_entropy(logits, batch['target'], reduction='sum'),
                            F.smooth_l1_loss(value / 100., batch['value'] / 100., reduction='sum'),
                            (logits.argmax(-1) == batch['target']).sum(),
                            (value - batch['value']).abs().sum()])
        sums += rows.cpu().numpy(); count += len(batch['target'])
    assert np.isfinite(sums).all()
    return dict(zip(('policy_ce', 'value_huber_100', 'teacher_top1', 'remaining_mae'), (sums / count).tolist()))


def export(model, path, run, epoch):
    save(path, {'parameters': {k: v.detach().cpu().tolist() for k, v in model.state_dict().items()},
                'dataset_sha256': sha(run / 'dataset.json'), 'epoch': epoch, 'seed': SEED})


def mechanism(run, device):
    data = Dataset(run); model = model_new(device)
    cases = [c for c in data.cases if c['role'] == 'train'][:4]
    ids = np.array([c['frame_start'] + c['frames'] // 2 for c in cases])
    optimizer = torch.optim.AdamW(model.parameters(), lr=.005, weight_decay=.0001)
    batch = tensors(data.batch(ids), device)
    before = evaluate(model, data, ids, device); started = time.monotonic()
    for _ in range(300): update(model, optimizer, batch)
    after = evaluate(model, data, ids, device)
    stored = {'model': {k: v.detach().cpu() for k, v in model.state_dict().items()},
              'optimizer': optimizer.state_dict(), 'step': 300}
    checkpoint(run / 'mechanism.pt', stored)
    restored = torch.load(run / 'mechanism.pt', map_location='cpu', weights_only=False)
    second = model_new(device); second.load_state_dict(restored['model'])
    second_opt = torch.optim.AdamW(second.parameters(), lr=.005, weight_decay=.0001)
    second_opt.load_state_dict(restored['optimizer'])
    # 保存直後から同じ1更新を行い、重みとAdam状態の再開を検証する。
    update(model, optimizer, batch); update(second, second_opt, batch)
    resume_error = max((v - second.state_dict()[key]).abs().max().item() for key, v in model.state_dict().items())
    export(model, run / 'mechanism_model.json', run, -1)
    # 実バッチの学習速度。機構用の更新は本学習へ引き継がない。
    bench = model_new(device); opt = torch.optim.AdamW(bench.parameters(), lr=.001)
    started_bench = time.monotonic()
    order = np.random.default_rng(SEED).permutation(data.splits['train'])
    for s in range(24): update(bench, opt, tensors(data.batch(order[s * BATCH:(s + 1) * BATCH], 1), device))
    if device == 'mps': torch.mps.synchronize()
    seconds_per_batch = (time.monotonic() - started_bench) / 24
    result = {'before': before, 'after': after, 'frame_ids': ids.tolist(), 'resume_max_error': resume_error,
              'seconds': time.monotonic() - started, 'seconds_per_batch': seconds_per_batch,
              'estimated_training_seconds': math.ceil(len(order) / BATCH) * EPOCHS * seconds_per_batch,
              'passed': after['teacher_top1'] == 1. and after['remaining_mae'] < 2. and resume_error < 1e-5}
    save(run / 'learning_check.json', result); print(json.dumps(result), flush=True)
    assert result['passed'], 'four-frame learning/resume check failed'


def train(run, device, seconds):
    data = Dataset(run); fingerprint = sha(run / 'dataset.json')
    model = model_new(device); optimizer = torch.optim.AdamW(model.parameters(), lr=.001, weight_decay=.0001)
    epoch = offset = steps = 0; history = []
    if (run / 'latest.pt').exists():
        saved = torch.load(run / 'latest.pt', map_location='cpu', weights_only=False)
        assert saved['dataset_sha256'] == fingerprint
        model.load_state_dict(saved['model']); optimizer.load_state_dict(saved['optimizer'])
        epoch, offset, steps, history = (saved[k] for k in ('epoch', 'offset', 'steps', 'history'))
        torch.set_rng_state(saved['torch_rng'])
        if device == 'mps': torch.mps.set_rng_state(saved['mps_rng'])
    save(run / 'config.json', {'seed': SEED, 'batch': BATCH, 'epochs': EPOCHS, 'initial_lr': .001, 'final_lr': .0001,
                              'weight_decay': .0001, 'loss': 'input-equal CE + SmoothL1(remaining/100)',
                              'dataset_sha256': fingerprint, 'parameters': sum(p.numel() for p in model.parameters())})
    stopped = False
    def stop(signum, frame):
        nonlocal stopped
        stopped = True
    signal.signal(signal.SIGTERM, stop); signal.signal(signal.SIGINT, stop)
    def persist():
        checkpoint(run / 'latest.pt', {'model': {k: v.detach().cpu() for k, v in model.state_dict().items()},
                   'optimizer': optimizer.state_dict(), 'epoch': epoch, 'offset': offset, 'steps': steps,
                   'history': history, 'dataset_sha256': fingerprint, 'torch_rng': torch.get_rng_state(),
                   'mps_rng': torch.mps.get_rng_state() if device == 'mps' else None})
    started = last_report = time.monotonic(); starting_steps = steps
    total_steps = math.ceil(len(data.splits['train']) / BATCH) * EPOCHS
    while epoch < EPOCHS:
        order = np.random.default_rng(SEED + epoch).permutation(data.splits['train'])
        while offset < len(order):
            if stopped or time.monotonic() - started >= seconds:
                persist(); status(run, 'training_interrupted', epoch=epoch, offset=offset, steps=steps)
                raise TimeoutError('training time limit or signal')
            progress = steps / max(1, total_steps - 1)
            lr = .0001 + .5 * (.001 - .0001) * (1 + math.cos(math.pi * progress))
            optimizer.param_groups[0]['lr'] = lr
            ids = order[offset:offset + BATCH]
            loss = update(model, optimizer, tensors(data.batch(ids, epoch + 1), device))
            offset += len(ids); steps += 1
            if steps % 512 == 0: persist()
            if time.monotonic() - last_report >= 30:
                rate = (steps - starting_steps) / (time.monotonic() - started)
                status(run, 'training', epoch=epoch + 1, steps=steps, total_steps=total_steps,
                       loss=float(loss), learning_rate=lr, updates_per_second=rate,
                       remaining_seconds=(total_steps - steps) / rate)
                last_report = time.monotonic()
        epoch += 1; offset = 0
        if epoch in (1, 10, 20, 40):
            ids = data.splits['train'] if epoch == EPOCHS else data.splits['train'][::41]
            row = {'epoch': epoch, 'steps': steps, 'train': evaluate(model, data, ids, device),
                   'validation': evaluate(model, data, data.splits['validation'], device)}
            history.append(row); save(run / f'metrics_epoch{epoch:03d}.json', row)
        persist()
    export(model, run / 'model.json', run, EPOCHS)
    save(run / 'result.json', {'final': history[-1], 'history': history, 'checkpoint_sha256': sha(run / 'latest.pt'),
                             'completed_at': now(), 'seconds': time.monotonic() - started})
    status(run, 'training_completed', epoch=epoch, steps=steps)


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN)
    p.add_argument('--device', choices=['cpu', 'mps'], default='mps'); p.add_argument('--mechanism', action='store_true')
    p.add_argument('--seconds', type=int, default=14000); args = p.parse_args()
    torch.set_num_threads(2); torch.set_num_interop_threads(2)
    if args.device == 'mps': torch.mps.set_per_process_memory_fraction(min(1., 16e9 / torch.mps.recommended_max_memory()))
    reporter = GPUReport(args.run / 'gpu_state.json') if args.device == 'mps' else None
    try:
        if args.mechanism: mechanism(args.run.resolve(), args.device)
        else: train(args.run.resolve(), args.device, args.seconds)
    finally:
        if reporter: reporter.close()
