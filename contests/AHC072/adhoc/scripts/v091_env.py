#!/usr/bin/env python3
"""C++の盤面環境と、再開可能な学習入力だけの並列エピソード。"""
import ctypes as ct
import json
import os
from pathlib import Path
import platform
import subprocess

import numpy as np

from v090_data import ROOT, RUN as BC_RUN, Dataset, MODEL_SPECS, save, sha, now

RUN = ROOT / 'results/nn_rank/v091/20261003_ppo_studio'
CONFIG = dict(seed=91002, environments=128, horizon=128, iterations=2048, seconds=43200,
              final_deadline='2026-10-04T16:00:00+09:00', max_episode_steps=2048,
              gamma=1., gae_lambda=.98, clip=.2, epochs=4, minibatch=256,
              value_coef=.5, entropy_coef=.001, bc_coef=.05, bc_batch=64,
              target_kl=.03, initial_lr=3e-5, final_lr=3e-6, weight_decay=.0001,
              gradient_clip=1., step_reward=-.01, failure_penalty=30., workers=8,
              model_spec=MODEL_SPECS['small'], curriculum_threshold=.8)


def load(path):
    return json.loads(path.read_text())


def build_library(directory):
    directory.mkdir(parents=True, exist_ok=True)
    binary = directory / ('environment.dylib' if platform.system() == 'Darwin' else 'environment.so')
    sources = [ROOT / 'adhoc/bin/v091_environment.cpp', ROOT / 'adhoc/scripts/v089_core.cpp.txt']
    identity = {str(p.relative_to(ROOT)): sha(p) for p in sources}
    marker = directory / 'environment_build.json'
    if marker.exists():
        previous = load(marker)
        assert previous['sources'] == identity and previous['binary_sha256'] == sha(binary)
        return binary
    env = os.environ.copy()
    if platform.system() == 'Darwin':
        env.setdefault('MACOSX_DEPLOYMENT_TARGET', '15.0')
        env.setdefault('SDKROOT', subprocess.check_output(['xcrun', '--show-sdk-path'], text=True).strip())
    command = [env.get('CXX', 'g++-15'), '-std=gnu++23', '-O2', '-march=native', '-pthread',
               '-fopenmp', '-fPIC', '-shared', '-Wall', '-Wextra', str(sources[0]), '-o', str(binary)]
    subprocess.run(command, check=True, env=env)
    save(marker, dict(sources=identity, binary_sha256=sha(binary), command=command, built_at=now()))
    return binary


