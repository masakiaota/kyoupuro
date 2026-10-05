#!/usr/bin/env python3
"""固定方策の完走列を採取し、実測手数による群内比較を診断する。"""
import argparse
import csv
from pathlib import Path
import subprocess
import time
from types import SimpleNamespace

import numpy as np
import torch

from train_v080_scaling import GPUReport
from train_v090_board import Model
from train_v091_ppo import act
from v089_data import Geometry, remaining, save, sha, now, status, ROOT
from v091_env import build_library
from v092_stream import FreshEngine, LIVE_BASE, describe, normalized, digest, excluded_inputs

PARENT = ROOT / 'results/nn_rank/v092/20261003_fresh_studio'
CHECKPOINT = PARENT / 'transition_full_episode_20261003/training/latest.pt'
FINGERPRINT = 'c87b59b7926773dceeee98f0e394871052aa82b0109eb24591689595abc5859d'
RUN = ROOT / 'results/nn_rank/v098/20261003_complete_probe'
CONFIG = dict(inputs=32, repetitions=4, max_steps=2048, seed=98003,
              official_seed_base=980000000000, cpu_workers=8, device='mps')


def action_line(code):
    p = code & 511
    return f"{p // 20} {p % 20} {(code >> 9) & 7} {'UDLR'[(code >> 12) & 3]} {((code >> 14) & 7) + 1}"


