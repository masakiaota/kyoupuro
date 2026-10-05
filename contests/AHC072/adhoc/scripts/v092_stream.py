#!/usr/bin/env python3
"""公式入力の有界先読み、C++地形の寿命管理、旧教師の必要時計算。"""
from collections import OrderedDict
import ctypes as ct
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import time

import numpy as np

from v090_data import Dataset, save, sha, now
from v091_env import Engine, BC_RUN, ROOT, CONFIG as PPO_CONFIG, load, build_library

RUN = ROOT / 'results/nn_rank/v092/20261003_fresh_studio'
CONFIG = dict(PPO_CONFIG, seed=92002, iterations=1536, seconds=21600,
              final_deadline='2026-10-04T12:00:00+09:00', initial_lr=2e-5,
              generator_workers=2, queue_capacity=256, seed_base=920000000000, seed_stride=64,
              bc_cache_capacity=128)
LIVE_BASE = 1000000


def normalized(text):
    lines = text.splitlines(); N, K = map(int, lines[0].split())
    return f'{N} {K}\n' + '\n'.join(s.strip() for s in lines[1:N+1]) + '\n'


def digest(text): return hashlib.sha256(text.encode()).hexdigest()


def excluded_inputs():
    blocked = set()
    for case in load(BC_RUN / 'input_manifest.json'):
        text = (BC_RUN / case['path']).read_text(); blocked.update((digest(text), digest(normalized(text))))
    for path in (ROOT / 'tools/in').glob('*.txt'):
        text = path.read_text(); blocked.update((digest(text), digest(normalized(text))))
    return blocked


def describe(text):
    lines = text.splitlines(); N, K = map(int, lines[0].split()); C = lines[1:N+1]
    state = np.zeros(400, np.uint32); colors = [0]*K
    for i, row in enumerate(C):
        for j, c in enumerate(row):
            if 'a' <= c <= 'l':
                state[i*20+j] = ord(c)-96; colors[ord(c)-97] += 1
    return state, dict(N=N, K=K, M=sum(colors), color_counts=colors, walls=sum(row.count('#') for row in C))


class FreshEngine(Engine):
    def __init__(self, binary, data, workers=8):
        super().__init__(binary, data, workers); self.handles = OrderedDict(); self.live = {}

    def from_text(self, item):
        with tempfile.TemporaryDirectory(prefix='v092-geometry-') as directory:
            path = Path(directory) / 'input.txt'; path.write_text(item['text'])
            handle = self.lib.v091_create(os.fsencode(path))
        if not handle: raise RuntimeError(self.lib.v091_error().decode())
        return dict(item, pointer=handle)

    def register(self, item):
        assert item['id'] not in self.live and item['id'] >= LIVE_BASE
        self.live[item['id']] = item

    def pointers(self, ids):
        protected = set(map(int, ids)); result = []
        for value in ids:
            cid = int(value)
            if cid >= LIVE_BASE:
                result.append(self.live[cid]['pointer']); continue
            case = self.data.by_index[cid]; assert case['role'] == 'train'
            if cid not in self.handles:
                path = BC_RUN / case['path']; assert sha(path) == case['sha256']
                handle = self.lib.v091_create(os.fsencode(path))
                if not handle: raise RuntimeError(self.lib.v091_error().decode())
                self.handles[cid] = handle
            self.handles.move_to_end(cid); result.append(self.handles[cid])
        for cid in list(self.handles):
            if len(self.handles) <= CONFIG['bc_cache_capacity']: break
            if cid not in protected: self.lib.v091_destroy(self.handles.pop(cid))
        return np.array(result, np.uintp)

    def release_except(self, retained):
        retained = set(map(int, retained))
        for cid in list(self.live):
            if cid not in retained: self.lib.v091_destroy(self.live.pop(cid)['pointer'])

    def serializable(self):
        return [{k:v for k,v in item.items() if k != 'pointer'} for item in self.live.values()]

    def close(self):
        self.release_except([]); super().close()


class TeacherData(Dataset):
    """合法手の巨大配列は読み込まず、教師状態から同じ特徴を再計算する。"""
    def batch(self, ids, epoch=0, groups=None):
        ids = np.asarray(ids, np.int64); cases = np.asarray(self.case_ids[ids], np.int32)
        if groups is None: groups = np.zeros(len(ids), np.int32)
        raw, _, counts = self.engine.observe(cases, np.asarray(self.states[ids]), groups)
        targets = np.asarray(self.targets[ids]); assert (targets < counts).all()
        return dict(raw, target=targets, value=np.asarray(self.remaining[ids]),
                    weight=(len(self.splits['train']) / self.train_case_count / self.lengths[cases]).astype(np.float32))


