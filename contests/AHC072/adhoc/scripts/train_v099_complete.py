#!/usr/bin/env python3
"""完走した列だけで更新し、採取単位の境界から再開できるGPU学習。"""
import argparse
from datetime import datetime
import json
from pathlib import Path
import signal
import time

import numpy as np
import torch

from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from v090_data import save, sha, status, now
from v091_env import load, build_library
from v092_stream import TeacherData, OfficialQueue, excluded_inputs
from v099_complete import (ROOT, RUN, BC_RUN, REVERSE_RUN, FreshEngine, ReverseEngine, SelectedReverse,
                           configuration, start_model, collect, optimize)


def export(root, model, iteration, steps, config):
    save(root / 'training/model.json', dict(parameters={k: v.detach().cpu().tolist() for k, v in model.state_dict().items()},
          spec=model.spec, size='small', epoch=iteration, rl_iterations=iteration, environment_steps=steps,
          dataset_sha256=sha(BC_RUN / 'data/dataset.json'), initial_checkpoint_sha256=sha(root / 'initial/checkpoint.pt'),
          seed=config['seed'], method=config['method'] + ' completed trajectories', reverse_auxiliary=config['reverse_coef'] > 0))


def train(root, config, device='mps', teacher_root=REVERSE_RUN):
    directory = root / 'training'; directory.mkdir(parents=True, exist_ok=True)
    if (directory / 'result.json').exists(): return load(directory / 'result.json')
    identity = dict(config=config, initial_checkpoint_sha256=sha(root / 'initial/checkpoint.pt'),
                    teacher_dataset_sha256=sha(BC_RUN / 'data/dataset.json'), device=device,
                    selection_sha256=sha(teacher_root / 'teachers/selection.json') if config['reverse_coef'] else None)
    if (directory / 'config.json').exists(): assert load(directory / 'config.json') == identity
    else: save(directory / 'config.json', identity)
    data = TeacherData(BC_RUN / 'data'); binary = build_library(root / 'environment')
    engine = (ReverseEngine(binary, data, SelectedReverse(teacher_root), config['workers']) if config['reverse_coef']
              else FreshEngine(binary, data, config['workers']))
    data.engine = engine
    model, optimizer = start_model(root / 'initial/checkpoint.pt', device, config)
    rngs = {name: np.random.default_rng(config['seed'] + i * 1009)
            for i, name in enumerate(('action', 'transform', 'shuffle', 'auxiliary'))}
    torch.manual_seed(config['seed'])
    if device == 'mps': torch.mps.manual_seed(config['seed'])
    previous = 0.; iteration = steps = 0; history = []; saved = None
    if (directory / 'latest.pt').exists():
        saved = torch.load(directory / 'latest.pt', map_location='cpu', weights_only=False)
        assert saved['identity'] == identity
        model.load_state_dict(saved['model']); optimizer.load_state_dict(saved['optimizer'])
        for name, rng in rngs.items(): rng.bit_generator.state = saved['rngs'][name]
        torch.set_rng_state(saved['torch_rng'])
        if device == 'mps': torch.mps.set_rng_state(saved['mps_rng'])
        previous, iteration, steps, history = (saved[k] for k in ('seconds', 'iteration', 'steps', 'history'))
    blocked = excluded_inputs(); save(directory / 'excluded_sha256.json', sorted(blocked))
    queue = OfficialQueue(root, engine, blocked, config, saved['queue'] if saved else None)
    queue.wait_full(); del saved
    started = time.monotonic(); stopped = False; last_report = 0.
    def seconds(): return previous + time.monotonic() - started
    def stop(signum, frame):
        nonlocal stopped
        stopped = True
    signal.signal(signal.SIGTERM, stop); signal.signal(signal.SIGINT, stop)
    def persist():
        assert not engine.live, 'checkpoint only after every completed batch has been consumed'
        snapshot = queue.snapshot()
        try:
            checkpoint(directory / 'latest.pt', dict(identity=identity, model={k: v.detach().cpu() for k,v in model.state_dict().items()},
                       optimizer=optimizer.state_dict(), rngs={k: r.bit_generator.state for k,r in rngs.items()},
                       torch_rng=torch.get_rng_state(), mps_rng=torch.mps.get_rng_state() if device == 'mps' else None,
                       queue=snapshot, seconds=seconds(), iteration=iteration, steps=steps, history=history))
            (directory / 'metrics.jsonl').write_text(''.join(json.dumps(row, allow_nan=False)+'\n' for row in history))
        finally: queue.resume()
    def report(phase, part, finished=None):
        nonlocal last_report
        if time.monotonic() - last_report < 30: return
        status(directory, phase, iteration=iteration, steps=steps, part=part, finished=finished,
               seconds=seconds(), remaining_seconds=max(0., config['seconds']-seconds()),
               queue_ready=len(queue.ready), active_geometries=len(engine.live), bc_cache=len(engine.handles), device=device)
        last_report = time.monotonic()
    reason = 'iteration_budget'
    try:
        if iteration == 0: persist()
        while iteration < config['iterations']:
            if stopped: persist(); raise InterruptedError('intentional stop at completed-batch checkpoint')
            if seconds() >= config['seconds']: reason = 'time_budget'; break
            if datetime.now().astimezone() >= datetime.fromisoformat(config['final_deadline']): reason = 'deadline'; break
            queue.set_update_phase(False); tick = time.monotonic()
            buffer, finished, final_states, items = collect(model, engine, queue, rngs['action'], rngs['transform'], device, config,
                                                          lambda n, done: report('collecting_complete_episodes', n, done))
            rollout_seconds = time.monotonic()-tick; queue.set_update_phase(True); tick = time.monotonic()
            metrics = optimize(model, optimizer, engine, data, buffer, rngs['shuffle'], rngs['auxiliary'], device, config,
                               min(1., seconds()/config['seconds']), lambda n: report('updating_complete_episodes', n))
            update_seconds = time.monotonic()-tick; iteration += 1; steps += len(buffer['ids'])
            complete = [r for r in finished if r['E'] == 0]
            lengths = np.array([r['T'] for r in finished]).reshape(config['boards'], config['repetitions'])
            row = dict(iteration=iteration, steps=steps, seconds=seconds(), rollout_seconds=rollout_seconds,
                       update_seconds=update_seconds, episodes=len(finished), successes=len(complete),
                       mean_T_completed=float(np.mean([r['T'] for r in complete])) if complete else None,
                       mean_E_finished=float(np.mean([r['E'] for r in finished])), mean_group_spread=float(np.mean(np.ptp(lengths, axis=1))),
                       queue_wait_seconds=queue.wait_seconds, queue_ready=len(queue.ready), **metrics)
            save(directory / 'episodes' / f'{iteration:04d}.json', dict(rows=finished,
                 inputs=[{k:v for k,v in item.items() if k!='pointer'} for item in items]))
            history.append(row); engine.release_except([]); engine.buffers.clear()
            assert len(engine.handles) <= config['bc_cache_capacity']
            if config['reverse_batch']: assert len(engine.reverse_handles) <= 128
            del buffer, final_states, items, finished
            persist(); status(directory, 'iteration_completed', **row, remaining_seconds=max(0., config['seconds']-seconds()))
            print(json.dumps(row, allow_nan=False), flush=True)
        assert iteration > 0
        persist(); export(root, model, iteration, steps, config)
        result = dict(iterations=iteration, environment_steps=steps, seconds=seconds(), reason=reason,
                      episodes=sum(r['episodes'] for r in history), successes=sum(r['successes'] for r in history),
                      history=history, checkpoint_sha256=sha(directory / 'latest.pt'),
                      model_sha256=sha(directory / 'model.json'), completed_at=now())
        save(directory / 'result.json', result); status(directory, 'training_completed', iterations=iteration, steps=steps, reason=reason)
        return result
    finally: queue.close(); engine.close()


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, required=True)
    p.add_argument('--config', type=Path, required=True); p.add_argument('--device', choices=('cpu','mps'), default='mps')
    p.add_argument('--gpu-state', type=Path, required=True); p.add_argument('--teachers', type=Path, default=REVERSE_RUN)
    a = p.parse_args(); torch.set_num_threads(2); torch.set_num_interop_threads(2)
    if a.device == 'mps': torch.mps.set_per_process_memory_fraction(min(1., 16e9 / torch.mps.recommended_max_memory()))
    reporter = GPUReport(a.gpu_state) if a.device == 'mps' else None
    try: train(a.run.resolve(), load(a.config), a.device, a.teachers.resolve())
    finally:
        if reporter: reporter.close()
