#!/usr/bin/env python3
"""完走列を固定方策で採取し、実測リターンまたは他の列との差で更新する。"""
from pathlib import Path
import time

import numpy as np
import torch
import torch.nn.functional as F

from train_v090_board import Model, tensors
from train_v091_ppo import act
from v091_env import CONFIG as OLD_CONFIG, load
from v092_stream import ROOT, BC_RUN, FreshEngine, describe
from v096_selected import ReverseEngine, ReverseData

RUN = ROOT / 'results/nn_rank/v099/20261003_complete_studio'
REVERSE_RUN = ROOT / 'results/nn_rank/v100/20261004_complete_reverse'
PARENT = ROOT / 'results/nn_rank/v092/20261003_fresh_studio'
INITIAL = PARENT / 'transition_full_episode_20261003/training/latest.pt'
INITIAL_SHA = 'c87b59b7926773dceeee98f0e394871052aa82b0109eb24591689595abc5859d'
REVERSE_SOURCE = ROOT / 'results/nn_rank/v096/20261003_selected_studio'


def configuration(method, reverse=False, mixed=False):
    assert method in ('mc_ppo', 'group') and (not mixed or reverse)
    config = dict(OLD_CONFIG, method=method, seed=100003 if reverse else 99003,
                  seed_base=1000000000000 if reverse else 990000000000,
                  seed_stride=64, boards=32, repetitions=4, environments=128,
                  iterations=128, seconds=2700, generator_workers=2, queue_capacity=64,
                  bc_cache_capacity=128, bc_pool_cases=64, reverse_pool_states=64,
                  loss_reference_steps=400, initial_lr=1e-5 if reverse else 2e-5,
                  final_lr=3e-6, final_deadline='2026-10-04T18:00:00+09:00',
                  bc_batch=32 if mixed else 64, bc_coef=.025 if mixed else .05,
                  reverse_batch=32 if mixed else 0, reverse_coef=.025 if mixed else 0.)
    for key in ('horizon', 'gae_lambda', 'curriculum_threshold'):
        config.pop(key)
    return config


def start_model(path, device, config):
    saved = torch.load(path, map_location='cpu', weights_only=False)
    model = Model(config['model_spec']).to(device)
    model.load_state_dict(saved['model'])
    if config['method'] == 'group':
        for parameter in model.critic.parameters(): parameter.requires_grad_(False)
    optimizer = torch.optim.AdamW([p for p in model.parameters() if p.requires_grad],
                                  lr=config['initial_lr'], weight_decay=config['weight_decay'])
    return model, optimizer


def measured_targets(lengths, remaining, episode, times, values, repetitions, method):
    lengths, remaining = np.asarray(lengths), np.asarray(remaining)
    penalty = np.where(remaining > 0, 30. + remaining / 256., 0.)
    costs = lengths / 100. + penalty
    returns = -(lengths[episode] - times) / 100. - penalty[episode]
    grouped = costs.reshape(-1, repetitions)
    relative = ((grouped.sum(1, keepdims=True) - grouped) / (repetitions - 1) - grouped).reshape(-1)
    advantages = returns - values if method == 'mc_ppo' else relative[episode]
    return returns.astype(np.float32), advantages.astype(np.float32), costs, relative


