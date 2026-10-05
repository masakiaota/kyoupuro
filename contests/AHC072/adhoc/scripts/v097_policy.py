#!/usr/bin/env python3
"""共有盤面CNNを使う条件付き三段階方策。"""
from collections import OrderedDict
import ctypes as ct
import os
from pathlib import Path
import platform
import subprocess
import numpy as np
import torch
from torch import nn
import torch.nn.functional as F
from train_v090_board import Model,tensors
from v092_stream import ROOT,BC_RUN,FreshEngine,TeacherData
from v090_data import save,sha,now
from v091_env import load

RUN=ROOT/'results/nn_rank/v097/20261003_factor_studio'
PARENT=ROOT/'results/nn_rank/v092/20261003_fresh_studio'
CONFIG=dict(seed=97097,batch=128,steps=20000,seconds=1200,interval=5000,workers=8,learning_rate=1e-5,new_learning_rate=1e-4,weight_decay=.0001,gradient_clip=1.)


def build_library(directory):
    directory.mkdir(parents=True,exist_ok=True);binary=directory/('factor.dylib' if platform.system()=='Darwin' else 'factor.so')
    paths=[ROOT/'adhoc/bin/v097_environment.cpp',ROOT/'adhoc/bin/v091_environment.cpp',ROOT/'adhoc/scripts/v089_core.cpp.txt']
    identity={str(p.relative_to(ROOT)):sha(p) for p in paths};marker=directory/'build.json'
    if marker.exists():assert load(marker)['sources']==identity;return binary
    env=os.environ.copy()
    if platform.system()=='Darwin':
        env.setdefault('MACOSX_DEPLOYMENT_TARGET','15.0');env.setdefault('SDKROOT',subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip())
    subprocess.run([env.get('CXX','g++-15'),'-std=gnu++23','-O2','-march=native','-pthread','-fopenmp','-fPIC','-shared',str(paths[0]),'-o',str(binary)],check=True,env=env)
    save(marker,dict(sources=identity,binary_sha256=sha(binary)));return binary


class ChoiceEngine(FreshEngine):
    def __init__(self,binary,data,workers=8):
        super().__init__(binary,data,workers);self.lib.v097_prepare.argtypes=[ct.c_int,ct.c_int]+[ct.c_void_p]*17+[ct.c_int]
        self.lib.v097_prepare.restype=ct.c_int

    def pointers(self,ids):
        # 検証状態は分類指標の計測時だけ渡す。学習の抽出はtrainだけに限定する。
        result=[];protected=set(map(int,ids))
        for cid in ids:
            cid=int(cid);case=self.data.by_index[cid];assert case['role'] in ('train','validation')
            if cid not in self.handles:
                path=BC_RUN/case['path'];assert sha(path)==case['sha256'];ptr=self.lib.v091_create(os.fsencode(path))
                if not ptr:raise RuntimeError(self.lib.v091_error().decode())
                self.handles[cid]=ptr
            self.handles.move_to_end(cid);result.append(self.handles[cid])
        for cid in list(self.handles):
            if len(self.handles)<=128:break
            if cid not in protected:self.lib.v091_destroy(self.handles.pop(cid))
        return np.array(result,np.uintp)


class ChoiceData(TeacherData):
    def raw(self,ids,groups):
        ids=np.asarray(ids,np.int64);cases=np.asarray(self.case_ids[ids],np.int32)
        raw,codes,counts=self.engine.observe(cases,np.asarray(self.states[ids]),groups)
        target=np.asarray(self.targets[ids],np.int64)
        return dict(raw,target=target),codes,counts

    def factor(self,raw,codes,counts):
        B=len(counts);f={
            'x':raw['x'],'p_summary':np.zeros((B,400,33),np.float32),'p_valid':np.zeros((B,400),bool),
            'k_summary':np.zeros((B,8,33),np.float32),'k_valid':np.zeros((B,8),bool),
            'dl_features':np.zeros((B,32,16),np.float32),'dl_src':np.zeros((B,32),np.int64),'dl_dst':np.zeros((B,32),np.int64),
            'dl_valid':np.zeros((B,32),bool),'p_target':np.zeros(B,np.int64),'k_target':np.zeros(B,np.int64),'dl_target':np.zeros(B,np.int64)}
        stride=raw['src'].strides[0]//8
        ins=[raw['src'],raw['dst'],raw['features'],codes,counts,raw['target']]
        outs=[f[k] for k in ('p_summary','p_valid','k_summary','k_valid','dl_features','dl_src','dl_dst','dl_valid','p_target','k_target','dl_target')]
        error=self.engine.lib.v097_prepare(B,stride,*[a.ctypes.data for a in ins+outs],self.engine.workers)
        assert error==0,error
        return f

    def draw(self,rng,size):
        selected=rng.integers(len(self.train_cases),size=size)
        ids=np.array([self.train_cases[i]['frame_start']+int(rng.integers(self.train_cases[i]['frames'])) for i in selected])
        return ids,rng.integers(8,size=size)

    def __init__(self,directory):
        super().__init__(directory);self.train_cases=[c for c in self.cases if c['role']=='train']


