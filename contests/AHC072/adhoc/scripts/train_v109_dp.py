#!/usr/bin/env python3
"""実測の配送費用で学習し、採取境界で全状態を保存する。"""
import argparse
from datetime import datetime
import json
from pathlib import Path
import signal
import time

import numpy as np
import torch

from check_v098_complete import action_line
from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from v089_data import Geometry, remaining
from v090_data import save, sha, status, now
from v091_env import load, build_library
from v092_stream import TeacherData, FreshEngine, OfficialQueue, describe, digest, normalized
from v099_complete import start_model, measured_targets
from v102_learning import optimize
from v104_extended import restore
from v109_dp import (BC_RUN, INITIAL_SHA, configuration, blocked_inputs, build_completion,
                     CompletionPool, collect, make_rngs, summary)


def engine_for(root, config):
    data = TeacherData(BC_RUN/'data')
    engine = FreshEngine(build_library(root/'environment'), data, config['workers']); data.engine = engine
    return data, engine


def model_state(model, optimizer, rngs):
    return dict(model={k: v.detach().cpu().clone() for k, v in model.state_dict().items()},
                optimizer=optimizer.state_dict(), rngs={k: r.bit_generator.state for k, r in rngs.items()},
                torch_rng=torch.get_rng_state(), mps_rng=torch.mps.get_rng_state())


def serialized(items): return [{k: v for k, v in item.items() if k != 'pointer'} for item in items]


def mechanism(root):
    directory = root/'mechanism'; directory.mkdir(parents=True, exist_ok=True)
    if (directory/'result.json').exists(): return load(directory/'result.json')
    started = time.monotonic(); config = dict(configuration(), boards=8, environments=32, epochs=1,
        seed_base=1090900000000, queue_capacity=16)
    data, engine = engine_for(root, config); queue = None
    pool = CompletionPool(build_completion(root), config['completion_workers'])
    model, optimizer = start_model(root/'initial/checkpoint.pt', 'mps', config)
    initial = {k: v.detach().cpu().clone() for k, v in model.state_dict().items()}
    rngs = make_rngs(config['seed']); torch.manual_seed(config['seed']); torch.mps.manual_seed(config['seed'])
    try:
        queue = OfficialQueue(directory, engine, blocked_inputs(root), config)
        queue.wait_full(); queue.set_update_phase(False)
        buffer, rows, states, items, timing = collect(model, engine, queue, pool, rngs, 'mps', config)
        save(directory/'episodes/0001.json', dict(rows=rows, inputs=serialized(items)))
        for i, item in enumerate(items):
            path = directory/f'{i}.txt'; path.write_text(item['text']); geo = Geometry(path)
            for slot in range(i*4, i*4+4):
                state = geo.initial.copy(); row = rows[slot]
                for code in row['actions']: geo.apply(state, action_line(code))
                assert np.array_equal(state, states[slot]) and remaining(state) == row['prefix_E']
                for code in row['tail']: geo.apply(state, action_line(code))
                assert remaining(state) == row['E'] and row['T'] == len(row['actions'])+len(row['tail'])
                assert np.isclose(row['cost'], row['T']/100.+(30.+row['E']/256. if row['E'] else 0.))
        lengths = np.array([r['T'] for r in rows]); left = np.array([r['E'] for r in rows])
        one = measured_targets(lengths, left, buffer['episode'], buffer['time'], buffer['value'], 4, 'group')
        two = measured_targets(lengths, left, buffer['episode'], buffer['time'], buffer['value']+10000, 4, 'group')
        assert np.array_equal(one[0], two[0]) and np.array_equal(one[1], two[1])
        for row in rows:
            others = [r['cost'] for r in rows if r['case'] == row['case'] and r['episode'] != row['episode']]
            assert np.isclose(row['group_advantage'], np.mean(others)-row['cost'])
        assert len(buffer['ids']) == sum(r['prefix_T'] for r in rows)
        queue.set_update_phase(True); queue.wait_full()
        snapshot = queue.snapshot(); expected = snapshot['ready'][0]
        checkpoint(directory/'before_update.pt', dict(model_state(model, optimizer, rngs), queue=snapshot))
        queue.resume()
        metrics = optimize(model, optimizer, engine, data, buffer, rngs['shuffle'], rngs['auxiliary'], 'mps', config, 0.)
        assert metrics['value_loss'] == metrics['bc_ce'] == metrics['bc_samples'] == 0
        restored, other = start_model(root/'initial/checkpoint.pt', 'mps', config)
        saved = torch.load(directory/'before_update.pt', map_location='cpu', weights_only=False)
        restore(restored, other, rngs, saved, 'mps')
        assert all(rngs[k].bit_generator.state == saved['rngs'][k] for k in rngs)
        again = optimize(restored, other, engine, data, buffer, rngs['shuffle'], rngs['auxiliary'], 'mps', config, 0.)
        error = max(float((v-restored.state_dict()[k]).abs().max().cpu()) for k, v in model.state_dict().items())
        assert error < 2e-6 and again['updates'] == metrics['updates']
        assert all(torch.equal(v.cpu(), initial[k]) for k, v in model.state_dict().items() if k.startswith('critic.'))
        assert any(not torch.equal(v.cpu(), initial[k]) for k, v in model.state_dict().items())
        # 非空のAdamW状態も保存・復元で全項目一致することを確認する。
        checkpoint(directory/'after_update.pt', model_state(model, optimizer, rngs))
        after = torch.load(directory/'after_update.pt', map_location='cpu', weights_only=False)
        restore(restored, other, rngs, after, 'mps')
        for key, values in after['optimizer']['state'].items():
            for field, value in values.items(): assert torch.equal(other.state_dict()['state'][key][field].cpu(), value)
        queue.close(); queue = None
        restored_queue = OfficialQueue(directory/'restored_queue', engine, blocked_inputs(root), config, snapshot)
        try:
            actual = restored_queue.take()
            assert all(actual[k] == expected[k] for k in ('id', 'ticket', 'seed', 'text', 'sha256'))
            engine.lib.v091_destroy(actual['pointer'])
        finally: restored_queue.close()
        varied = sum(len({r['T'] for r in rows[i:i+4] if r['E'] == 0}) >= 2 for i in range(0, len(rows), 4))
        elapsed = time.monotonic()-started
        passed = sum(r['E'] == 0 for r in rows) >= 28 and varied >= 4 and elapsed <= 600
        result = dict(passed=passed, all_legally_replayed=True, value_independent=True, critic_unchanged=True,
                      prefix_only_update=True, optimizer_roundtrip=True, generator_roundtrip=True,
                      resume_weight_max_error=error, varied_groups=varied, seconds=elapsed, timing=timing,
                      metrics=metrics, summary=summary(rows), completed_at=now())
        save(directory/'result.json', result); return result
    finally:
        if queue: queue.close()
        pool.close(); engine.close()


