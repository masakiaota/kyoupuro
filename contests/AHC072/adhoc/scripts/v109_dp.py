#!/usr/bin/env python3
"""NN前半と配送DPの実測総手数による、価値学習なしの群内比較。"""
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import time

import numpy as np

from check_v089_board import compile_binary
from v090_data import save, sha, now
from v091_env import load
from v092_stream import ROOT, BC_RUN, describe, digest, normalized, excluded_inputs
from v099_complete import configuration as parent_configuration, measured_targets
from v101_compute import act

RUN = ROOT/'results/nn_rank/v109/20261004_dp_reward_studio'
INITIAL = ROOT/'results/nn_rank/v105/20261004_nn_lns_studio/frozen/checkpoint.pt'
INITIAL_SHA = 'e9d615587a1a655b7b2818e2f19564619050e2ffd669050b015b7c46766b90d3'
PARENT = ROOT/'adhoc/bin/v108_assisted_base.cpp'
PARENT_SHA = 'e78b456fbb28d217cab5428979963aad1018270388acf03e64e26ba6585892d8'
FRACTIONS = ((3, 4), (1, 2), (1, 4), (1, 8))


def configuration():
    return dict(parent_configuration('group'), seed=109003, seed_base=1090000000000,
                iterations=240, seconds=7200, initial_lr=6e-6, final_lr=2e-6,
                value_coef=0., bc_coef=0., max_episode_steps=512,
                completion_workers=12, dp_build_budget=4096)


def blocked_inputs(root, completed_iterations=0):
    blocked = excluded_inputs()
    for path in (ROOT/'tools/validation1').glob('*.txt'):
        text = path.read_text(); blocked.update((digest(text), digest(normalized(text))))
    for directory, limit in ((root/'training/episodes', completed_iterations), (root/'mechanism/episodes', 1)):
        for path in directory.glob('*.json'):
            if int(path.stem) <= limit:
                blocked.update(item['sha256'] for item in load(path)['inputs'])
    return blocked


def build_completion(root):
    where = root/'completion'; where.mkdir(parents=True, exist_ok=True)
    assert sha(PARENT) == PARENT_SHA
    original = PARENT.read_text()
    start = original.index('class DeliveryDP {')
    end = original.index('\n};', start)+4
    # ネット重みとLNSは不要。独立再生用NN盤面と親のDPだけを取り出す。
    text = original[:original.index('constexpr int NN_WIDTH=')]
    text += original[original.index('} // namespace nn'):start]
    text += '''struct V109Budget {};
static int v109_dp_builds=0;
static void v109_dp_tick() {
    if(v109_dp_builds==4096)throw V109Budget();
    ++v109_dp_builds;
}
'''
    dp = original[start:end]; assert dp.count('time_keeper.check();') == 1
    text += dp.replace('time_keeper.check();', 'v109_dp_tick();')+'\n'
    text += (ROOT/'adhoc/scripts/v109_completion.cpp.txt').read_text()
    text = '// v109_dp_completion.cpp\n'+text.split('\n', 1)[1]
    source = ROOT/'adhoc/bin/v109_dp_completion.cpp'
    if source.exists(): assert source.read_text() == text
    else: source.write_text(text)
    binary = where/'complete'; marker = where/'build.json'
    identity = dict(source_sha256=sha(source), parent_sha256=PARENT_SHA)
    if marker.exists():
        recorded = load(marker); assert recorded['identity'] == identity and recorded['binary_sha256'] == sha(binary)
    else:
        compile_binary(source.stem, binary, local=False)
        save(marker, dict(identity=identity, binary_sha256=sha(binary), built_at=now()))
    return binary


class CompletionPool:
    def __init__(self, binary, workers=12):
        self.binary = binary
        self.executor = ThreadPoolExecutor(max_workers=workers)

    def submit(self, item, prefixes, states):
        payload = item['text']+str(len(prefixes))+'\n'
        for actions, state in zip(prefixes, states):
            payload += str(len(actions))+'\n'+' '.join(map(str, actions))+'\n'
            payload += ' '.join(map(str, state))+'\n'
        def execute():
            # 壊れた補完処理の停止用。通常の試行予算は機械速度に依存しないDP表の構築回数。
            proc = subprocess.run([self.binary], input=payload, text=True, capture_output=True, timeout=60)
            if proc.returncode: raise RuntimeError(f"completion seed={item['seed']}: {proc.stderr}")
            rows = json.loads(proc.stdout); assert len(rows) == len(prefixes)
            for row, actions in zip(rows, prefixes):
                assert row['verified'] and row['prefix_T'] == len(actions)
                assert row['T'] == len(actions)+len(row['tail'])
                assert (row['E'] == 0) == (row['status'] == 'completed')
            return rows
        return self.executor.submit(execute)

    def close(self): self.executor.shutdown(wait=True)