class FactorModel(Model):
    def __init__(self,spec):
        super().__init__(spec);C,H=spec['width'],spec['hidden']
        self.tower=nn.Sequential(nn.Linear(2*C+33,H),nn.ReLU(),nn.Linear(H,1))
        self.split=nn.Sequential(nn.Linear(2*C+8+33,H),nn.ReLU(),nn.Linear(H,1))
        for head in (self.tower,self.split):nn.init.zeros_(head[2].weight);nn.init.zeros_(head[2].bias)

    def forward(self,batch):
        floor=batch['x'][:,:1];h=F.relu(self.input(batch['x']))*floor
        for block in self.blocks:h=block(h,floor)
        pooled=h.sum((2,3))/floor.sum((2,3));cells=h.flatten(2).transpose(1,2);B=len(cells);C=self.spec['width']
        p=self.tower(torch.cat((cells,pooled[:,None].expand(-1,400,-1),batch['p_summary']),-1)).squeeze(-1).masked_fill(~batch['p_valid'],-1e9)
        tower=cells[torch.arange(B,device=cells.device),batch['p_target']]
        ks=torch.eye(8,device=cells.device)[None].expand(B,-1,-1)
        k=self.split(torch.cat((tower[:,None].expand(-1,8,-1),pooled[:,None].expand(-1,8,-1),ks,batch['k_summary']),-1)).squeeze(-1).masked_fill(~batch['k_valid'],-1e9)
        source=torch.gather(cells,1,batch['dl_src'][...,None].expand(-1,-1,C));dest=torch.gather(cells,1,batch['dl_dst'][...,None].expand(-1,-1,C))
        dl=self.actor(torch.cat((source,dest,pooled[:,None].expand(-1,32,-1),batch['dl_features']),-1)).squeeze(-1).masked_fill(~batch['dl_valid'],-1e9)
        return p,k,dl


def initial(variant,device):
    torch.manual_seed(CONFIG['seed'])
    if device=='mps':torch.mps.manual_seed(CONFIG['seed'])
    saved=torch.load(PARENT/'initial/checkpoint.pt',map_location='cpu',weights_only=False);spec=saved['identity']['config']['model_spec']
    model=FactorModel(spec) if variant=='factor' else Model(spec)
    missing,unexpected=model.load_state_dict(saved['model'],strict=False)
    assert not unexpected and (not missing if variant=='flat' else set(missing)=={'tower.0.weight','tower.0.bias','tower.2.weight','tower.2.bias','split.0.weight','split.0.bias','split.2.weight','split.2.bias'})
    model=model.to(device)
    for p in model.critic.parameters():p.requires_grad_(False)
    old=[p for n,p in model.named_parameters() if p.requires_grad and not n.startswith(('tower.','split.'))]
    groups=[dict(params=old,lr=CONFIG['learning_rate'])]
    if variant=='factor':groups.append(dict(params=list(model.tower.parameters())+list(model.split.parameters()),lr=CONFIG['new_learning_rate']))
    optimizer=torch.optim.AdamW(groups,weight_decay=CONFIG['weight_decay'])
    return model,optimizer


def loss(model,batch,variant):
    if variant=='flat':logits,_=model(batch);return F.cross_entropy(logits,batch['target'])
    return sum(F.cross_entropy(logits,batch[key]) for logits,key in zip(model(batch),('p_target','k_target','dl_target')))
