#!/usr/bin/env python3
"""保存した重みとoptimizer、乱数と未使用入力を引き継いで追加学習する。"""
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
from v099_complete import BC_RUN, FreshEngine, start_model
from v102_learning import collect, optimize
from v104_extended import restore, inherited_state, blocked_inputs, POINTS, INITIAL_SHA
import shutil


def export(root, model, iteration, steps, config):
    save(root / 'training/model.json', dict(parameters={k: v.detach().cpu().tolist() for k, v in model.state_dict().items()},
          spec=model.spec, size='small', epoch=iteration, rl_iterations=iteration, environment_steps=steps,
          dataset_sha256=sha(BC_RUN / 'data/dataset.json'), initial_checkpoint_sha256=INITIAL_SHA,
          seed=config['seed'], method=config['method'] + ' completed trajectories', reverse_auxiliary=config['reverse_coef'] > 0))


def train(root, config, device='mps'):
    directory = root / 'training'; directory.mkdir(parents=True, exist_ok=True)
    if (directory / 'result.json').exists(): return load(directory / 'result.json')
    identity = dict(config=config, initial_checkpoint_sha256=sha(root / 'initial/checkpoint.pt'),
                    teacher_dataset_sha256=sha(BC_RUN / 'data/dataset.json'), device=device)
    if (directory / 'config.json').exists(): assert load(directory / 'config.json') == identity
    else: save(directory / 'config.json', identity)
    data = TeacherData(BC_RUN / 'data'); binary = build_library(root / 'environment')
    engine = FreshEngine(binary, data, config['workers'])
    data.engine = engine
    model, optimizer = start_model(root / 'initial/checkpoint.pt', device, config)
    rngs = {name: np.random.default_rng(config['seed'] + i * 1009)
            for i, name in enumerate(('action', 'transform', 'shuffle', 'auxiliary'))}
    torch.manual_seed(config['seed'])
    if device == 'mps': torch.mps.manual_seed(config['seed'])
    previous = 0.; iteration = steps = 0; history = []
    resumed = (directory / 'latest.pt').exists()
    saved = torch.load(directory / 'latest.pt', map_location='cpu', weights_only=False) if resumed else inherited_state(root)
    if resumed: assert saved['identity'] == identity
    restore(model, optimizer, rngs, saved, device)
    if resumed:
        previous, iteration, steps, history = (saved[k] for k in ('seconds', 'iteration', 'steps', 'history'))
    else:
        assert all(torch.equal(v.cpu(), saved['model'][k]) for k,v in model.state_dict().items())
        save(directory / 'inheritance.json', dict(initial_sha256=sha(root / 'initial/checkpoint.pt'),
             parent_iteration=saved['iteration'], parent_steps=saved['steps'],
             optimizer_steps=sorted({int(v['step']) for v in optimizer.state_dict()['state'].values()}),
             consume_ticket=saved['queue']['consume_ticket'], next_ticket=saved['queue']['next_ticket'],
             rngs_restored=True, model_identical=True))
    blocked = blocked_inputs(root,iteration); save(directory / 'excluded_sha256.json', sorted(blocked))
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
    def snapshot_point():
        if iteration not in POINTS: return
        point = root / 'points' / f'{iteration:04d}'
        marker = point / 'snapshot.json'
        if marker.exists():
            prior = load(marker)
            assert prior['checkpoint_sha256'] == sha(point / 'training/latest.pt')
            assert prior['model_sha256'] == sha(point / 'training/model.json')
            return
        (point / 'training').mkdir(parents=True, exist_ok=True)
        shutil.copy2(directory / 'latest.pt', point / 'training/latest.pt')
        export(point, model, iteration, steps, config)
        save(marker, dict(iteration=iteration, checkpoint_sha256=sha(point / 'training/latest.pt'),
             model_sha256=sha(point / 'training/model.json'), completed_at=now()))
    reason = 'iteration_budget'
    try:
        if iteration == 0: persist()
        # 固定点の保存途中で終了しても、次の更新より先に同じ重みを保存し直す。
        snapshot_point()
        while iteration < config['iterations']:
            if stopped: persist(); raise InterruptedError('intentional stop at completed-batch checkpoint')
            if seconds() >= config['seconds']: reason = 'time_budget'; break
            if datetime.now().astimezone() >= datetime.fromisoformat(config['final_deadline']): reason = 'deadline'; break
            queue.set_update_phase(False); tick = time.monotonic()
            buffer, finished, final_states, items = collect(model, engine, queue, rngs['action'], rngs['transform'], device, config,
                                                          lambda n, done: report('collecting_complete_episodes', n, done))
            rollout_seconds = time.monotonic()-tick; queue.set_update_phase(True); tick = time.monotonic()
            metrics = optimize(model, optimizer, engine, data, buffer, rngs['shuffle'], rngs['auxiliary'], device, config,
                               iteration / max(1, config['iterations']-1), lambda n: report('updating_complete_episodes', n))
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
            history.append(row)
            # 待ち行列と独立に、消費済み盤面の内容ハッシュだけを保持する。
            blocked.update(item['sha256'] for item in items)
            engine.release_except([]); engine.buffers.clear()
            assert len(engine.handles) <= config['bc_cache_capacity']
            del buffer, final_states, items, finished
            persist()
            snapshot_point()
            status(directory, 'iteration_completed', **row, remaining_seconds=max(0., config['seconds']-seconds()))
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
    p.add_argument('--gpu-state', type=Path, required=True)
    a = p.parse_args(); torch.set_num_threads(2); torch.set_num_interop_threads(2)
    if a.device == 'mps': torch.mps.set_per_process_memory_fraction(min(1., 16e9 / torch.mps.recommended_max_memory()))
    reporter = GPUReport(a.gpu_state) if a.device == 'mps' else None
    try: train(a.run.resolve(), load(a.config), a.device)
    finally:
        if reporter: reporter.close()
