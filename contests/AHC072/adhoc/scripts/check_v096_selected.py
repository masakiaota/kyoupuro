#!/usr/bin/env python3
"""新しい補助教師の全8変換と、両学習経路の短いCPU更新を検査する。"""
import argparse
from pathlib import Path
import shutil
import subprocess
import sys
import numpy as np
import torch
from v096_selected import RUN,ROOT,PARENT,BC_RUN,ReverseData,ReverseEngine
from v092_stream import TeacherData
from v091_env import build_library,load
from v090_data import save,sha,now
from train_v096_selected import train


def execute(root):
    d=root/'mechanism';d.mkdir(parents=True,exist_ok=True)
    if (d/'result.json').exists():return load(d/'result.json')
    data=TeacherData(BC_RUN/'data');reverse=ReverseData(root);engine=ReverseEngine(build_library(d/'environment'),data,reverse);data.engine=engine
    ids=np.linspace(0,len(reverse.actions)-1,8,dtype=int);errors=[]
    try:
        for g in range(8):
            raw,codes,_=engine.observe(-1-np.asarray(reverse.case_ids[ids],np.int32),np.asarray(reverse.states[ids]),np.full(8,g))
            valid=(codes==reverse.actions[ids,None])&raw['valid'];assert (valid.sum(1)==1).all()
        for variant in ('control','mixed'):
            initial=d/variant/'initial';initial.mkdir(parents=True,exist_ok=True);target=initial/'checkpoint.pt'
            if not target.exists():shutil.copy2(PARENT/'initial/checkpoint.pt',target)
    finally:engine.close()
    # 現在のMPS学習との競合を避け、機構の勾配更新だけCPUで通す。
    for variant in ('control','mixed'):
        result=train(d/variant,'cpu',root,True);assert result['iterations']==2
        final=result['final'];assert final['bc_ce']>0 and np.isfinite(final['gradient_norm'])
        if variant=='mixed':assert final['reverse_ce']>0 and final['reverse_samples']>0
        else:assert final['reverse_samples']==0
    a=torch.load(d/'control/initial/checkpoint.pt',weights_only=False);b=torch.load(d/'mixed/initial/checkpoint.pt',weights_only=False)
    assert all(torch.equal(v,b['model'][k]) for k,v in a['model'].items())
    saved=load(d/'mixed/training/result.json')
    report=dict(passed=True,transformed_states_checked=64,shared_initial_sha256=sha(d/'control/initial/checkpoint.pt'),
                reverse_ce=saved['final']['reverse_ce'],reverse_samples=saved['final']['reverse_samples'],device='cpu',completed_at=now())
    save(d/'result.json',report);print(report,flush=True);return report


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args();torch.set_num_threads(2);torch.set_num_interop_threads(2);execute(a.run.resolve())
