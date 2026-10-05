#!/usr/bin/env python3
"""共有初期値、学習の更新、教師一致率と二つの操作選択を確認する。"""
import json
from pathlib import Path
import subprocess
import numpy as np
import torch
from v097_policy import *
from train_v097_policy import metric_frames,evaluate
from check_v089_board import compile_binary,transformed_input
from v089_data import Geometry,remaining


def execute():
    d=RUN/'mechanism';d.mkdir(parents=True,exist_ok=True)
    if (d/'training_result.json').exists():return load(d/'training_result.json')
    data=ChoiceData(BC_RUN/'data');engine=ChoiceEngine(build_library(RUN/'environment'),data);data.engine=engine
    flat,fo=initial('flat','cpu');factor,ho=initial('factor','cpu')
    assert all(torch.equal(v,factor.state_dict()[k]) for k,v in flat.state_dict().items())
    rng=np.random.default_rng(97097);ids,groups=data.draw(rng,8);raw,codes,counts=data.raw(ids,groups);fac=data.factor(raw,codes,counts)
    for p in np.flatnonzero(fac['p_valid'][0]):assert any(raw['src'][0,:counts[0]]==p)
    numbers={}
    for variant,model,optimizer,batch in [('flat',flat,fo,tensors(raw,'cpu')),('factor',factor,ho,tensors(fac,'cpu'))]:
        before=float(loss(model,batch,variant).detach())
        for _ in range(8):
            objective=loss(model,batch,variant);optimizer.zero_grad();objective.backward();torch.nn.utils.clip_grad_norm_(model.parameters(),1.);optimizer.step()
        after=float(loss(model,batch,variant).detach());assert np.isfinite(after) and after<before
        numbers[variant]=dict(before_ce=before,after_ce=after)
    frames=metric_frames(RUN,data)
    for variant,model in [('flat',flat),('factor',factor)]:
        sample={k:v[:32] for k,v in frames['validation'].items()};numbers[variant]['classification_check']=evaluate(model,data,sample,variant,'cpu')
    wrapper=ROOT/'adhoc/bin/check_v097_top2_mechanism.cpp'
    wrapper.write_text('// check_v097_top2_mechanism.cpp\n#define V097_DIAGNOSTIC\n#define V097_TOP2\n#include "v097_mechanism.cpp"\n')
    top2=d/'top2_checker';compile_binary(wrapper.stem,top2,True)
    greedy=d/'numerical/checker_local';checked=0
    for case in [c for c in data.cases if c['role']=='train'][:4]:
        fid=case['frame_start']+max(0,case['frames']-4);geo=Geometry(BC_RUN/case['path']);snap=d/'decoder_snapshot.txt';state=np.array(data.states[fid]);snap.write_text(' '.join(map(str,state))+'\n')
        inp=(BC_RUN/case['path']).read_text()
        dumped=json.loads(subprocess.run([greedy,snap,'dump'],input=inp,text=True,capture_output=True,check=True).stdout)
        p_order=sorted(dumped['p'],key=lambda row:(-row[1],row[0]));p=p_order[0][0]
        k_order=sorted([row for row in dumped['k'] if row[0]==p],key=lambda row:(-row[2],row[1]));k=k_order[0][1]
        options=[row for row in dumped['actions'] if (row[0]&511)==p and ((row[0]>>9)&7)==k]
        expected_greedy=max(options,key=lambda row:row[1])[0];tops={row[0] for row in p_order[:2]}
        expected_top=max([row for row in dumped['actions'] if (row[0]&511) in tops],key=lambda row:row[2])[0]
        for binary,expected in ((greedy,expected_greedy),(top2,expected_top)):
            proc=subprocess.run([binary,snap],input=inp,text=True,capture_output=True,check=True,timeout=10.)
            lines=[line for line in proc.stdout.splitlines() if line.strip()];assert lines
            current=state.copy();actual=None
            for line in lines:
                code=geo.apply(current,line)
                if actual is None:actual=code
            assert actual==expected,(case['index'],binary,actual,expected)
            checked+=1
    engine.close();result=dict(passed=True,shared_initial_weights_equal=True,gradients=numbers,decoder_checks=checked,completed_at=now())
    save(d/'training_result.json',result);print(json.dumps(result),flush=True);return result


if __name__=='__main__':
    torch.set_num_threads(2);torch.set_num_interop_threads(2);execute()
