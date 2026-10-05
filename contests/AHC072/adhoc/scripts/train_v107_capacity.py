#!/usr/bin/env python3
"""同じ教師と標本順を使い、元容量と約2倍容量を比較する。"""
import argparse
from collections import OrderedDict
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
import json
import os
from pathlib import Path
import signal
import time

import numpy as np
import torch
import torch.nn.functional as F
from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from train_v090_board import Model
from v089_data import save, sha, status, now
from v091_env import Engine, build_library, load
from v101_compute import distribution, owned
from v107_capacity import INITIAL, INITIAL_SHA, SPECS, initial_model as capacity_model

ACTIVE_CAPACITY = None


class Teacher:
    def __init__(self, directory):
        self.directory=Path(directory);self.metadata=load(self.directory/'dataset.json')
        self.root=Path(self.metadata['root']);self.cases=self.metadata['cases'];self.by_index={c['index']:c for c in self.cases}
        for key in ('states','actions','changed'):
            path=self.directory/(key+'.npy');assert sha(path)==self.metadata['array_sha256'][key]
            setattr(self,key,np.load(path,mmap_mode='r'))
        self.changed_by_case={c['index']:np.flatnonzero(self.changed[c['frame_start']:c['frame_start']+c['frames']])+c['frame_start'] for c in self.cases}
        assert all(len(v)>0 for v in self.changed_by_case.values())

    def choose(self,rng):
        cases=rng.integers(len(self.cases),size=8);ids=[];frames=[]
        for i in cases:
            c=self.cases[i];ids.extend([c['index']]*16)
            frames.extend((c['frame_start']+rng.integers(c['frames'],size=8)).tolist())
            changed=self.changed_by_case[c['index']];frames.extend(rng.choice(changed,size=8).tolist())
        return dict(ids=np.asarray(ids,np.int32),frames=np.asarray(frames,np.int64),groups=rng.integers(8,size=128,dtype=np.int32))

    def batch(self,engine,selection):
        frames=selection['frames'];raw,codes,_=engine.observe(selection['ids'],np.asarray(self.states[frames]),selection['groups'])
        match=(codes==self.actions[frames,None]) & raw['valid'];assert (match.sum(1)==1).all()
        return owned(dict(raw,target=match.argmax(1).astype(np.int64)))


class TeacherEngine(Engine):
    def __init__(self,binary,data,workers=8):
        super().__init__(binary,data,workers);self.handles=OrderedDict()

    def pointers(self,ids):
        result=[];protected=set(map(int,ids))
        for value in ids:
            cid=int(value)
            if cid not in self.handles:
                row=self.data.by_index[cid];path=self.data.root/row['path'];assert sha(path)==row['sha256']
                pointer=self.lib.v091_create(os.fsencode(path))
                if not pointer:raise RuntimeError(self.lib.v091_error().decode())
                self.handles[cid]=pointer
            self.handles.move_to_end(cid);result.append(self.handles[cid])
        for cid in list(self.handles):
            if len(self.handles)<=128:break
            if cid not in protected:self.lib.v091_destroy(self.handles.pop(cid))
        return np.asarray(result,np.uintp)


def configuration(diagnostic):
    assert ACTIVE_CAPACITY in SPECS
    return dict(seed=107001 if diagnostic else 107002, updates=20000 if diagnostic else 80000,
                seconds=1800 if diagnostic else 7200, batch=128, initial_lr=1e-4 if diagnostic else 3e-5,
                final_lr=1e-5 if diagnostic else 3e-6, weight_decay=1e-4, gradient_clip=1., workers=8,
                capacity=ACTIVE_CAPACITY, model_spec=SPECS[ACTIVE_CAPACITY], diagnostic=diagnostic,
                deadline='2026-10-04T18:00:00+09:00')


def initial_model(device):
    return capacity_model(ACTIVE_CAPACITY, device)


@torch.no_grad()
def metrics(model,data,engine,device):
    rng=np.random.default_rng(103009);sums=np.zeros(4);model.eval()
    # 毎回同じ4096標本。検証入力ではなく、学習列の再現能力だけを診断する。
    for _ in range(32):
        raw=data.batch(engine,data.choose(rng));logp,_=distribution(model,raw,device)
        target=torch.as_tensor(raw['target'],device=device);ce=F.nll_loss(logp,target,reduction='none')
        correct=(logp.argmax(-1)==target).float();changed=(torch.arange(128,device=device)%16)>=8
        sums+=torch.stack([ce.sum(),correct.sum(),ce[changed].sum(),correct[changed].sum()]).cpu().numpy()
    return dict(ce=float(sums[0]/4096),top1=float(sums[1]/4096),changed_ce=float(sums[2]/2048),changed_top1=float(sums[3]/2048))


def export(root,model,updates,data,config):
    save(root/'training/model.json',dict(parameters={k:v.detach().cpu().tolist() for k,v in model.state_dict().items()},
         spec=model.spec,size=ACTIVE_CAPACITY,epoch=updates,dataset_sha256=sha(data.directory/'dataset.json'),
         initial_checkpoint_sha256=INITIAL_SHA,seed=config['seed'],method='BC on '+data.metadata['variant']+' self trajectories'))


