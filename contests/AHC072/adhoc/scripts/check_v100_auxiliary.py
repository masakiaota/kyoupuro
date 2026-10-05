#!/usr/bin/env python3
"""再選別教師の合法手照合と、完走学習への模倣補助の接続を検査する。"""
import argparse
from pathlib import Path
import numpy as np
import torch
from train_v080_scaling import GPUReport
from v089_data import Geometry, remaining, save, sha, now
from v091_env import build_library, load
from v092_stream import TeacherData, OfficialQueue, excluded_inputs
from v099_complete import (REVERSE_RUN, BC_RUN, configuration, start_model, ReverseEngine, SelectedReverse,
                           collect, optimize)
from check_v098_complete import action_line


def execute(root):
    d=root/'mechanism';d.mkdir(parents=True,exist_ok=True)
    if (d/'result.json').exists():return load(d/'result.json')
    choice=load(root/'initial_choice.json');method=choice['method'];initial=root/'control/initial/checkpoint.pt'
    assert sha(initial)==sha(root/'mixed/initial/checkpoint.pt')==choice['checkpoint_sha256']
    config=dict(configuration(method,True,True),boards=2,environments=8,queue_capacity=8,
                seed_base=1000900000000,epochs=1,minibatch=64,bc_batch=8,reverse_batch=8,
                bc_pool_cases=8,reverse_pool_states=16)
    data=TeacherData(BC_RUN/'data');reverse=SelectedReverse(root)
    engine=ReverseEngine(build_library(d/'environment'),data,reverse,8);data.engine=engine
    queue=OfficialQueue(d,engine,excluded_inputs(),config)
    try:
        ids=reverse.selected[np.linspace(0,len(reverse.selected)-1,8,dtype=int)]
        for g in range(8):
            raw,codes,_=engine.observe(-1-np.asarray(reverse.case_ids[ids],np.int32),np.asarray(reverse.states[ids]),np.full(8,g))
            assert (((codes==reverse.actions[ids,None]) & raw['valid']).sum(1)==1).all()
        model,_=start_model(initial,'mps',config);queue.wait_full();queue.set_update_phase(False)
        buffer,episodes,states,items=collect(model,engine,queue,np.random.default_rng(100104),np.random.default_rng(100105),'mps',config)
        queue.pause()
        for i,item in enumerate(items):
            path=d/f'{i}.txt';path.write_text(item['text']);geo=Geometry(path)
            for j in range(i*4,i*4+4):
                state=geo.initial.copy()
                for code in episodes[j]['actions']:geo.apply(state,action_line(code))
                assert np.array_equal(state,states[j]) and remaining(state)==episodes[j]['E']
        results={};initial_weights=torch.load(initial,map_location='cpu',weights_only=False)['model']
        for variant in ('control','mixed'):
            current_config=dict(config,bc_batch=16 if variant=='control' else 8,bc_coef=.05 if variant=='control' else .025,
                                reverse_batch=0 if variant=='control' else 8,reverse_coef=0. if variant=='control' else .025)
            current,optimizer=start_model(initial,'mps',current_config)
            assert all(torch.equal(v.cpu(),initial_weights[k]) for k,v in current.state_dict().items())
            result=optimize(current,optimizer,engine,data,buffer,np.random.default_rng(100106),np.random.default_rng(100107),
                            'mps',current_config,0.)
            assert result['bc_ce']>0 and (result['reverse_samples']>0)==(variant=='mixed')
            assert (result['reverse_ce']>0)==(variant=='mixed')
            results[variant]=result
        result=dict(passed=True,transformed_states=64,episodes=len(episodes),all_legal=True,updates=results,
                    shared_initial_sha256=sha(initial),selection_sha256=sha(root/'teachers/selection.json'),completed_at=now())
        save(d/'result.json',result);return result
    finally:queue.close();engine.close()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=REVERSE_RUN);p.add_argument('--gpu-state',type=Path,required=True)
    a=p.parse_args();torch.set_num_threads(2);torch.set_num_interop_threads(2)
    torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    reporter=GPUReport(a.gpu_state)
    try:execute(a.run.resolve())
    finally:reporter.close()
