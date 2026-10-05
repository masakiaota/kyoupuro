#!/usr/bin/env python3
"""模倣学習済みの実操作方策を、固定予算のPPOで更新する。"""
import argparse
from datetime import datetime
import json
from pathlib import Path
import signal
import time

import numpy as np
import torch
import torch.nn.functional as F

from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from train_v090_board import Model, tensors
from v090_data import Dataset, save, sha, status, now
from v091_env import BC_RUN, RUN, CONFIG, Engine, Episodes, advantages, rewards_and_done, load, build_library


def initial_model(device):
    saved = load(BC_RUN / 'models/small/model.json')
    assert sha(BC_RUN / 'models/small/model.json') == '4e1e9759b17b901a81c8608eb90333a5e97eaa8602b7a6a8135a90827fb3b31d'
    model = Model(saved['spec'])
    model.load_state_dict({k: torch.tensor(v, dtype=torch.float32) for k, v in saved['parameters'].items()})
    return model.to(device)


@torch.no_grad()
def act(model, raw, rng, device):
    logits, value = model(tensors(raw, device))
    logp = F.log_softmax(logits, -1).cpu().numpy()
    # CPUの乱数を保存することで、MPS上での確率選択も再開可能にする。
    probabilities = np.exp(logp.astype(np.float64))
    cumulative = np.cumsum(probabilities, axis=-1)
    cumulative /= cumulative[:, -1:]
    cumulative[:, -1] = 1.
    action = (cumulative < rng.random((len(logp), 1))).sum(-1).astype(np.int64)
    assert raw['valid'][np.arange(len(action)), action].all()
    return action, logp[np.arange(len(action)), action], -value.cpu().numpy() / 100.


def collect(model, engine, episodes, rng, device, config, report=None):
    T, B = config['horizon'], episodes.size
    buffer = dict(states=np.empty((T, B, 400), np.uint32), ids=np.empty((T, B), np.int32),
                  groups=np.empty((T, B), np.int32), action=np.empty((T, B), np.int64),
                  old_logp=np.empty((T, B), np.float32), value=np.empty((T, B), np.float32),
                  reward=np.empty((T, B), np.float32), done=np.empty((T, B), bool))
    completed = []
    model.eval()
    for t in range(T):
        for key in ('states', 'ids', 'groups'): buffer[key][t] = getattr(episodes, key)
        raw, codes, _ = engine.observe(episodes.ids, episodes.states, episodes.groups)
        actions, logp, values = act(model, raw, rng, device)
        chosen = codes[np.arange(B), actions].copy()
        remaining, dead = engine.step(episodes.ids, episodes.states, chosen)
        episodes.steps += 1
        rewards, done = rewards_and_done(remaining, dead, episodes.steps, config)
        buffer['action'][t] = actions; buffer['old_logp'][t] = logp; buffer['value'][t] = values
        buffer['reward'][t] = rewards; buffer['done'][t] = done
        for i in np.flatnonzero(done):
            completed.append(dict(case=int(episodes.ids[i]), T=int(episodes.steps[i]), E=int(remaining[i]),
                                  start_remaining_teacher=int(episodes.starts[i]), dead=bool(dead[i])))
        episodes.reset(np.flatnonzero(done))
        if report and t % 32 == 31: report(t + 1)
    with torch.no_grad():
        raw, _, _ = engine.observe(episodes.ids, episodes.states, episodes.groups)
        _, final = model(tensors(raw, device))
        final = -final.cpu().numpy() / 100.
    adv, returns = advantages(buffer['reward'], buffer['done'], buffer['value'], final, config)
    buffer['advantages'] = (adv - adv.mean()) / max(float(adv.std()), 1e-8)
    buffer['returns'] = returns
    return {k: v.reshape((T * B,) + v.shape[2:]) for k, v in buffer.items()}, completed


def sample_bc(data, rng, count):
    cases = [c for c in data.cases if c['role'] == 'train']
    selected = rng.integers(len(cases), size=count)
    ids = np.array([cases[i]['frame_start'] + int(rng.integers(cases[i]['frames'])) for i in selected])
    return data.batch(ids, groups=rng.integers(8, size=count))


def ppo_loss(model, raw, actions, old_logp, adv, returns, device, config):
    batch = tensors(raw, device); logits, costs = model(batch)
    log_distribution = F.log_softmax(logits, -1)
    selected = torch.as_tensor(actions, device=device)
    logp = log_distribution.gather(-1, selected[:, None]).squeeze(-1)
    old = torch.as_tensor(old_logp, device=device); advantage = torch.as_tensor(adv, device=device)
    target = torch.as_tensor(returns, device=device)
    change = logp - old; ratio = change.exp()
    policy = -torch.minimum(ratio * advantage, ratio.clamp(1. - config['clip'], 1. + config['clip']) * advantage).mean()
    value = F.smooth_l1_loss(-costs / 100., target)
    entropy = -(log_distribution.exp() * log_distribution).sum(-1).mean()
    kl = (ratio - 1. - change).mean()
    clip_fraction = ((ratio - 1.).abs() > config['clip']).float().mean()
    loss = policy + config['value_coef'] * value - config['entropy_coef'] * entropy
    stats = torch.stack((policy.detach(), value.detach(), entropy.detach(), kl.detach(), clip_fraction.detach()))
    return loss, stats