def main(root):
    root.mkdir(parents=True, exist_ok=True)
    assert not (root / 'started.json').exists(), 'Existing diagnostic must be inspected before restarting.'
    assert sha(CHECKPOINT) == FINGERPRINT
    save(root / 'started.json', dict(config=CONFIG, checkpoint_sha256=FINGERPRINT, started_at=now()))
    started = time.monotonic()
    seeds = [CONFIG['official_seed_base'] + i for i in range(CONFIG['inputs'])]
    seed_file = root / 'seeds.txt'
    seed_file.write_text(''.join(f'{s}\n' for s in seeds))
    subprocess.run([ROOT / 'tools/target/release/gen', seed_file, '--dir', root / 'inputs'],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    blocked, seen, inputs = excluded_inputs(), set(), []
    engine = FreshEngine(build_library(PARENT / 'environment'), SimpleNamespace(by_index={}), CONFIG['cpu_workers'])
    try:
        for i, seed in enumerate(seeds):
            path = root / 'inputs' / f'{i:04d}.txt'
            text = normalized(path.read_text())
            fingerprint = digest(text)
            assert fingerprint not in blocked and fingerprint not in seen
            seen.add(fingerprint)
            initial, distribution = describe(text)
            item = dict(id=LIVE_BASE + i, seed=seed, text=text, sha256=fingerprint,
                        statistics=distribution, role='training_diagnostic')
            engine.register(engine.from_text(item))
            inputs.append(dict(index=i, seed=seed, path=str(path), sha256=sha(path), **distribution))
        save(root / 'inputs.json', inputs)
        saved = torch.load(CHECKPOINT, map_location='cpu', weights_only=False)
        original = {k: v.clone() for k, v in saved['model'].items()}
        model = Model(saved['identity']['config']['model_spec']).to('mps')
        model.load_state_dict(original)
        model.eval()
        del saved
        rng = np.random.default_rng(CONFIG['seed'])
        ids = np.repeat(np.arange(LIVE_BASE, LIVE_BASE + CONFIG['inputs'], dtype=np.int32), CONFIG['repetitions'])
        states = np.stack([describe(engine.live[int(cid)]['text'])[0] for cid in ids])
        groups = np.zeros(len(ids), np.int32)
        active = np.ones(len(ids), bool)
        counts = np.zeros(len(ids), np.int32)
        ended_remaining = np.full(len(ids), -1, np.int32)
        actions = [[] for _ in ids]
        last_size = None
        collect_started = time.monotonic()
        for step in range(CONFIG['max_steps']):
            slots = np.flatnonzero(active)
            if not len(slots): break
            # 可変batchの全サイズをキャッシュするとメモリが積み上がるため、現在のサイズだけを保持する。
            if len(slots) != last_size:
                engine.buffers.clear()
                last_size = len(slots)
            current = np.ascontiguousarray(states[slots])
            raw, codes, _ = engine.observe(ids[slots], current, groups[slots])
            chosen, _, _ = act(model, raw, rng, 'mps')
            chosen_codes = codes[np.arange(len(slots)), chosen].copy()
            left, dead = engine.step(ids[slots], current, chosen_codes)
            states[slots] = current
            counts[slots] += 1
            for slot, code in zip(slots, chosen_codes): actions[int(slot)].append(int(code))
            done = (left == 0) | dead | (counts[slots] >= CONFIG['max_steps'])
            ended = slots[done]
            ended_remaining[ended] = left[done]
            active[ended] = False
            if step % 64 == 63:
                status(root, 'collecting_complete_episodes',steps=step+1,finished=int((~active).sum()),
                       seconds=time.monotonic()-started)
        assert not active.any() and (ended_remaining >= 0).all()
        collect_seconds = time.monotonic() - collect_started
        assert all(torch.equal(original[k], v.detach().cpu()) for k, v in model.state_dict().items())
        output = root / 'outputs'
        output.mkdir()
        rows, boards = [], []
        status(root, 'independent_replay', seconds=time.monotonic()-started)
        for i, item in enumerate(inputs):
            geometry = Geometry(Path(item['path']))
            costs, turns = [], []
            for j in range(CONFIG['repetitions']):
                slot = i * CONFIG['repetitions'] + j
                state = geometry.initial.copy()
                lines = [action_line(code) for code in actions[slot]]
                for line, code in zip(lines, actions[slot]): assert geometry.apply(state, line) == code
                assert np.array_equal(state, states[slot])
                E, T = remaining(state), len(lines)
                assert E == int(ended_remaining[slot]) and T == int(counts[slot])
                (output / f'{i:04d}_{j}.txt').write_text('\n'.join(lines) + '\n')
                # 従来の失敗罰則を維持し、短く失敗した列を成功列より高く評価しない。
                cost = T / 100. + (30. + E / 256. if E else 0.)
                costs.append(cost); turns.append(T)
                rows.append(dict(case=i, repetition=j, seed=item['seed'], T=T, E=E, cost=cost, legal=True))
            costs = np.asarray(costs)
            # 自分の結果を除いた3本を基準にすれば、基準が自分の行動に依存しない。
            advantages = (costs.sum() - costs) / (CONFIG['repetitions'] - 1) - costs
            assert abs(float(advantages.sum())) < 1e-8
            for row, advantage in zip(rows[-CONFIG['repetitions']:], advantages): row['group_advantage'] = float(advantage)
            complete = all(r['E'] == 0 for r in rows[-CONFIG['repetitions']:])
            boards.append(dict(case=i, all_complete=complete, turns=turns, shortest=min(turns),
                               longest=max(turns), mean=float(np.mean(turns)), spread=max(turns)-min(turns)))
        complete_boards = [x for x in boards if x['all_complete']]
        differentiated = sum(x['spread'] >= 5 for x in complete_boards)
        total_seconds = time.monotonic() - started
        result = dict(config=CONFIG, checkpoint_sha256=FINGERPRINT, parameters_unchanged=True,
                      all_legal=True, episodes=len(rows), completed=sum(x['E']==0 for x in rows),
                      complete_boards=len(complete_boards), differentiated_boards=differentiated,
                      mean_T_completed=float(np.mean([x['T'] for x in rows if x['E']==0])),
                      mean_within_board_spread=float(np.mean([x['spread'] for x in complete_boards])) if complete_boards else None,
                      median_within_board_spread=float(np.median([x['spread'] for x in complete_boards])) if complete_boards else None,
                      mean_best_of_four_gain=float(np.mean([x['mean']-x['shortest'] for x in complete_boards])) if complete_boards else None,
                      collect_seconds=collect_seconds, seconds=total_seconds, learned_updates=0,
                      feasible=total_seconds <= 300 and len(complete_boards)>=30 and differentiated>=24,
                      completed_at=now())
        save(root/'episodes.json', rows); save(root/'boards.json', boards); save(root/'result.json', result)
        with (root/'episodes.csv').open('w') as stream:
            writer=csv.DictWriter(stream,fieldnames=list(rows[0]));writer.writeheader();writer.writerows(rows)
        status(root,'completed',**result)
        print(result,flush=True)
    finally:
        engine.close()


if __name__ == '__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--run',type=Path,default=RUN);args=parser.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2)
    torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    reporter=GPUReport(args.run/'gpu_state.json')
    try:main(args.run.resolve())
    finally:reporter.close()
