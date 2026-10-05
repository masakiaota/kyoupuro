#!/usr/bin/env python3
"""選別逆教師の有界キャッシュと、既存教師を併用するPPO。"""
from collections import OrderedDict
import os
from pathlib import Path
import numpy as np
import torch
import torch.nn.functional as F
from v092_stream import ROOT,BC_RUN,CONFIG as BASE,FreshEngine
from v091_env import load
from v090_data import sha
from train_v090_board import tensors
from train_v091_ppo import optimize as ordinary_optimize,ppo_loss,sample_bc

RUN=ROOT/'results/nn_rank/v096/20261003_selected_studio'
PARENT=ROOT/'results/nn_rank/v092/20261003_fresh_studio'


def configuration(variant,mechanism=False):
    assert variant in ('control','mixed')
    result=dict(BASE,seed=96096,seed_base=960100000000,iterations=512,seconds=7200,
                final_deadline='2026-10-04T20:30:00+09:00',initial_lr=1e-5,final_lr=3e-6,
                bc_batch=32 if variant=='mixed' else 64,bc_coef=.025 if variant=='mixed' else .05,
                reverse_batch=32 if variant=='mixed' else 0,reverse_coef=.025 if variant=='mixed' else 0.)
    if mechanism:result.update(environments=8,horizon=8,iterations=2,seconds=600,epochs=1,minibatch=32,bc_batch=8,reverse_batch=8 if variant=='mixed' else 0,queue_capacity=16)
    return result


class ReverseData:
    def __init__(self,root):
        self.root=Path(root);d=self.root/'teachers';self.metadata=load(d/'dataset.json')
        assert self.metadata['training_eligible']
        self.cases={r['index']:r for r in self.metadata['cases']}
        for key in ('states','actions','case_ids','known_steps'):
            assert sha(d/(key+'.npy'))==self.metadata['array_sha256'][key]
            setattr(self,key,np.load(d/(key+'.npy'),mmap_mode='r'))

    def sample(self,engine,rng,size):
        ids=rng.integers(len(self.actions),size=size);groups=rng.integers(8,size=size)
        raw,codes,_=engine.observe(-1-np.asarray(self.case_ids[ids],np.int32),np.asarray(self.states[ids]),groups)
        match=(codes==self.actions[ids,None]) & raw['valid'];assert (match.sum(1)==1).all()
        return dict(raw,target=match.argmax(1).astype(np.int64))


class ReverseEngine(FreshEngine):
    def __init__(self,binary,data,reverse,workers=8):
        super().__init__(binary,data,workers);self.reverse=reverse;self.reverse_handles=OrderedDict()

    def pointers(self,ids):
        ids=np.asarray(ids);positive=ids>=0;result=np.empty(len(ids),np.uintp)
        if positive.any():result[positive]=super().pointers(ids[positive])
        protected=set(map(int,-1-ids[~positive]))
        for i in np.flatnonzero(~positive):
            cid=-1-int(ids[i])
            if cid not in self.reverse_handles:
                case=self.reverse.cases[cid];path=self.reverse.root/case['path'];assert sha(path)==case['sha256']
                ptr=self.lib.v091_create(os.fsencode(path))
                if not ptr:raise RuntimeError(self.lib.v091_error().decode())
                self.reverse_handles[cid]=ptr
            self.reverse_handles.move_to_end(cid);result[i]=self.reverse_handles[cid]
        for cid in list(self.reverse_handles):
            if len(self.reverse_handles)<=128:break
            if cid not in protected:self.lib.v091_destroy(self.reverse_handles.pop(cid))
        return result

    def close(self):
        for pointer in self.reverse_handles.values():self.lib.v091_destroy(pointer)
        self.reverse_handles.clear();super().close()


def optimize(model,optimizer,engine,data,buffer,rng,device,config,progress=0.,report=None):
    if config['reverse_coef']==0:
        result=ordinary_optimize(model,optimizer,engine,data,buffer,rng,device,config,progress,report)
        return dict(result,reverse_ce=0.,reverse_samples=0)
    model.train();sums=np.zeros(8);updates=0;kl_stop=False
    lr=config['initial_lr']+progress*(config['final_lr']-config['initial_lr']);optimizer.param_groups[0]['lr']=lr
    for epoch in range(config['epochs']):
        order=rng.permutation(len(buffer['ids']))
        for offset in range(0,len(order),config['minibatch']):
            ids=order[offset:offset+config['minibatch']]
            raw,_,_=engine.observe(buffer['ids'][ids],buffer['states'][ids],buffer['groups'][ids])
            loss,stats=ppo_loss(model,raw,*(buffer[k][ids] for k in ('action','old_logp','advantages','returns')),device,config)
            numbers=stats.cpu().numpy();assert np.isfinite(numbers).all()
            if numbers[3]>config['target_kl']:kl_stop=True;break
            bc=tensors(sample_bc(data,rng,config['bc_batch']),device)
            # CPU診断ではto(cpu)が共有バッファを参照するため、次の同サイズ観測の前に分離する。
            if device=='cpu':bc={k:v.clone() for k,v in bc.items()}
            logits,_=model(bc)
            imitation=F.cross_entropy(logits,bc['target'])
            reverse=tensors(engine.reverse.sample(engine,rng,config['reverse_batch']),device);reverse_logits,_=model(reverse)
            reverse_loss=F.cross_entropy(reverse_logits,reverse['target'])
            loss=loss+config['bc_coef']*imitation+config['reverse_coef']*reverse_loss
            optimizer.zero_grad(set_to_none=True);loss.backward()
            grad=torch.nn.utils.clip_grad_norm_(model.parameters(),config['gradient_clip'],error_if_nonfinite=True);optimizer.step()
            sums[:5]+=numbers;sums[5]+=float(imitation.detach());sums[6]+=float(grad);sums[7]+=float(reverse_loss.detach());updates+=1
            if report and updates%32==0:report(updates)
        if kl_stop:break
    assert updates>0
    result=dict(zip(('policy_loss','value_loss','entropy','approx_kl','clip_fraction','bc_ce','gradient_norm','reverse_ce'),(sums/updates).tolist()))
    result.update(updates=updates,kl_stopped=kl_stop,learning_rate=lr,reverse_samples=updates*config['reverse_batch'])
    return result