def optimize(model, optimizer, engine, data, buffer, rng, device, config, progress=0., report=None):
    model.train(); sums = np.zeros(7); updates = 0; kl_stop = False
    lr = config['initial_lr'] + progress * (config['final_lr'] - config['initial_lr'])
    optimizer.param_groups[0]['lr'] = lr
    for epoch in range(config['epochs']):
        order = rng.permutation(len(buffer['ids']))
        for offset in range(0, len(order), config['minibatch']):
            ids = order[offset:offset + config['minibatch']]
            raw, _, _ = engine.observe(buffer['ids'][ids], buffer['states'][ids], buffer['groups'][ids])
            loss, stats = ppo_loss(model, raw, *(buffer[k][ids] for k in ('action', 'old_logp', 'advantages', 'returns')),
                                   device, config)
            numbers = stats.cpu().numpy()
            assert np.isfinite(numbers).all()
            if numbers[3] > config['target_kl']:
                kl_stop = True; break
            bc = tensors(sample_bc(data, rng, config['bc_batch']), device)
            bc_logits, _ = model(bc)
            imitation = F.cross_entropy(bc_logits, bc['target'])
            loss = loss + config['bc_coef'] * imitation
            optimizer.zero_grad(set_to_none=True); loss.backward()
            grad = torch.nn.utils.clip_grad_norm_(model.parameters(), config['gradient_clip'], error_if_nonfinite=True)
            optimizer.step()
            sums[:5] += numbers; sums[5] += float(imitation.detach()); sums[6] += float(grad)
            updates += 1
            if report and updates % 32 == 0: report(updates)
        if kl_stop: break
    assert updates > 0, 'no PPO update: old and current policies should agree before the first update'
    result = dict(zip(('policy_loss', 'value_loss', 'entropy', 'approx_kl', 'clip_fraction', 'bc_ce', 'gradient_norm'),
                      (sums / updates).tolist()))
    result.update(updates=updates, kl_stopped=kl_stop, learning_rate=lr)
    return result


def export(model, path, iteration, steps):
    save(path, dict(parameters={k: v.detach().cpu().tolist() for k, v in model.state_dict().items()},
                    spec=model.spec, size='small', epoch=iteration, rl_iterations=iteration, environment_steps=steps,
                    dataset_sha256=sha(BC_RUN / 'data/dataset.json'), initial_model_sha256=sha(BC_RUN / 'models/small/model.json'),
                    seed=CONFIG['seed'], method='PPO with BC auxiliary'))