class Engine:
    def __init__(self, binary, data, workers=8):
        self.lib = ct.CDLL(str(binary)); self.data = data; self.workers = workers
        self.handles = {}; self.buffers = {}
        self.lib.v091_create.argtypes = [ct.c_char_p]; self.lib.v091_create.restype = ct.c_void_p
        self.lib.v091_destroy.argtypes = [ct.c_void_p]
        self.lib.v091_error.restype = ct.c_char_p
        self.lib.v091_observe.argtypes = [ct.c_int, ct.c_void_p, ct.c_void_p, ct.c_void_p, ct.c_int] + [ct.c_void_p] * 7 + [ct.c_int]
        self.lib.v091_observe.restype = ct.c_int
        self.lib.v091_step.argtypes = [ct.c_int] + [ct.c_void_p] * 5 + [ct.c_int]
        self.lib.v091_step.restype = ct.c_int

    def pointers(self, ids):
        for cid in np.unique(ids):
            cid = int(cid)
            if cid in self.handles: continue
            case = self.data.by_index[cid]
            assert case['role'] == 'train', 'RL may only access training geometries'
            path = BC_RUN / case['path']; assert sha(path) == case['sha256']
            handle = self.lib.v091_create(os.fsencode(path))
            if not handle: raise RuntimeError(self.lib.v091_error().decode())
            self.handles[cid] = handle
        return np.array([self.handles[int(i)] for i in ids], np.uintp)

    def observe(self, ids, states, groups):
        n = len(ids); states = np.ascontiguousarray(states, np.uint32)
        groups = np.ascontiguousarray(groups, np.int32); pointers = self.pointers(ids)
        # 1匹当たりの合法手は最大18個、全400マスに1匹でも7,200以下。
        capacity = 8192
        if n not in self.buffers:
            self.buffers[n] = dict(x=np.zeros((n, 40, 20, 20), np.float32),
                src=np.zeros((n, capacity), np.int64), dst=np.zeros((n, capacity), np.int64),
                features=np.zeros((n, capacity, 16), np.float32), codes=np.zeros((n, capacity), np.uint32),
                counts=np.zeros(n, np.int32), remaining=np.zeros(n, np.int32))
        b = self.buffers[n]
        args = [b[k].ctypes.data for k in ('x', 'src', 'dst', 'features', 'codes', 'counts', 'remaining')]
        result = self.lib.v091_observe(n, pointers.ctypes.data, states.ctypes.data, groups.ctypes.data,
                                      capacity, *args, self.workers)
        if result: raise RuntimeError(self.lib.v091_error().decode())
        if np.any(b['counts'] == 0): raise RuntimeError('terminal state passed to policy')
        length = int(b['counts'].max())
        raw = {k: b[k] if k == 'x' else b[k][:, :length] for k in ('x', 'src', 'dst', 'features')}
        raw['valid'] = np.arange(length)[None] < b['counts'][:, None]
        return raw, b['codes'][:, :length], b['counts'].copy()

    def step(self, ids, states, codes):
        assert states.dtype == np.uint32 and states.flags.c_contiguous
        codes = np.ascontiguousarray(codes, np.uint32); pointers = self.pointers(ids)
        remaining = np.empty(len(ids), np.int32); dead = np.empty(len(ids), np.int32)
        result = self.lib.v091_step(len(ids), pointers.ctypes.data, states.ctypes.data, codes.ctypes.data,
                                   remaining.ctypes.data, dead.ctypes.data, self.workers)
        if result: raise RuntimeError(self.lib.v091_error().decode())
        return remaining, dead.astype(bool)

    def close(self):
        for handle in self.handles.values(): self.lib.v091_destroy(handle)
        self.handles.clear()


class Episodes:
    def __init__(self, data, rng, size, curriculum):
        self.data, self.rng, self.size, self.curriculum = data, rng, size, curriculum
        self.cases = [c for c in data.cases if c['role'] == 'train']
        assert len(self.cases) == 4096
        self.ids = np.zeros(size, np.int32); self.groups = np.zeros(size, np.int32)
        self.states = np.zeros((size, 400), np.uint32); self.steps = np.zeros(size, np.int32)
        self.starts = np.zeros(size, np.int32)
        self.reset(np.arange(size))

    def reset(self, slots):
        for i in slots:
            case = self.cases[int(self.rng.integers(len(self.cases)))]; offset = 0
            if self.curriculum:
                choice = self.rng.random()
                if choice >= .5: offset = max(0, case['frames'] - (50 if choice < .75 else 10))
            self.ids[i] = case['index']; self.groups[i] = self.rng.integers(8)
            self.states[i] = self.data.states[case['frame_start'] + offset]
            self.steps[i] = 0; self.starts[i] = case['frames'] - offset

    def state_dict(self):
        return {k: getattr(self, k).copy() for k in ('ids', 'groups', 'states', 'steps', 'starts')}

    def restore(self, values):
        for k, v in values.items():
            assert v.shape == getattr(self, k).shape
            getattr(self, k)[:] = v
        assert all(self.data.by_index[int(i)]['role'] == 'train' for i in self.ids)


def rewards_and_done(remaining, dead, steps, config=CONFIG):
    done = (remaining == 0) | dead | (steps >= config['max_episode_steps'])
    reward = np.full(len(remaining), config['step_reward'], np.float32)
    failure = done & (remaining > 0)
    reward[failure] -= config['failure_penalty'] + remaining[failure] / 256.
    return reward, done


def advantages(rewards, dones, values, final_values, config=CONFIG):
    out = np.zeros_like(rewards); following = final_values.copy(); running = np.zeros_like(final_values)
    for t in range(len(rewards) - 1, -1, -1):
        alive = 1. - dones[t].astype(np.float32)
        delta = rewards[t] + config['gamma'] * following * alive - values[t]
        running = delta + config['gamma'] * config['gae_lambda'] * alive * running
        out[t] = running; following = values[t]
    return out, out + values