def train(root,data_directory,diagnostic,device='mps'):
    directory=root/'training';directory.mkdir(parents=True,exist_ok=True)
    if (directory/'result.json').exists():return load(directory/'result.json')
    config=configuration(diagnostic);data=Teacher(data_directory);engine=TeacherEngine(build_library(root/'environment'),data)
    identity=dict(config=config,dataset_sha256=sha(data_directory/'dataset.json'),initial_sha256=INITIAL_SHA,device=device)
    if (directory/'config.json').exists():assert load(directory/'config.json')==identity
    else:save(directory/'config.json',identity)
    model=initial_model(device);optimizer=torch.optim.AdamW((p for p in model.parameters() if p.requires_grad),lr=config['initial_lr'],weight_decay=config['weight_decay'])
    rng=np.random.default_rng(config['seed']);torch.manual_seed(config['seed'])
    if device=='mps':torch.mps.manual_seed(config['seed'])
    previous=0.;update=0;history=[];pending=None;saved=None
    if (directory/'latest.pt').exists():
        saved=torch.load(directory/'latest.pt',map_location='cpu',weights_only=False);assert saved['identity']==identity
        model.load_state_dict(saved['model']);optimizer.load_state_dict(saved['optimizer']);rng.bit_generator.state=saved['rng']
        torch.set_rng_state(saved['torch_rng'])
        if device=='mps':torch.mps.set_rng_state(saved['mps_rng'])
        previous,update,history,pending=(saved[k] for k in ('seconds','updates','history','pending'))
    del saved
    if not (directory/'initial_metrics.json').exists():save(directory/'initial_metrics.json',metrics(model,data,engine,device))
    started=time.monotonic();stopped=False;window=[];last_report=0.
    def seconds():return previous+time.monotonic()-started
    def stop(signum,frame):
        nonlocal stopped
        stopped=True
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    def persist():
        checkpoint(directory/'latest.pt',dict(identity=identity,model={k:v.detach().cpu() for k,v in model.state_dict().items()},
                   optimizer=optimizer.state_dict(),rng=rng.bit_generator.state,torch_rng=torch.get_rng_state(),
                   mps_rng=torch.mps.get_rng_state() if device=='mps' else None,seconds=seconds(),updates=update,history=history,pending=pending))
        (directory/'metrics.jsonl').write_text(''.join(json.dumps(r,allow_nan=False)+'\n' for r in history))
    reason='update_budget';wait_seconds=update_seconds=0.
    try:
        if pending is None:pending=data.choose(rng)
        if update==0:persist()
        model.train()
        with ThreadPoolExecutor(max_workers=1) as pool:
            future=pool.submit(data.batch,engine,pending)
            while update<config['updates']:
                if stopped:reason='interrupted';break
                if seconds()>=config['seconds']:reason='time_budget';break
                if datetime.now().astimezone()>=datetime.fromisoformat(config['deadline']):reason='deadline';break
                tick=time.monotonic();raw=future.result();wait_seconds+=time.monotonic()-tick
                # 先読み標本とその抽出後の乱数を保存すれば、再開時に同じ次batchを使える。
                pending=data.choose(rng) if update+1<config['updates'] else None
                if pending is not None:future=pool.submit(data.batch,engine,pending)
                lr=config['initial_lr']+(config['final_lr']-config['initial_lr'])*update/max(1,config['updates']-1)
                optimizer.param_groups[0]['lr']=lr;tick=time.monotonic()
                logp,_=distribution(model,raw,device);target=torch.as_tensor(raw['target'],device=device)
                loss=F.nll_loss(logp,target);optimizer.zero_grad(set_to_none=True);loss.backward()
                gradient=torch.nn.utils.clip_grad_norm_(model.parameters(),1.,error_if_nonfinite=True);optimizer.step()
                numbers=torch.stack([loss.detach(),(logp.detach().argmax(-1)==target).float().mean(),gradient]).cpu().numpy()
                assert np.isfinite(numbers).all();window.append(numbers);update+=1;update_seconds+=time.monotonic()-tick
                if update%250==0:
                    values=np.mean(window,axis=0);window=[]
                    history.append(dict(updates=update,seconds=seconds(),ce=float(values[0]),top1=float(values[1]),gradient=float(values[2]),
                                        learning_rate=lr,wait_seconds=wait_seconds,update_seconds=update_seconds))
                    persist()
                if time.monotonic()-last_report>30:
                    status(directory,'learning',updates=update,budget=config['updates'],seconds=seconds(),learning_rate=lr,
                           wait_seconds=wait_seconds,update_seconds=update_seconds,device=device,geometry_cache=len(engine.handles))
                    last_report=time.monotonic()
        persist()
        if reason=='interrupted':raise InterruptedError('stopped after saved update')
        final=metrics(model,data,engine,device);export(root,model,update,data,config)
        result=dict(updates=update,complete_budget=update==config['updates'],reason=reason,seconds=seconds(),
                    initial=load(directory/'initial_metrics.json'),final=final,history=history,checkpoint_sha256=sha(directory/'latest.pt'),completed_at=now())
        save(directory/'result.json',result);status(directory,'completed',updates=update,reason=reason)
        return result
    finally:engine.close()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=Path);p.add_argument('--data',required=True,type=Path)
    p.add_argument('--capacity',required=True,choices=('small','wide'));p.add_argument('--diagnostic',action='store_true');p.add_argument('--device',default='mps',choices=('cpu','mps'));p.add_argument('--gpu-state',required=True,type=Path)
    a=p.parse_args();ACTIVE_CAPACITY=a.capacity;torch.set_num_threads(2);torch.set_num_interop_threads(2)
    if a.device=='mps':torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    reporter=GPUReport(a.gpu_state) if a.device=='mps' else None
    try:train(a.run.resolve(),a.data.resolve(),a.diagnostic,a.device)
    finally:
        if reporter:reporter.close()