def train(root, device, mechanism=False):
    config = dict(CONFIG)
    if mechanism: config['iterations'] = 2; config['seconds'] = 1200
    directory = root / ('mechanism_training' if mechanism else 'training'); directory.mkdir(parents=True, exist_ok=True)
    if (directory / 'result.json').exists(): return load(directory / 'result.json')
    data = Dataset(BC_RUN / 'data'); fingerprint = sha(BC_RUN / 'data/dataset.json')
    initial_success = load(BC_RUN / 'evaluation/small/comparison.json')['initial']['stochastic']['completion_rate']
    curriculum = initial_success < config['curriculum_threshold']
    identity = dict(config=config, dataset_sha256=fingerprint, initial_model_sha256=sha(BC_RUN / 'models/small/model.json'),
                    curriculum=curriculum, device=device)
    if (directory / 'config.json').exists(): assert load(directory / 'config.json') == identity
    else: save(directory / 'config.json', identity)
    rng = np.random.default_rng(config['seed']); torch.manual_seed(config['seed'])
    if device == 'mps': torch.mps.manual_seed(config['seed'])
    model = initial_model(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=config['initial_lr'], weight_decay=config['weight_decay'])
    engine = Engine(build_library(root / 'environment'), data, config['workers'])
    episodes = Episodes(data, rng, config['environments'], curriculum)
    iteration = steps = 0; previous_seconds = 0.; history = []; episode_count = 0; successes = 0
    if (directory / 'latest.pt').exists():
        saved = torch.load(directory / 'latest.pt', map_location='cpu', weights_only=False)
        assert saved['identity'] == identity
        model.load_state_dict(saved['model']); optimizer.load_state_dict(saved['optimizer'])
        episodes.restore(saved['episodes']); rng.bit_generator.state = saved['rng']
        torch.set_rng_state(saved['torch_rng'])
        if device == 'mps': torch.mps.set_rng_state(saved['mps_rng'])
        iteration, steps, previous_seconds, history, episode_count, successes = (
            saved[k] for k in ('iteration', 'steps', 'seconds', 'history', 'episode_count', 'successes'))
        del saved
    started = time.monotonic(); stopped = False; last_report = 0.
    def stop(signum, frame):
        nonlocal stopped
        stopped = True
    signal.signal(signal.SIGTERM, stop); signal.signal(signal.SIGINT, stop)
    def seconds(): return previous_seconds + time.monotonic() - started
    def persist():
        checkpoint(directory / 'latest.pt', dict(identity=identity, model={k: v.detach().cpu() for k, v in model.state_dict().items()},
                   optimizer=optimizer.state_dict(), episodes=episodes.state_dict(), rng=rng.bit_generator.state,
                   torch_rng=torch.get_rng_state(), mps_rng=torch.mps.get_rng_state() if device == 'mps' else None,
                   iteration=iteration, steps=steps, seconds=seconds(), history=history, episode_count=episode_count, successes=successes))
    def report(phase, part):
        nonlocal last_report
        elapsed = seconds()
        if time.monotonic() - last_report < 30: return
        rate = (iteration - starting_iteration) / max(time.monotonic() - started, .001)
        remaining = min(config['seconds'] - elapsed, (config['iterations'] - iteration) / rate if rate else config['seconds'])
        status(directory, phase, iteration=iteration, steps=steps, part=part, seconds=elapsed,
               remaining_seconds=max(0., remaining), episodes=episode_count, successes=successes,
               device=str(next(model.parameters()).device))
        last_report = time.monotonic()
    starting_iteration = iteration; reason = 'iteration_budget'
    try:
        while iteration < config['iterations']:
            if stopped: persist(); raise InterruptedError('training interrupted at a checkpoint boundary')
            if seconds() >= config['seconds']: reason = 'time_budget'; break
            if not mechanism and datetime.now().astimezone() >= datetime.fromisoformat(config['final_deadline']):
                reason = 'deadline'; break
            tick = time.monotonic()
            buffer, finished = collect(model, engine, episodes, rng, device, config, lambda n: report('collecting', n))
            rollout_seconds = time.monotonic() - tick; tick = time.monotonic()
            progress = min(1., max(iteration / config['iterations'], seconds() / config['seconds']))
            metrics = optimize(model, optimizer, engine, data, buffer, rng, device, config, progress,
                               lambda n: report('updating', n))
            update_seconds = time.monotonic() - tick
            iteration += 1; steps += len(buffer['ids']); episode_count += len(finished)
            complete = [r for r in finished if r['E'] == 0]; successes += len(complete)
            row = dict(iteration=iteration, steps=steps, seconds=seconds(), rollout_seconds=rollout_seconds,
                       update_seconds=update_seconds, episodes=len(finished), successes=len(complete),
                       mean_T_completed=float(np.mean([r['T'] for r in complete])) if complete else None,
                       mean_E_finished=float(np.mean([r['E'] for r in finished])) if finished else None,
                       reward_mean=float(buffer['reward'].mean()), **metrics)
            history.append(row)
            # チェックポイントを正本とする。JSONLは中断時に末尾が先行しても復元できる。
            persist()
            with (directory / 'metrics.jsonl').open('a') as stream: stream.write(json.dumps(row, allow_nan=False) + '\n')
            with (directory / 'episodes.jsonl').open('a') as stream:
                for item in finished: stream.write(json.dumps(dict(iteration=iteration, **item)) + '\n')
            if iteration in (512, 1024, 2048): export(model, directory / f'model_iteration{iteration:04d}.json', iteration, steps)
            status(directory, 'iteration_completed', **row,
                   remaining_seconds=min(max(0., config['seconds'] - seconds()),
                       (config['iterations'] - iteration) * (seconds() - previous_seconds) / (iteration - starting_iteration)))
            print(json.dumps(row, allow_nan=False), flush=True)
            del buffer
        assert iteration > 0, 'no training completed'
        persist(); export(model, directory / 'model.json', iteration, steps)
        result = dict(iterations=iteration, environment_steps=steps, seconds=seconds(), reason=reason,
                      episodes=episode_count, successes=successes, history=history, final=history[-1],
                      checkpoint_sha256=sha(directory / 'latest.pt'), model_sha256=sha(directory / 'model.json'), completed_at=now())
        save(directory / 'result.json', result); status(directory, 'training_completed', iterations=iteration, steps=steps, reason=reason)
        return result
    finally:
        engine.close()


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN)
    p.add_argument('--device', choices=('cpu', 'mps'), default='mps'); p.add_argument('--mechanism', action='store_true')
    args = p.parse_args(); args.run = args.run.resolve()
    torch.set_num_threads(2); torch.set_num_interop_threads(2)
    if args.device == 'mps': torch.mps.set_per_process_memory_fraction(min(1., 16e9 / torch.mps.recommended_max_memory()))
    reporter = GPUReport(args.run / 'gpu_state.json') if args.device == 'mps' else None
    try: train(args.run, args.device, args.mechanism)
    finally:
        if reporter: reporter.close()
