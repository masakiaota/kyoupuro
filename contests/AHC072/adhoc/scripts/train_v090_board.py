#!/usr/bin/env python3
"""固定120周で実盤面CNNを学習する。本実行の容量は小型に固定。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
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
from train_v089_board import tensors, update
from v090_data import Dataset, RUN, SEED, BATCH, EPOCHS, MODEL_SPECS, save, sha, status, now


class Block(nn.Module):
    def __init__(self, width):
        super().__init__()
        self.depth = nn.Conv2d(width, width, 3, padding=1, groups=width)
        self.point = nn.Conv2d(width, width, 1)

    def forward(self, x, floor):
        return F.relu(x + self.point(F.relu(self.depth(x)))) * floor


class Model(nn.Module):
    def __init__(self, spec):
        super().__init__()
        self.spec = dict(spec); C, H = spec['width'], spec['hidden']
        self.input = nn.Conv2d(40, C, 1)
        self.blocks = nn.ModuleList([Block(C) for _ in range(spec['depth'])])
        self.actor = nn.Sequential(nn.Linear(3 * C + 16, H), nn.ReLU(), nn.Linear(H, 1))
        self.critic = nn.Sequential(nn.Linear(C, H), nn.ReLU(), nn.Linear(H, 1))

    def forward(self, batch):
        C, H = self.spec['width'], self.spec['hidden']
        floor = batch['x'][:, :1]
        h = F.relu(self.input(batch['x'])) * floor
        for block in self.blocks: h = block(h, floor)
        pooled = h.sum((2, 3)) / floor.sum((2, 3))
        cells = h.flatten(2).transpose(1, 2)
        weight, bias = self.actor[0].weight, self.actor[0].bias
        source = F.linear(cells, weight[:, :C]); dest = F.linear(cells, weight[:, C:2 * C])
        hidden = torch.gather(source, 1, batch['src'][..., None].expand(-1, -1, H))
        hidden = hidden + torch.gather(dest, 1, batch['dst'][..., None].expand(-1, -1, H))
        hidden = hidden + F.linear(pooled, weight[:, 2 * C:3 * C], bias)[:, None]
        hidden = hidden + F.linear(batch['features'], weight[:, 3 * C:])
        logits = self.actor[2](F.relu(hidden)).squeeze(-1).masked_fill(~batch['valid'], -1e9)
        value = F.softplus(self.critic(pooled).squeeze(-1)) * 100.
        return logits, value


def model_new(size, device):
    torch.manual_seed(SEED)
    if device == 'mps': torch.mps.manual_seed(SEED)
    return Model(MODEL_SPECS[size]).to(device)


def prefetched_batches(data, order, offset, epoch):
    """入力順を変えず、CPUの次の1batchの準備をGPU更新と重ねる。"""
    if offset >= len(order): return
    with ThreadPoolExecutor(max_workers=1) as pool:
        ids = order[offset:offset + BATCH]
        future = pool.submit(data.batch, ids, epoch)
        while offset < len(order):
            raw = future.result()
            following = offset + len(ids)
            next_ids = order[following:following + BATCH]
            if len(next_ids): future = pool.submit(data.batch, next_ids, epoch)
            yield ids, raw
            offset, ids = following, next_ids


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


def export(model, path, data_dir, epoch, size):
    save(path, {'parameters': {k: v.detach().cpu().tolist() for k, v in model.state_dict().items()},
                'spec': model.spec, 'size': size, 'dataset_sha256': sha(data_dir / 'dataset.json'),
                'epoch': epoch, 'seed': SEED})


def mechanism(root, size, device, steps=300, resume=False):
    run = root / 'models' / size; run.mkdir(parents=True, exist_ok=True)
    data = Dataset(root / 'data'); model = model_new(size, device)
    # すべて学習入力。開始・中盤・終盤の異なる合法手数を含める。
    cases = [c for c in data.cases if c['role'] == 'train'][:4]
    ids = np.array([c['frame_start'] + c['frames'] // 2 for c in cases])
    optimizer = torch.optim.AdamW(model.parameters(), lr=.001, weight_decay=.0001)
    batch = tensors(data.batch(ids), device)
    before = evaluate(model, data, ids, device); started = time.monotonic(); first_step = 0
    if resume:
        saved = torch.load(run / 'mechanism.pt', map_location='cpu', weights_only=False)
        model.load_state_dict(saved['model']); optimizer.load_state_dict(saved['optimizer'])
        first_step = saved['step']; assert first_step < steps
        previous = json.loads((run / 'learning_check.json').read_text())
        save(run / f'learning_check_step{first_step}.json', previous)
        before = previous['before']
        del saved
    for _ in range(first_step, steps): update(model, optimizer, batch)
    after = evaluate(model, data, ids, device)
    checkpoint(run / 'mechanism.pt', {'model': {k: v.detach().cpu() for k, v in model.state_dict().items()},
                                     'optimizer': optimizer.state_dict(), 'step': steps})
    restored = torch.load(run / 'mechanism.pt', map_location='cpu', weights_only=False)
    second = model_new(size, device); second.load_state_dict(restored['model'])
    other_opt = torch.optim.AdamW(second.parameters(), lr=.001, weight_decay=.0001)
    other_opt.load_state_dict(restored['optimizer'])
    update(model, optimizer, batch); update(second, other_opt, batch)
    resume_error = max((v - second.state_dict()[key]).abs().max().item() for key, v in model.state_dict().items())
    export(model, run / 'mechanism_model.json', root / 'data', -1, size)
    del model, second, optimizer, other_opt, batch, restored
    if device == 'mps': torch.mps.empty_cache()
    bench = model_new(size, device); opt = torch.optim.AdamW(bench.parameters(), lr=.001)
    order = np.random.default_rng(SEED).permutation(data.splits['train'])
    times = []
    for step in range(32):
        tick = time.monotonic()
        update(bench, opt, tensors(data.batch(order[step * BATCH:(step + 1) * BATCH], 1), device))
        if device == 'mps': torch.mps.synchronize()
        if step >= 8: times.append(time.monotonic() - tick)
    seconds_per_batch = float(np.mean(times))
    result = {'before': before, 'after': after, 'frame_ids': ids.tolist(), 'resume_max_error': resume_error,
              'mechanism_steps': steps, 'resumed_from_step': first_step,
              'parameters': sum(p.numel() for p in bench.parameters()), 'spec': bench.spec,
              'seconds': time.monotonic() - started, 'seconds_per_batch': seconds_per_batch,
              'estimated_training_seconds': math.ceil(len(order) / BATCH) * EPOCHS * seconds_per_batch,
              'passed': after['teacher_top1'] == 1. and after['remaining_mae'] < 2. and resume_error < 1e-5}
    save(run / 'learning_check.json', result); print(json.dumps(result), flush=True)
    assert result['passed'], f'{size}: four-frame learning/resume check failed'


def train(root, size, device, seconds):
    run = root / 'models' / size; run.mkdir(parents=True, exist_ok=True)
    data_dir = root / 'data'; data = Dataset(data_dir); fingerprint = sha(data_dir / 'dataset.json')
    model = model_new(size, device); optimizer = torch.optim.AdamW(model.parameters(), lr=.001, weight_decay=.0001)
    epoch = offset = steps = 0; history = []
    if (run / 'latest.pt').exists():
        saved = torch.load(run / 'latest.pt', map_location='cpu', weights_only=False)
        assert saved['dataset_sha256'] == fingerprint and saved['spec'] == model.spec
        model.load_state_dict(saved['model']); optimizer.load_state_dict(saved['optimizer'])
        epoch, offset, steps, history = (saved[k] for k in ('epoch', 'offset', 'steps', 'history'))
        torch.set_rng_state(saved['torch_rng'])
        if device == 'mps': torch.mps.set_rng_state(saved['mps_rng'])
    save(run / 'config.json', {'size': size, 'spec': model.spec, 'seed': SEED, 'batch': BATCH,
         'epochs': EPOCHS, 'initial_lr': .001, 'final_lr': .0001, 'weight_decay': .0001,
         'loss': 'input-equal CE + SmoothL1(remaining/100)', 'dataset_sha256': fingerprint,
         'parameters': sum(p.numel() for p in model.parameters()), 'train_curve_stride': 17,
         'prefetch_batches': 1, 'device': device, 'parameter_device': str(next(model.parameters()).device)})
    stopped = False
    def stop(signum, frame):
        nonlocal stopped
        stopped = True
    signal.signal(signal.SIGTERM, stop); signal.signal(signal.SIGINT, stop)
    def persist():
        checkpoint(run / 'latest.pt', {'model': {k: v.detach().cpu() for k, v in model.state_dict().items()},
                   'optimizer': optimizer.state_dict(), 'spec': model.spec, 'epoch': epoch, 'offset': offset,
                   'steps': steps, 'history': history, 'dataset_sha256': fingerprint,
                   'torch_rng': torch.get_rng_state(), 'mps_rng': torch.mps.get_rng_state() if device == 'mps' else None})
    started = last_report = time.monotonic(); starting_steps = steps
    total_steps = math.ceil(len(data.splits['train']) / BATCH) * EPOCHS
    while epoch < EPOCHS:
        order = np.random.default_rng(SEED + epoch).permutation(data.splits['train'])
        for ids, raw in prefetched_batches(data, order, offset, epoch + 1):
            if stopped or time.monotonic() - started >= seconds:
                persist(); status(run, 'training_interrupted', epoch=epoch, offset=offset, steps=steps)
                raise TimeoutError('training time limit or signal')
            progress = steps / max(1, total_steps - 1)
            lr = .0001 + .5 * (.001 - .0001) * (1 + math.cos(math.pi * progress))
            optimizer.param_groups[0]['lr'] = lr
            loss = update(model, optimizer, tensors(raw, device))
            offset += len(ids); steps += 1
            if steps % 512 == 0: persist()
            if time.monotonic() - last_report >= 30:
                rate = (steps - starting_steps) / (time.monotonic() - started)
                status(run, 'training', size=size, epoch=epoch + 1, steps=steps, total_steps=total_steps,
                       loss=float(loss), learning_rate=lr, updates_per_second=rate,
                       remaining_seconds=(total_steps - steps) / rate)
                last_report = time.monotonic()
        epoch += 1; offset = 0
        if epoch in (1, 10, 40, 80, 120):
            row = {'epoch': epoch, 'steps': steps,
                   'train_fixed': evaluate(model, data, data.splits['train'][::17], device),
                   'validation': evaluate(model, data, data.splits['validation'], device)}
            if epoch == EPOCHS: row['train_all'] = evaluate(model, data, data.splits['train'], device)
            history.append(row); save(run / f'metrics_epoch{epoch:03d}.json', row)
            export(model, run / f'model_epoch{epoch:03d}.json', data_dir, epoch, size)
        persist()
    export(model, run / 'model.json', data_dir, EPOCHS, size)
    save(run / 'result.json', {'final': history[-1], 'history': history, 'checkpoint_sha256': sha(run / 'latest.pt'),
                             'completed_at': now(), 'seconds': time.monotonic() - started})
    status(run, 'training_completed', epoch=epoch, steps=steps, size=size)


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN)
    p.add_argument('--size', choices=list(MODEL_SPECS), required=True)
    p.add_argument('--device', choices=['cpu', 'mps'], default='mps'); p.add_argument('--mechanism', action='store_true')
    p.add_argument('--mechanism-steps', type=int, default=300); p.add_argument('--mechanism-resume', action='store_true')
    p.add_argument('--seconds', type=int, default=172000); args = p.parse_args()
    torch.set_num_threads(2); torch.set_num_interop_threads(2)
    if args.device == 'mps': torch.mps.set_per_process_memory_fraction(min(1., 16e9 / torch.mps.recommended_max_memory()))
    gpu = args.run / 'gpu_state.json'; reporter = GPUReport(gpu) if args.device == 'mps' else None
    try:
        if args.mechanism: mechanism(args.run.resolve(), args.size, args.device, args.mechanism_steps, args.mechanism_resume)
        else: train(args.run.resolve(), args.size, args.device, args.seconds)
    finally:
        if reporter: reporter.close()