def collect(model, engine, queue, pool, rngs, device, config, report=None, boundary_indexes=None):
    items = [queue.take() for _ in range(config['boards'])]
    for item in items: engine.register(item)
    repetitions = config['repetitions']; size = len(items)*repetitions
    ids = np.repeat(np.array([item['id'] for item in items], np.int32), repetitions)
    groups = np.repeat(rngs['transform'].integers(8, size=len(items), dtype=np.int32), repetitions)
    boundary = (rngs['boundary'].integers(4, size=len(items)) if boundary_indexes is None else np.asarray(boundary_indexes))
    assert len(boundary) == len(items)
    targets = np.repeat([item['statistics']['M']*FRACTIONS[int(b)][0]//FRACTIONS[int(b)][1]
                         for item, b in zip(items, boundary)], repetitions)
    states = np.stack([describe(item['text'])[0] for item in items]).repeat(repetitions, axis=0)
    active = np.ones(size, bool); lengths = np.zeros(size, np.int32)
    prefix_E = np.full(size, -1, np.int32); reasons = ['']*size
    actions = [[] for _ in range(size)]; futures = {}
    parts = {k: [] for k in ('states', 'ids', 'groups', 'action', 'old_logp', 'value', 'episode', 'time')}
    previous_size = None; model.eval(); started = time.monotonic()
    for t in range(config['max_episode_steps']):
        slots = np.flatnonzero(active)
        if not len(slots): break
        if len(slots) != previous_size: engine.buffers.clear(); previous_size = len(slots)
        current = np.ascontiguousarray(states[slots]); current_ids = ids[slots]; current_groups = groups[slots]
        raw, codes, _ = engine.observe(current_ids, current, current_groups)
        chosen, logp, values = act(model, raw, rngs['action'], device)
        chosen_codes = codes[np.arange(len(slots)), chosen].copy()
        for key, value in dict(states=current.copy(), ids=current_ids, groups=current_groups,
                               action=chosen, old_logp=logp, value=values, episode=slots,
                               time=np.full(len(slots), t, np.int32)).items(): parts[key].append(value)
        left, dead = engine.step(current_ids, current, chosen_codes)
        states[slots] = current; lengths[slots] += 1
        for slot, code in zip(slots, chosen_codes): actions[int(slot)].append(int(code))
        done = (left <= targets[slots]) | dead | (lengths[slots] >= config['max_episode_steps'])
        ended = slots[done]; prefix_E[ended] = left[done]; active[ended] = False
        for local in np.flatnonzero(done):
            slot = int(slots[local])
            reasons[slot] = ('completed' if left[local] == 0 else 'boundary' if left[local] <= targets[slot]
                             else 'no_legal_action' if dead[local] else 'prefix_budget')
        # 4列が揃った盤面を直ちにCPUへ渡す。最大32盤面で、採取ごとに全結果を回収する。
        for board in range(len(items)):
            a = board*repetitions; b = a+repetitions
            if board not in futures and not active[a:b].any():
                futures[board] = pool.submit(items[board], actions[a:b], states[a:b].copy())
        if report and t % 64 == 63: report(t+1, int((~active).sum()))
    assert not active.any() and (prefix_E >= 0).all() and len(futures) == len(items)
    nn_seconds = time.monotonic()-started; wait_started = time.monotonic()
    completed = [row for board in range(len(items)) for row in futures[board].result()]
    dp_wait_seconds = time.monotonic()-wait_started
    buffer = {k: np.concatenate(v) for k, v in parts.items()}
    total = np.array([row['T'] for row in completed]); remaining = np.array([row['E'] for row in completed])
    returns, advantages, costs, relative = measured_targets(total, remaining, buffer['episode'], buffer['time'],
                                                           buffer['value'], repetitions, 'group')
    buffer.update(returns=returns, advantages=advantages)
    rows = []
    for slot, result in enumerate(completed):
        assert result['prefix_E'] == int(prefix_E[slot])
        board = slot//repetitions
        rows.append(dict(result, episode=slot, case=int(ids[slot]), seed=items[board]['seed'],
                         group=int(groups[slot]), repetition=slot%repetitions, boundary=int(boundary[board]),
                         target_E=int(targets[slot]), nn_stop=reasons[slot],
                         cost=float(costs[slot]), group_advantage=float(relative[slot]), actions=actions[slot]))
    timing = dict(nn_collect_seconds=nn_seconds, dp_wait_seconds=dp_wait_seconds,
                  dp_cpu_seconds=sum(r['dp_seconds'] for r in rows), dp_tasks=len(futures))
    return buffer, rows, states, items, timing


def make_rngs(seed):
    return {name: np.random.default_rng(seed+i*1009)
            for i, name in enumerate(('action', 'transform', 'shuffle', 'auxiliary', 'boundary'))}


def summary(rows):
    completed = [row for row in rows if row['E'] == 0]
    return dict(episodes=len(rows), successes=len(completed),
                mean_T_completed=float(np.mean([r['T'] for r in completed])) if completed else None,
                mean_E_finished=float(np.mean([r['E'] for r in rows])),
                mean_cost_all=float(np.mean([r['cost'] for r in rows])),
                mean_prefix_T=float(np.mean([r['prefix_T'] for r in rows])),
                mean_dp_T=float(np.mean([len(r['tail']) for r in rows])),
                mean_dp_builds=float(np.mean([r['dp_builds'] for r in rows])),
                statuses={status: sum(r['status'] == status for r in rows) for status in sorted({r['status'] for r in rows})})
