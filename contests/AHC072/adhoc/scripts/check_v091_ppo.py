#!/usr/bin/env python3
"""PPO環境の正確さ、更新、保存再開と提出側の数値を確認する。"""
import argparse
import json
from pathlib import Path
import subprocess

import numpy as np
import torch
import torch.nn.functional as F

from build_v090_board import build, quantize
from check_v089_board import compile_binary, transformed_input
from train_v077_rank import checkpoint
from train_v090_board import Model, tensors
from train_v091_ppo import initial_model, collect, optimize, ppo_loss, act
from v090_data import Dataset, Geometry, remaining, save, sha, now
from v091_env import BC_RUN, ROOT, RUN, CONFIG, Engine, Episodes, advantages, rewards_and_done, build_library, load


def line(code):
    p = int(code) & 511
    return f'{p // 20} {p % 20} {(int(code) >> 9) & 7} {"UDLR"[(int(code) >> 12) & 3]} {((int(code) >> 14) & 7) + 1}'


def mechanism(root):
    directory = root / 'mechanism'; directory.mkdir(parents=True, exist_ok=True)
    marker = directory / 'result.json'
    if marker.exists(): return load(marker)
    data = Dataset(BC_RUN / 'data'); binary = build_library(root / 'environment')
    engine = Engine(binary, data); rng = np.random.default_rng(CONFIG['seed'])
    train = [c for c in data.cases if c['role'] == 'train']
    selected = train[:4] + [train[i] for i in rng.choice(len(train), 12, replace=False)]
    ids = np.array([c['frame_start'] + c['frames'] // 2 for c in selected])
    cases = np.asarray(data.case_ids[ids]); states = np.asarray(data.states[ids])
    errors = dict(board=0., features=0.)
    for group in range(8):
        groups = np.full(len(ids), group)
        raw, codes, counts = engine.observe(cases, states, groups)
        expected = data.batch(ids, groups=groups)
        errors['board'] = max(errors['board'], float(np.max(np.abs(raw['x'] - expected['x']))))
        assert np.array_equal(raw['valid'], expected['valid'])
        for b, fid in enumerate(ids):
            n = counts[b]
            assert np.array_equal(codes[b, :n], data.codes[data.offsets[fid]:data.offsets[fid + 1]])
            assert np.array_equal(raw['src'][b, :n], expected['src'][b, :n])
            assert np.array_equal(raw['dst'][b, :n], expected['dst'][b, :n])
            errors['features'] = max(errors['features'], float(np.max(np.abs(raw['features'][b, :n] - expected['features'][b, :n]))))
    assert max(errors.values()) < 2e-6
    geo = {c['index']: Geometry(BC_RUN / c['path']) for c in selected}
    independent = states.copy(); cpp = states.copy(); transitions = 0
    for _ in range(64):
        raw, codes, counts = engine.observe(cases, cpp, rng.integers(8, size=len(cases)))
        chosen = np.array([codes[b, rng.integers(n)] for b, n in enumerate(counts)])
        for b, cid in enumerate(cases): geo[int(cid)].apply(independent[b], line(chosen[b]))
        e, dead = engine.step(cases, cpp, chosen)
        assert np.array_equal(cpp, independent)
        assert np.array_equal(e, [remaining(s) for s in independent]) and not dead.any()
        transitions += len(cases)
        assert (e > 0).all(), 'unexpected completion in a random-transition diagnostic'
    # 最後の帰巣とエピソード終端も、教師の最終操作で独立確認する。
    finals = np.array([c['frame_start'] + c['frames'] - 1 for c in selected]); states = np.asarray(data.states[finals]).copy()
    last_codes = np.array([data.codes[data.offsets[f] + data.targets[f]] for f in finals], np.uint32)
    for b, cid in enumerate(cases): geo[int(cid)].apply(states[b], line(last_codes[b]))
    cpp = np.asarray(data.states[finals]).copy(); e, dead = engine.step(cases, cpp, last_codes)
    assert np.array_equal(states, cpp) and (e == 0).all() and not dead.any()
    # 終端後の別入力の価値が、前エピソードの利得に漏れないことを確認する。
    r = np.full((3, 2), -.01, np.float32); done = np.array([[False, False], [True, False], [True, True]])
    adv, ret = advantages(r, done, np.zeros_like(r), np.array([99., 99.], np.float32), dict(CONFIG, gae_lambda=1.))
    assert np.allclose(ret, [[-.02, -.03], [-.01, -.02], [-.01, -.01]])
    reward, terminal = rewards_and_done(np.array([0, 2, 3, 1]), np.array([0, 0, 1, 0], bool), np.array([2048, 2048, 8, 8]))
    assert np.array_equal(terminal, [1, 1, 1, 0]) and np.allclose(reward, [-.01, -30.01 - 2/256, -30.01 - 3/256, -.01])
    # MPSの実更新と、同一データ・最適化状態からの再開を確認する。
    device = 'mps'; model = initial_model(device)
    config = dict(CONFIG, environments=8, horizon=16, epochs=1, minibatch=128, bc_batch=8)
    episodes = Episodes(data, rng, 8, False)
    buffer, _ = collect(model, engine, episodes, rng, device, config)
    raw, _, _ = engine.observe(buffer['ids'], buffer['states'], buffer['groups'])
    _, stats = ppo_loss(model, raw, *(buffer[k] for k in ('action', 'old_logp', 'advantages', 'returns')), device, config)
    assert abs(float(stats[3])) < 1e-5 and float(stats[4]) == 0
    optimizer = torch.optim.AdamW(model.parameters(), lr=config['initial_lr'], weight_decay=config['weight_decay'])
    before = {k: v.detach().cpu().clone() for k, v in model.state_dict().items()}
    first = optimize(model, optimizer, engine, data, buffer, rng, device, config)
    change = max(float((v.detach().cpu() - before[k]).abs().max()) for k, v in model.state_dict().items())
    assert change > 0
    checkpoint(directory / 'resume.pt', dict(model=model.state_dict(), optimizer=optimizer.state_dict(),
                                             rng=rng.bit_generator.state, episodes=episodes.state_dict()))
    restored = torch.load(directory / 'resume.pt', map_location='cpu', weights_only=False)
    other = initial_model(device); other.load_state_dict(restored['model'])
    other_opt = torch.optim.AdamW(other.parameters(), lr=config['initial_lr'], weight_decay=config['weight_decay'])
    other_opt.load_state_dict(restored['optimizer']); other_rng = np.random.default_rng(); other_rng.bit_generator.state = restored['rng']
    optimize(model, optimizer, engine, data, buffer, rng, device, config)
    optimize(other, other_opt, engine, data, buffer, other_rng, device, config)
    resume_error = max(float((v - other.state_dict()[k]).abs().max()) for k, v in model.state_dict().items())
    assert resume_error < 1e-5 and rng.bit_generator.state == other_rng.bit_generator.state
    other_episodes = Episodes(data, other_rng, 8, False)
    other_episodes.restore(restored['episodes']); other_rng.bit_generator.state = rng.bit_generator.state
    raw, codes, _ = engine.observe(episodes.ids, episodes.states, episodes.groups)
    actions, _, _ = act(model, raw, rng, device); chosen = codes[np.arange(8), actions].copy()
    raw, codes, _ = engine.observe(other_episodes.ids, other_episodes.states, other_episodes.groups)
    action2, _, _ = act(other, raw, other_rng, device)
    assert np.array_equal(actions, action2)
    engine.step(episodes.ids, episodes.states, chosen)
    engine.step(other_episodes.ids, other_episodes.states, codes[np.arange(8), action2].copy())
    assert np.array_equal(episodes.states, other_episodes.states)
    # 正負の利得を与えた手の確率が、対応する向きへ動くことを別の初期重みで確認する。
    direction_checks = []
    raw, _, _ = engine.observe(cases[:4], np.asarray(data.states[ids[:4]]), np.zeros(4, np.int32))
    for sign in (1., -1.):
        probe = initial_model(device); opt = torch.optim.SGD(probe.parameters(), lr=1e-5)
        logits, _ = probe(tensors(raw, device)); action = logits.argmax(-1)
        score = F.log_softmax(logits, -1).gather(1, action[:, None]).mean(); before_score = float(score.detach())
        opt.zero_grad(); (-sign * score).backward(); opt.step()
        with torch.no_grad():
            logits, _ = probe(tensors(raw, device)); after_score = float(F.log_softmax(logits, -1).gather(1, action[:, None]).mean())
        assert sign * (after_score - before_score) > 0
        direction_checks.append(dict(advantage=sign, logp_change=after_score - before_score))
    result = dict(passed=True, transforms=len(ids)*8, feature_error=errors, independent_transitions=transitions,
                  terminal_transitions=len(ids), first_update=first, parameter_change=change, resume_max_error=resume_error,
                  signed_advantage=direction_checks, device=str(next(model.parameters()).device), completed_at=now())
    save(marker, result); engine.close(); print(json.dumps(result), flush=True)
    return result


def numerical(root):
    directory = root / 'numerical'; directory.mkdir(parents=True, exist_ok=True)
    marker = directory / 'result.json'; model_path = root / 'training/model.json'
    if marker.exists():
        result = load(marker); assert result['model_sha256'] == sha(model_path); return result
    source = ROOT / 'src/bin/v091_nn_ppo.cpp'; storage = build(model_path, source)
    source.write_text(source.read_text().replace('// epoch=', '// PPO iteration=', 1))
    diagnostic = ROOT / 'adhoc/bin/check_v091_ppo.cpp'
    diagnostic.write_text('// check_v091_ppo.cpp\n#define V090_DIAGNOSTIC\n#include "../../src/bin/v091_nn_ppo.cpp"\n')
    binaries = [directory / 'checker_local', directory / 'checker_judge']
    for binary, local in zip(binaries, [True, False]): compile_binary(diagnostic.stem, binary, local)
    data = Dataset(BC_RUN / 'data'); saved = load(model_path); model = Model(saved['spec'])
    restored, _, _ = quantize(saved['parameters']); model.load_state_dict({k: torch.from_numpy(v) for k, v in restored.items()})
    model.eval(); errors = dict(board_features=0., action_features=0., logits=0., value=0.); checks = 0
    for case in [c for c in data.cases if c['role'] == 'train'][:4]:
        fid = case['frame_start'] + case['frames'] // 2; geo = Geometry(BC_RUN / case['path'])
        for group in range(8):
            mapping = data.maps[geo.N][group]; inp = transformed_input(geo, mapping)
            state = np.zeros(400, np.uint32); state[mapping] = data.states[fid]
            snapshot = directory / 'snapshot.txt'; snapshot.write_text(' '.join(map(str, state)) + '\n')
            raw = data.batch([fid], groups=[group])
            with torch.no_grad(): logits, value = model(tensors(raw, 'cpu'))
            codes = data.codes[data.offsets[fid]:data.offsets[fid+1]].astype(np.int64)
            transformed = mapping[codes & 511] | (codes & (7 << 9)) | (data.dirs[geo.N][group][(codes >> 12) & 3] << 12) | (codes & (7 << 14))
            for binary in binaries if group == 0 else binaries[:1]:
                proc = subprocess.run([binary, snapshot, 'dump'], input=inp, text=True, capture_output=True, check=True, timeout=10)
                out = json.loads(proc.stdout); index = {code: i for i, code in enumerate(out['codes'])}
                assert set(index) == set(transformed.tolist())
                order = [index[int(code)] for code in transformed]
                values = dict(board_features=np.max(np.abs(np.array(out['x']).reshape(400,40)-raw['x'][0].reshape(40,400).T)),
                              action_features=np.max(np.abs(np.array(out['features']).reshape(-1,16)[order]-raw['features'][0])),
                              logits=np.max(np.abs(np.array(out['logits'])[order]-logits[0].numpy())), value=abs(out['value']-float(value[0])))
                for k,v in values.items(): errors[k]=max(errors[k],float(v))
                checks += 1
    assert errors['board_features']<2e-6 and errors['action_features']<2e-6 and errors['logits']<.003 and errors['value']<.03, errors
    result = dict(passed=True, checks=checks, errors=errors, model_sha256=sha(model_path), solver_sha256=sha(source), storage=storage, completed_at=now())
    save(marker, result); print(json.dumps(result), flush=True); return result


if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);p.add_argument('--numerical',action='store_true');args=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2)
    if not args.numerical: torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    (numerical if args.numerical else mechanism)(args.run.resolve())
