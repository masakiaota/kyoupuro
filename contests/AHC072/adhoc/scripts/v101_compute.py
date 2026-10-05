#!/usr/bin/env python3
"""合法手と損失を保ち、GPUが扱う配列形状を少数種類にまとめる。"""
import numpy as np
import torch
import torch.nn.functional as F
from train_v090_board import tensors


def padded(raw):
    n, length = raw['valid'].shape
    b = 1 << (n - 1).bit_length()
    size = max(32, 1 << (length - 1).bit_length())
    result = {}
    for key in ('x', 'src', 'dst', 'features', 'valid'):
        value = raw[key]
        shape = (b,) + (value.shape[1:] if key == 'x' else (size,) + value.shape[2:])
        result[key] = np.zeros(shape, value.dtype)
        if key == 'x': result[key][:n] = value
        else: result[key][:n, :length] = value
    # 余分な盤面もpoolingの分母とsoftmaxを有限に保つ。損失には含めない。
    if b > n:
        result['x'][n:] = raw['x'][0]
        result['valid'][n:, 0] = True
    return result


def distribution(model, raw, device, bucket=True):
    data = padded(raw) if bucket else raw
    batch = tensors({k: data[k] for k in ('x', 'src', 'dst', 'features', 'valid')}, device)
    logits, value = model(batch)
    # softmaxも埋めた形状で実行し、実在する盤面だけを返す。
    return F.log_softmax(logits, -1)[:len(raw['x'])], value[:len(raw['x'])]


@torch.no_grad()
def act(model, raw, rng, device):
    logp, value = distribution(model, raw, device)
    logp = logp.cpu().numpy()
    probabilities = np.exp(logp.astype(np.float64))
    cumulative = np.cumsum(probabilities, axis=-1)
    cumulative /= cumulative[:, -1:]
    cumulative[:, -1] = 1.
    action = (cumulative < rng.random((len(logp), 1))).sum(-1).astype(np.int64)
    assert (action < raw['valid'].shape[1]).all()
    assert raw['valid'][np.arange(len(action)), action].all()
    return action, logp[np.arange(len(action)), action], -value.cpu().numpy() / 100.


def loss(model, raw, actions, old_logp, adv, returns, device, config, scale, bucket=True):
    log_distribution, costs = distribution(model, raw, device, bucket)
    selected = torch.as_tensor(actions, device=device)
    logp = log_distribution.gather(-1, selected[:, None]).squeeze(-1)
    # 診断では同じ固定重みの勾配だけを測る。本学習は採取時の確率を渡す。
    old = logp.detach() if old_logp is None else torch.as_tensor(old_logp, device=device)
    change = logp - old
    ratio = change.exp()
    advantage = torch.as_tensor(adv, device=device)
    policy = -torch.minimum(ratio * advantage, ratio.clamp(1.-config['clip'], 1.+config['clip']) * advantage).mean() * scale
    value = (F.smooth_l1_loss(-costs / 100., torch.as_tensor(returns, device=device))
             if config['method'] == 'mc_ppo' else torch.zeros((), device=device))
    entropy = -(log_distribution.exp() * log_distribution).sum(-1).mean()
    kl = (ratio - 1. - change).mean()
    clipped = ((ratio - 1.).abs() > config['clip']).float().mean()
    total = policy + config['value_coef'] * value - config['entropy_coef'] * entropy
    return total, torch.stack([x.detach() for x in (policy, value, entropy, kl, clipped)])


def imitation(model, raw, device, bucket=True):
    logp, _ = distribution(model, raw, device, bucket)
    return F.nll_loss(logp, torch.as_tensor(raw['target'], device=device))


def owned(raw):
    """C++の再利用配列を次のobserveから切り離す。"""
    return {k: np.array(v, copy=True, order='C') for k, v in raw.items()}