class AuditQueue:
    def __init__(self, engine, cases):
        self.engine = engine; self.cases = iter(enumerate(cases))
    def take(self):
        i, case = next(self.cases); path = BC_RUN/case['path']; assert sha(path) == case['sha256']
        text = normalized(path.read_text()); _, statistics = describe(text)
        return self.engine.from_text(dict(id=2000000+i, ticket=i, seed=case['seed'], text=text,
                                         sha256=digest(text), statistics=statistics))


def audit(root, label):
    directory = root/'audit'/label; directory.mkdir(parents=True, exist_ok=True)
    if (directory/'result.json').exists(): return load(directory/'result.json')
    cases = [c for c in load(BC_RUN/'input_manifest.json') if c['role'] == 'validation'][::8]
    assert len(cases) == 32
    save(directory/'input_manifest.json', cases)
    config = configuration(); data, engine = engine_for(root, config)
    pool = CompletionPool(build_completion(root), config['completion_workers'])
    path = root/('initial/checkpoint.pt' if label == 'before' else 'training/latest.pt')
    model, _ = start_model(path, 'mps', config)
    try:
        _, rows, _, items, timing = collect(model, engine, AuditQueue(engine, cases), pool, make_rngs(109800),
                                           'mps', config, boundary_indexes=np.arange(32)%4)
        save(directory/'episodes.json', dict(rows=rows, inputs=serialized(items)))
        result = dict(summary=summary(rows), by_boundary={str(i): summary([r for r in rows if r['boundary'] == i])
                      for i in range(4)}, checkpoint_sha256=sha(path), timing=timing, completed_at=now())
        save(directory/'result.json', result); return result
    finally: pool.close(); engine.close()