class OfficialQueue:
    def __init__(self, root, engine, blocked, config=CONFIG, snapshot=None):
        self.root, self.engine, self.blocked, self.config = root, engine, blocked, dict(config)
        directory = root / 'generator'; directory.mkdir(parents=True, exist_ok=True)
        self.binary = directory / 'official_gen'
        original = ROOT / 'tools/target/release/gen'
        if not self.binary.exists(): shutil.copy2(original, self.binary)
        assert sha(self.binary) == sha(original)
        self.generator_sha256 = sha(self.binary)
        self.cv = threading.Condition(); self.ready = {}; self.inflight = 0; self.paused = True
        self.stopping = False; self.error = None; self.next_ticket = self.consume_ticket = 0
        self.update_phase = True
        self.wait_seconds = 0.; self.generated = 0; self.max_outstanding = 0
        if snapshot:
            assert snapshot['generator_sha256'] == self.generator_sha256
            assert snapshot['seed_base'] == config['seed_base'] and snapshot['seed_stride'] == config['seed_stride']
            self.next_ticket = snapshot['next_ticket']; self.consume_ticket = snapshot['consume_ticket']
            for item in snapshot['ready']:
                assert item['sha256'] == digest(item['text']) and item['sha256'] not in blocked
                self.ready[item['ticket']] = engine.from_text(item)
            assert sorted(self.ready) == list(range(self.consume_ticket, self.next_ticket))
        self.threads = [threading.Thread(target=self.worker, daemon=True) for _ in range(config['generator_workers'])]
        for thread in self.threads: thread.start()
        self.resume()

    def generate(self, ticket):
        for attempt in range(self.config['seed_stride']):
            seed = self.config['seed_base'] + ticket * self.config['seed_stride'] + attempt
            with tempfile.TemporaryDirectory(prefix='v092-official-') as directory:
                directory = Path(directory); (directory / 'seed.txt').write_text(str(seed)+'\n')
                subprocess.run([self.binary, directory/'seed.txt', '--dir', directory/'inputs'],
                               stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=True, timeout=30.)
                text = normalized((directory/'inputs/0000.txt').read_text())
            fingerprint = digest(text)
            if fingerprint in self.blocked: continue
            _, statistics = describe(text)
            return self.engine.from_text(dict(id=LIVE_BASE+ticket, ticket=ticket, seed=seed, text=text,
                                              sha256=fingerprint, statistics=statistics))
        raise RuntimeError('seed range exhausted by excluded inputs')

    def worker(self):
        while True:
            with self.cv:
                # 採取中は半分を下回った場合だけ補充し、通常の生成をGPU更新中へ寄せる。
                while not self.stopping and (self.paused or len(self.ready)+self.inflight >=
                        (self.config['queue_capacity'] if self.update_phase else self.config['queue_capacity']//2)): self.cv.wait()
                if self.stopping: return
                ticket=self.next_ticket; self.next_ticket+=1; self.inflight+=1
                self.max_outstanding=max(self.max_outstanding,len(self.ready)+self.inflight)
            try:
                item=self.generate(ticket)
                with self.cv:
                    self.ready[ticket]=item; self.generated+=1
                    with (self.root/'generator/generated.jsonl').open('a') as stream:
                        import json
                        stream.write(json.dumps({k:v for k,v in item.items() if k not in ('pointer','text')})+'\n')
            except BaseException as error:
                with self.cv: self.error=repr(error); self.stopping=True
            finally:
                with self.cv: self.inflight-=1; self.cv.notify_all()

    def take(self):
        tick=time.monotonic()
        with self.cv:
            while self.consume_ticket not in self.ready:
                if self.error: raise RuntimeError(self.error)
                if self.stopping: raise RuntimeError('generator stopped before next input')
                if self.paused and not self.inflight: raise RuntimeError('prefilled diagnostic queue exhausted')
                self.cv.wait(timeout=1.)
            item=self.ready.pop(self.consume_ticket); self.consume_ticket+=1; self.cv.notify_all()
        self.wait_seconds+=time.monotonic()-tick
        return item

    def pause(self):
        with self.cv:
            self.paused=True
            while self.inflight: self.cv.wait(timeout=1.)
            if self.error: raise RuntimeError(self.error)

    def resume(self):
        with self.cv: self.paused=False; self.cv.notify_all()

    def set_update_phase(self, active):
        with self.cv: self.update_phase=active; self.cv.notify_all()

    def wait_full(self):
        with self.cv:
            while len(self.ready)+self.inflight < self.config['queue_capacity'] or self.inflight:
                if self.error: raise RuntimeError(self.error)
                self.cv.wait(timeout=.1)

    def snapshot(self):
        self.pause()
        with self.cv:
            return dict(generator_sha256=self.generator_sha256, seed_base=self.config['seed_base'],
                        seed_stride=self.config['seed_stride'], next_ticket=self.next_ticket, consume_ticket=self.consume_ticket,
                        ready=[{k:v for k,v in self.ready[i].items() if k!='pointer'} for i in sorted(self.ready)])

    def close(self):
        with self.cv: self.stopping=True; self.cv.notify_all()
        for thread in self.threads: thread.join()
        for item in self.ready.values(): self.engine.lib.v091_destroy(item['pointer'])
        self.ready.clear()


class FreshEpisodes:
    def __init__(self, engine, queue, rng, size, snapshot=None):
        self.engine, self.queue, self.rng, self.size = engine, queue, rng, size
        self.ids=np.zeros(size,np.int32);self.groups=np.zeros(size,np.int32)
        self.states=np.zeros((size,400),np.uint32);self.steps=np.zeros(size,np.int32);self.starts=np.full(size,-1,np.int32)
        if snapshot:
            for key,value in snapshot.items(): getattr(self,key)[:]=value
        else: self.reset(np.arange(size))

    def reset(self, slots):
        for i in slots:
            item=self.queue.take();self.engine.register(item)
            self.ids[i]=item['id'];self.groups[i]=self.rng.integers(8)
            self.states[i]=describe(item['text'])[0];self.steps[i]=0

    def state_dict(self):
        return {k:getattr(self,k).copy() for k in ('ids','groups','states','steps','starts')}
