#!/usr/bin/env python3
"""v099の完走採取を保ち、形状統一と1バッチ先読みで群内比較を更新する。"""
from concurrent.futures import ThreadPoolExecutor
import numpy as np
import torch
from v092_stream import describe
from v099_complete import measured_targets, AuxiliaryPool
from v101_compute import act, loss, imitation, owned


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


def optimize(model, optimizer, engine, data, buffer, shuffle_rng, auxiliary_rng, device, config, progress, report=None):
    model.train(); engine.buffers.clear()
    pool = AuxiliaryPool(data, engine, auxiliary_rng, config)
    lr = config['initial_lr'] + progress * (config['final_lr']-config['initial_lr'])
    optimizer.param_groups[0]['lr'] = lr
    scale = len(buffer['ids'])/(config['environments']*config['loss_reference_steps'])
    sums = np.zeros(8); updates = 0; kl_stop = False
    def prepare(indexes):
        # この1スレッドだけがC++の再利用配列と教師抽出乱数に触る。
        raw, _, _ = engine.observe(buffer['ids'][indexes], buffer['states'][indexes], buffer['groups'][indexes])
        raw = owned(raw)
        bc = owned(pool.original()) if config['bc_coef'] else None
        return indexes, raw, bc
    with ThreadPoolExecutor(max_workers=1) as executor:
        for epoch in range(config['epochs']):
            order = shuffle_rng.permutation(len(buffer['ids']))
            partitions = [order[i:i+config['minibatch']] for i in range(0,len(order),config['minibatch'])]
            future = executor.submit(prepare,partitions[0])
            for position in range(len(partitions)):
                indexes, raw, bc = future.result()
                if position+1<len(partitions): future=executor.submit(prepare,partitions[position+1])
                total, stats = loss(model,raw,*(buffer[k][indexes] for k in ('action','old_logp','advantages','returns')),
                                    device,config,scale)
                numbers=stats.cpu().numpy();assert np.isfinite(numbers).all()
                if numbers[3]>config['target_kl']: kl_stop=True;break
                auxiliary = imitation(model,bc,device) if bc is not None else torch.zeros((),device=device)
                total = total + config['bc_coef']*auxiliary
                optimizer.zero_grad(set_to_none=True);total.backward()
                gradient=torch.nn.utils.clip_grad_norm_(model.parameters(),config['gradient_clip'],error_if_nonfinite=True)
                optimizer.step();updates+=1
                sums[:5]+=numbers;sums[5:]+=[float(auxiliary.detach()),0.,float(gradient)]
                if report and updates%32==0: report(updates)
            if kl_stop: break
    # executorを閉じてから乱数と環境を保存する。先読みは最大1バッチ。
    assert updates>0, 'initial policy must match its recorded action probabilities'
    result=dict(zip(('policy_loss','value_loss','entropy','approx_kl','clip_fraction','bc_ce','reverse_ce','gradient_norm'),
                    (sums/updates).tolist()))
    return dict(result,updates=updates,kl_stopped=kl_stop,learning_rate=lr,policy_loss_scale=scale,reverse_samples=0,
                bc_samples=updates*config['bc_batch'] if config['bc_coef'] else 0)
