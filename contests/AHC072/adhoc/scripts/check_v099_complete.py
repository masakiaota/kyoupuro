#!/usr/bin/env python3
"""完走採取、実測目標、GPU更新、optimizerと入力生成の再開を確かめる。"""
import argparse
from pathlib import Path
import tempfile
import time

import numpy as np
import torch

from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from train_v090_board import tensors
from v089_data import Geometry, remaining, save, sha, now
from v091_env import build_library
from v092_stream import TeacherData, OfficialQueue, excluded_inputs
from v099_complete import (RUN, BC_RUN, INITIAL, INITIAL_SHA, configuration, start_model, FreshEngine,
                           collect, measured_targets, optimize, loss)
from check_v098_complete import action_line


def execute(root):
    directory = root / 'mechanism'; directory.mkdir(parents=True, exist_ok=True)
    assert not (directory / 'result.json').exists(), 'completed mechanism must not be repeated'
    assert sha(INITIAL) == INITIAL_SHA
    config = dict(configuration('group'), boards=2, repetitions=4, environments=8, queue_capacity=8,
                  minibatch=64, epochs=1, bc_batch=8, bc_pool_cases=8, seed_base=990900000000)
    data = TeacherData(BC_RUN / 'data'); engine = FreshEngine(build_library(directory / 'environment'), data, 8)
    data.engine = engine
    queue = OfficialQueue(directory, engine, excluded_inputs(), config)
    rngs = [np.random.default_rng(99100+i) for i in range(4)]
    model, _ = start_model(INITIAL, 'mps', config)
    queue.wait_full(); queue.set_update_phase(False); started = time.monotonic()
    try:
        buffer, episodes, end_states, items = collect(model, engine, queue, rngs[0], rngs[1], 'mps', config)
        collect_seconds = time.monotonic()-started
        for i, item in enumerate(items):
            inp = directory / f'{i}.txt'; inp.write_text(item['text']); geo = Geometry(inp)
            for slot in range(i*4, i*4+4):
                state = geo.initial.copy()
                for code in episodes[slot]['actions']: geo.apply(state, action_line(code))
                assert np.array_equal(state, end_states[slot]) and remaining(state) == episodes[slot]['E']
        lengths = np.array([r['T'] for r in episodes]); left = np.array([r['E'] for r in episodes])
        for method in ('group', 'mc_ppo'):
            one = measured_targets(lengths, left, buffer['episode'], buffer['time'], buffer['value'], 4, method)
            two = measured_targets(lengths, left, buffer['episode'], buffer['time'], buffer['value']+100, 4, method)
            assert np.array_equal(one[0], two[0])
            if method == 'group': assert np.array_equal(one[1], two[1])
            else: assert np.allclose(one[1]-100, two[1])
        synthetic = measured_targets(np.array([1,2,5,2048]), np.array([1,0,0,0]), np.arange(4), np.zeros(4), np.zeros(4), 4, 'group')
        assert synthetic[2][0] > synthetic[2][3] and synthetic[3][1] > synthetic[3][2]
        assert abs(float(synthetic[3].sum())) < 1e-10
        # 再開位置の待ち行列内容と、次のseed・盤面・乱数を一致させる。
        queue.set_update_phase(True); queue.wait_full(); snapshot = queue.snapshot()
        expected = snapshot['ready'][0]; queue.close()
        queue = OfficialQueue(directory, engine, excluded_inputs(), config, snapshot)
        queue.pause(); received = queue.take()
        assert all(received[k] == expected[k] for k in ('ticket','seed','sha256','text'))
        engine.lib.v091_destroy(received['pointer'])
        updates = {}; initial = torch.load(INITIAL, map_location='cpu', weights_only=False)['model']
        for method in ('mc_ppo', 'group'):
            current_config = dict(config, method=method)
            current, optimizer = start_model(INITIAL, 'mps', current_config)
            assert all(torch.equal(v.cpu(), initial[k]) for k,v in current.state_dict().items())
            returns, adv, _, _ = measured_targets(lengths, left, buffer['episode'], buffer['time'], buffer['value'], 4, method)
            current_buffer = dict(buffer, returns=returns, advantages=adv)
            tick = time.monotonic()
            metrics = optimize(current, optimizer, engine, data, current_buffer,
                               np.random.default_rng(99103), np.random.default_rng(99104), 'mps', current_config, 0.)
            assert metrics['updates'] > 0 and metrics['bc_ce'] > 0
            assert (metrics['value_loss'] > 0) == (method == 'mc_ppo')
            assert any(not torch.equal(v.cpu(), initial[k]) for k,v in current.state_dict().items())
            if method == 'group':
                assert all(torch.equal(v.cpu(), initial[k]) for k,v in current.state_dict().items() if k.startswith('critic.'))
            # 保存したoptimizerから同じ1更新を再現する。評価による条件調整は行わない。
            snapshot_path = directory / f'{method}_resume.pt'
            checkpoint(snapshot_path, dict(model={k:v.detach().cpu() for k,v in current.state_dict().items()}, optimizer=optimizer.state_dict()))
            restored, other = start_model(INITIAL, 'mps', current_config)
            saved = torch.load(snapshot_path, map_location='cpu', weights_only=False)
            restored.load_state_dict(saved['model']); other.load_state_dict(saved['optimizer'])
            idx = np.arange(32)
            for net, opt in ((current,optimizer),(restored,other)):
                raw, _, _ = engine.observe(buffer['ids'][idx], buffer['states'][idx], buffer['groups'][idx])
                total, _ = loss(net, raw, *(current_buffer[k][idx] for k in ('action','old_logp','advantages','returns')),
                                'mps', current_config, 1.)
                opt.zero_grad(set_to_none=True); total.backward(); torch.nn.utils.clip_grad_norm_(net.parameters(),1.); opt.step()
            error = max(float((v-restored.state_dict()[k]).abs().max()) for k,v in current.state_dict().items())
            assert error < 1e-6, error
            updates[method] = dict(metrics, seconds=time.monotonic()-tick, resume_max_error=error)
            del current, restored, optimizer, other, saved
        result = dict(passed=True, full_episodes=len(episodes), successes=sum(r['E']==0 for r in episodes),
                      independent_replay=True, return_independent_of_value=True, group_no_critic_update=True,
                      queue_resume_identical=True, initial_sha256=INITIAL_SHA, updates=updates,
                      collect_seconds=collect_seconds, seconds=time.monotonic()-started, device='mps', completed_at=now())
        save(directory / 'result.json', result); print(result, flush=True)
    finally: queue.close(); engine.close()


if __name__ == '__main__':
    p=argparse.ArgumentParser(); p.add_argument('--run',type=Path,default=RUN); a=p.parse_args()
    torch.set_num_threads(2); torch.set_num_interop_threads(2)
    torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    reporter=GPUReport(a.run/'gpu_state.json')
    try: execute(a.run.resolve())
    finally: reporter.close()