def train(root):
    config = configuration(); directory = root/'training'; directory.mkdir(parents=True, exist_ok=True)
    if (directory/'result.json').exists(): return load(directory/'result.json')
    identity = dict(config=config, initial_checkpoint_sha256=sha(root/'initial/checkpoint.pt'),
                    teacher_dataset_sha256=sha(BC_RUN/'data/dataset.json'), device='mps')
    assert identity['initial_checkpoint_sha256'] == INITIAL_SHA
    if (directory/'config.json').exists(): assert load(directory/'config.json') == identity
    else: save(directory/'config.json', identity)
    data, engine = engine_for(root, config); queue = None
    pool = CompletionPool(build_completion(root), config['completion_workers'])
    model, optimizer = start_model(root/'initial/checkpoint.pt', 'mps', config)
    rngs = make_rngs(config['seed']); torch.manual_seed(config['seed']); torch.mps.manual_seed(config['seed'])
    previous = 0.; iteration = steps = 0; history = []; saved = None
    if (directory/'latest.pt').exists():
        saved = torch.load(directory/'latest.pt', map_location='cpu', weights_only=False)
        assert saved['identity'] == identity
        restore(model, optimizer, rngs, saved, 'mps')
        previous, iteration, steps, history = (saved[k] for k in ('seconds', 'iteration', 'steps', 'history'))
    blocked = blocked_inputs(root, iteration); save(directory/'excluded_sha256.json', sorted(blocked))
    queue = OfficialQueue(directory, engine, blocked, config, saved['queue'] if saved else None)
    queue.wait_full(); del saved
    started = time.monotonic(); stopped = False; last_report = 0.
    def seconds(): return previous+time.monotonic()-started
    def stop(signum, frame):
        nonlocal stopped
        stopped = True
    signal.signal(signal.SIGTERM, stop); signal.signal(signal.SIGINT, stop)
    def persist():
        assert not engine.live
        snapshot = queue.snapshot()
        try:
            checkpoint(directory/'latest.pt', dict(model_state(model, optimizer, rngs), identity=identity,
                       queue=snapshot, seconds=seconds(), iteration=iteration, steps=steps, history=history))
            (directory/'metrics.jsonl').write_text(''.join(json.dumps(row, allow_nan=False)+'\n' for row in history))
        finally: queue.resume()
    def report(phase, part, finished=None):
        nonlocal last_report
        if time.monotonic()-last_report < 30: return
        status(directory, phase, iteration=iteration, steps=steps, part=part, finished=finished,
               seconds=seconds(), queue_ready=len(queue.ready), active_geometries=len(engine.live), device='mps')
        last_report = time.monotonic()
    reason = 'iteration_budget'
    try:
        if iteration == 0: persist()
        while iteration < config['iterations']:
            if stopped: persist(); raise InterruptedError('stopped at saved cohort boundary')
            if seconds() >= config['seconds']: reason = 'time_budget'; break
            if datetime.now().astimezone() >= datetime.fromisoformat(config['final_deadline']): reason = 'deadline'; break
            queue.set_update_phase(False); tick = time.monotonic()
            buffer, rows, final_states, items, timing = collect(model, engine, queue, pool, rngs, 'mps', config,
                                                 lambda n, done: report('collecting_prefix_and_dp', n, done))
            rollout_seconds = time.monotonic()-tick; queue.set_update_phase(True); tick = time.monotonic()
            metrics = optimize(model, optimizer, engine, data, buffer, rngs['shuffle'], rngs['auxiliary'], 'mps', config,
                               iteration/max(1, config['iterations']-1), lambda n: report('updating', n))
            update_seconds = time.monotonic()-tick; iteration += 1; steps += len(buffer['ids'])
            lengths = np.array([r['T'] for r in rows]).reshape(config['boards'], config['repetitions'])
            row = dict(iteration=iteration, steps=steps, seconds=seconds(), rollout_seconds=rollout_seconds,
                       update_seconds=update_seconds, mean_group_spread=float(np.mean(np.ptp(lengths, axis=1))),
                       queue_wait_seconds=queue.wait_seconds, queue_ready=len(queue.ready),
                       **summary(rows), **timing, **metrics)
            save(directory/'episodes'/f'{iteration:04d}.json', dict(rows=rows, inputs=serialized(items)))
            history.append(row); blocked.update(item['sha256'] for item in items)
            engine.release_except([]); engine.buffers.clear()
            del buffer, final_states, items, rows
            persist(); status(directory, 'iteration_completed', **row)
            print(json.dumps(row, allow_nan=False), flush=True)
        assert iteration > 0
        persist()
        save(directory/'model.json', dict(parameters={k: v.detach().cpu().tolist() for k, v in model.state_dict().items()},
             spec=model.spec, size='small', epoch=iteration, rl_iterations=iteration, environment_steps=steps,
             dataset_sha256=identity['teacher_dataset_sha256'], initial_checkpoint_sha256=INITIAL_SHA,
             seed=config['seed'], method='group comparison of measured prefix plus DP completion'))
        result = dict(iterations=iteration, environment_steps=steps, seconds=seconds(), reason=reason,
                      episodes=sum(r['episodes'] for r in history), successes=sum(r['successes'] for r in history),
                      history=history, checkpoint_sha256=sha(directory/'latest.pt'),
                      model_sha256=sha(directory/'model.json'), completed_at=now())
        save(directory/'result.json', result); status(directory, 'training_completed', iterations=iteration, steps=steps, reason=reason)
        return result
    finally:
        if queue: queue.close()
        pool.close(); engine.close()


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, required=True)
    p.add_argument('--phase', choices=('mechanism', 'before', 'train', 'after'), required=True)
    p.add_argument('--gpu-state', type=Path, required=True); a = p.parse_args()
    torch.set_num_threads(2); torch.set_num_interop_threads(2)
    torch.mps.set_per_process_memory_fraction(min(1., 16e9/torch.mps.recommended_max_memory()))
    reporter = GPUReport(a.gpu_state)
    try:
        root = a.run.resolve()
        result = mechanism(root) if a.phase == 'mechanism' else train(root) if a.phase == 'train' else audit(root, a.phase)
        print(json.dumps({k: v for k, v in result.items() if k != 'history'}, allow_nan=False), flush=True)
    finally: reporter.close()