def collect(model, engine, queue, action_rng, transform_rng, device, config, report=None):
    """更新や盤面補充を途中に挟まず、全列の終端まで採取する。"""
    items = [queue.take() for _ in range(config['boards'])]
    for item in items: engine.register(item)
    repetitions = config['repetitions']; size = len(items) * repetitions
    ids = np.repeat(np.array([x['id'] for x in items], np.int32), repetitions)
    groups = np.repeat(transform_rng.integers(8, size=len(items), dtype=np.int32), repetitions)
    states = np.stack([describe(x['text'])[0] for x in items]).repeat(repetitions, axis=0)
    active = np.ones(size, bool); lengths = np.zeros(size, np.int32)
    final_remaining = np.full(size, -1, np.int32); actions_by_episode = [[] for _ in ids]
    parts = {k: [] for k in ('states', 'ids', 'groups', 'action', 'old_logp', 'value', 'episode', 'time')}
    previous_size = None; model.eval()
    for t in range(config['max_episode_steps']):
        slots = np.flatnonzero(active)
        if not len(slots): break
        # 可変batch全サイズの巨大な合法手配列を保持しない。
        if len(slots) != previous_size: engine.buffers.clear(); previous_size = len(slots)
        current = np.ascontiguousarray(states[slots]); current_ids = ids[slots]; current_groups = groups[slots]
        raw, codes, _ = engine.observe(current_ids, current, current_groups)
        chosen, logp, value = act(model, raw, action_rng, device)
        chosen_codes = codes[np.arange(len(slots)), chosen].copy()
        for key, value_array in dict(states=current.copy(), ids=current_ids, groups=current_groups,
                                     action=chosen, old_logp=logp, value=value, episode=slots,
                                     time=np.full(len(slots), t, np.int32)).items():
            parts[key].append(value_array)
        left, dead = engine.step(current_ids, current, chosen_codes)
        states[slots] = current; lengths[slots] += 1
        for slot, code in zip(slots, chosen_codes): actions_by_episode[int(slot)].append(int(code))
        done = (left == 0) | dead | (lengths[slots] >= config['max_episode_steps'])
        ended = slots[done]; final_remaining[ended] = left[done]; active[ended] = False
        if report and t % 64 == 63: report(t + 1, int((~active).sum()))
    assert not active.any() and (final_remaining >= 0).all()
    buffer = {k: np.concatenate(v) for k, v in parts.items()}
    del parts
    returns, advantages, costs, relative = measured_targets(lengths, final_remaining, buffer['episode'],
                        buffer['time'], buffer['value'], repetitions, config['method'])
    buffer.update(returns=returns, advantages=advantages)
    rows = [dict(episode=i, case=int(ids[i]), seed=items[i // repetitions]['seed'], group=int(groups[i]),
                 repetition=i % repetitions, T=int(lengths[i]), E=int(final_remaining[i]),
                 cost=float(costs[i]), group_advantage=float(relative[i]), actions=actions_by_episode[i])
            for i in range(size)]
    return buffer, rows, states, items


class SelectedReverse(ReverseData):
    def __init__(self, selection_root):
        super().__init__(REVERSE_SOURCE)
        selected = load(selection_root / 'teachers/selection.json')
        assert selected['training_eligible']
        self.selected = np.asarray(selected['selected_indexes'], np.int64)
        self.weights = np.asarray(selected['weights'], np.float32)
        assert len(self.selected) == len(self.weights) and np.isclose(self.weights.mean(), 1.)


class AuxiliaryPool:
    """更新中に使う地形を絞る。採取ごとに再抽出し、全教師を抽出対象に保つ。"""
    def __init__(self, data, engine, rng, config):
        self.data, self.engine, self.rng, self.config = data, engine, rng, config
        all_cases = [c for c in data.cases if c['role'] == 'train']
        indexes = rng.choice(len(all_cases), size=min(len(all_cases), config['bc_pool_cases']), replace=False)
        self.cases = [all_cases[i] for i in indexes]
        self.reverse_ids = (rng.integers(len(engine.reverse.selected), size=config['reverse_pool_states'])
                            if config['reverse_batch'] else None)

    def original(self):
        size = self.config['bc_batch']; rng = self.rng
        cases = [self.cases[i] for i in rng.integers(len(self.cases), size=size)]
        frames = np.array([c['frame_start'] + rng.integers(c['frames']) for c in cases], np.int64)
        return self.data.batch(frames, groups=rng.integers(8, size=size))

    def reverse(self):
        reverse = self.engine.reverse; size = self.config['reverse_batch']; rng = self.rng
        pool_indexes = self.reverse_ids[rng.integers(len(self.reverse_ids), size=size)]
        indexes = reverse.selected[pool_indexes]
        raw, codes, _ = self.engine.observe(-1-np.asarray(reverse.case_ids[indexes], np.int32),
                                           np.asarray(reverse.states[indexes]), rng.integers(8, size=size))
        match = (codes == reverse.actions[indexes, None]) & raw['valid']
        assert (match.sum(1) == 1).all()
        return dict(raw, target=match.argmax(1).astype(np.int64)), reverse.weights[pool_indexes]


def loss(model, raw, actions, old_logp, adv, returns, device, config, scale):
    batch = tensors(raw, device)
    if device == 'cpu': batch = {k: v.clone() for k, v in batch.items()}
    logits, costs = model(batch); log_distribution = F.log_softmax(logits, -1)
    selected = torch.as_tensor(actions, device=device)
    logp = log_distribution.gather(-1, selected[:, None]).squeeze(-1)
    change = logp - torch.as_tensor(old_logp, device=device); ratio = change.exp()
    advantage = torch.as_tensor(adv, device=device)
    policy = -torch.minimum(ratio * advantage, ratio.clamp(1.-config['clip'], 1.+config['clip']) * advantage).mean() * scale
    value = (F.smooth_l1_loss(-costs / 100., torch.as_tensor(returns, device=device))
             if config['method'] == 'mc_ppo' else torch.zeros((), device=device))
    entropy = -(log_distribution.exp() * log_distribution).sum(-1).mean()
    kl = (ratio - 1. - change).mean()
    clipped = ((ratio - 1.).abs() > config['clip']).float().mean()
    total = policy + config['value_coef'] * value - config['entropy_coef'] * entropy
    return total, torch.stack([x.detach() for x in (policy, value, entropy, kl, clipped)])


def optimize(model, optimizer, engine, data, buffer, shuffle_rng, auxiliary_rng, device, config, progress, report=None):
    model.train(); engine.buffers.clear()
    pool = AuxiliaryPool(data, engine, auxiliary_rng, config)
    lr = config['initial_lr'] + progress * (config['final_lr'] - config['initial_lr'])
    optimizer.param_groups[0]['lr'] = lr
    scale = len(buffer['ids']) / (config['environments'] * config['loss_reference_steps'])
    sums = np.zeros(8); updates = 0; kl_stop = False
    for epoch in range(config['epochs']):
        order = shuffle_rng.permutation(len(buffer['ids']))
        for start in range(0, len(order), config['minibatch']):
            indexes = order[start:start + config['minibatch']]
            raw, _, _ = engine.observe(buffer['ids'][indexes], buffer['states'][indexes], buffer['groups'][indexes])
            total, stats = loss(model, raw, *(buffer[k][indexes] for k in ('action', 'old_logp', 'advantages', 'returns')),
                                device, config, scale)
            numbers = stats.cpu().numpy(); assert np.isfinite(numbers).all()
            if numbers[3] > config['target_kl']: kl_stop = True; break
            bc = tensors(pool.original(), device)
            if device == 'cpu': bc = {k: v.clone() for k, v in bc.items()}
            logits, _ = model(bc); imitation = F.cross_entropy(logits, bc['target'])
            total = total + config['bc_coef'] * imitation
            reverse_loss = torch.zeros((), device=device)
            if config['reverse_batch']:
                raw_reverse, weight = pool.reverse(); reverse_batch = tensors(raw_reverse, device)
                if device == 'cpu': reverse_batch = {k: v.clone() for k, v in reverse_batch.items()}
                reverse_logits, _ = model(reverse_batch)
                reverse_loss = (F.cross_entropy(reverse_logits, reverse_batch['target'], reduction='none') *
                                torch.as_tensor(weight, device=device)).mean()
                total = total + config['reverse_coef'] * reverse_loss
            optimizer.zero_grad(set_to_none=True); total.backward()
            grad = torch.nn.utils.clip_grad_norm_(model.parameters(), config['gradient_clip'], error_if_nonfinite=True)
            optimizer.step(); updates += 1
            sums[:5] += numbers; sums[5:] += [float(imitation.detach()), float(reverse_loss.detach()), float(grad)]
            if report and updates % 32 == 0: report(updates)
        if kl_stop: break
    assert updates > 0, 'initial policy must match its recorded action probabilities'
    result = dict(zip(('policy_loss', 'value_loss', 'entropy', 'approx_kl', 'clip_fraction', 'bc_ce', 'reverse_ce', 'gradient_norm'),
                      (sums / updates).tolist()))
    return dict(result, updates=updates, kl_stopped=kl_stop, learning_rate=lr, policy_loss_scale=scale,
                reverse_samples=updates * config['reverse_batch'])
