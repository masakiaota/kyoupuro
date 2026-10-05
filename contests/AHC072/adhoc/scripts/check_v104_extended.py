#!/usr/bin/env python3
"""追加学習の引き継ぎと、完走した実測結果による更新を確認する。"""
import argparse
from pathlib import Path
import numpy as np
import torch
from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from v089_data import Geometry,remaining,save,sha,now
from v091_env import build_library
from v092_stream import TeacherData,FreshEngine,OfficialQueue
from v099_complete import BC_RUN,start_model,measured_targets
from v102_learning import collect,optimize
from v104_extended import configuration,inherited_state,restore,blocked_inputs,INITIAL_SHA
from check_v098_complete import action_line


def execute(root):
    directory=root/'mechanism';directory.mkdir(parents=True,exist_ok=True)
    assert not (directory/'result.json').exists()
    config=configuration();saved=inherited_state(root);data=TeacherData(BC_RUN/'data')
    engine=FreshEngine(build_library(directory/'environment'),data,8);data.engine=engine
    blocked=blocked_inputs(root);queue=None
    model,optimizer=start_model(root/'initial/checkpoint.pt','mps',config)
    rngs={k:np.random.default_rng() for k in saved['rngs']};restore(model,optimizer,rngs,saved,'mps')
    assert all(torch.equal(v.cpu(),saved['model'][k]) for k,v in model.state_dict().items())
    actual=optimizer.state_dict()
    assert actual['param_groups']==saved['optimizer']['param_groups']
    for key,state in saved['optimizer']['state'].items():
        for field,value in state.items():assert torch.equal(actual['state'][key][field].cpu(),value)
    for key,rng in rngs.items():assert rng.bit_generator.state==saved['rngs'][key]
    try:
        queue=OfficialQueue(directory/'inherited_queue',engine,blocked,config,saved['queue']);queue.pause()
        expected=saved['queue']['ready'][0];item=queue.take()
        assert all(item[k]==expected[k] for k in ('id','ticket','seed','text','sha256'))
        engine.lib.v091_destroy(item['pointer']);queue.close();queue=None
        small=dict(config,boards=2,repetitions=4,environments=8,epochs=1,minibatch=64,queue_capacity=8,seed_base=1040900000000)
        queue=OfficialQueue(directory/'small',engine,blocked,small);queue.wait_full();queue.set_update_phase(False)
        buffer,episodes,final_states,items=collect(model,engine,queue,rngs['action'],rngs['transform'],'mps',small)
        for i,item in enumerate(items):
            path=directory/f'{i}.txt';path.write_text(item['text']);geo=Geometry(path)
            for slot in range(i*4,i*4+4):
                state=geo.initial.copy()
                for code in episodes[slot]['actions']:geo.apply(state,action_line(code))
                assert np.array_equal(state,final_states[slot]) and remaining(state)==episodes[slot]['E']
        lengths=np.array([e['T'] for e in episodes]);left=np.array([e['E'] for e in episodes])
        one=measured_targets(lengths,left,buffer['episode'],buffer['time'],buffer['value'],4,'group')
        two=measured_targets(lengths,left,buffer['episode'],buffer['time'],buffer['value']+100,4,'group')
        assert np.array_equal(one[0],two[0]) and np.array_equal(one[1],two[1])
        queue.set_update_phase(True)
        # 比較用の同じ更新を保存状態から一度復元する。診断重みは本学習へ渡さない。
        checkpoint(directory/'before_update.pt',dict(model={k:v.detach().cpu() for k,v in model.state_dict().items()},
             optimizer=optimizer.state_dict(),rngs={k:r.bit_generator.state for k,r in rngs.items()},
             torch_rng=torch.get_rng_state(),mps_rng=torch.mps.get_rng_state()))
        metrics=optimize(model,optimizer,engine,data,buffer,rngs['shuffle'],rngs['auxiliary'],'mps',small,0.)
        assert metrics['value_loss']==metrics['bc_ce']==metrics['bc_samples']==0
        assert metrics['updates']>0
        restored,other=start_model(root/'initial/checkpoint.pt','mps',config)
        checkpoint_state=torch.load(directory/'before_update.pt',map_location='cpu',weights_only=False)
        restore(restored,other,rngs,checkpoint_state,'mps')
        again=optimize(restored,other,engine,data,buffer,rngs['shuffle'],rngs['auxiliary'],'mps',small,0.)
        error=max(float((v-restored.state_dict()[k]).abs().max().cpu()) for k,v in model.state_dict().items())
        assert error<2e-6 and again['updates']==metrics['updates']
        assert all(torch.equal(v.cpu(),saved['model'][k]) for k,v in model.state_dict().items() if k.startswith('critic.'))
        assert any(not torch.equal(v.cpu(),saved['model'][k]) for k,v in model.state_dict().items())
        save(directory/'result.json',dict(passed=True,initial_sha256=INITIAL_SHA,inherited_optimizer_steps=27840,
             inherited_next_seed=expected['seed'],inherited_rngs_identical=True,episodes=len(episodes),
             successes=sum(e['E']==0 for e in episodes),all_legally_replayed=True,metrics=metrics,
             resume_weight_max_error=error,critic_unchanged=True,device='mps',completed_at=now()))
    finally:
        if queue:queue.close()
        engine.release_except([]);engine.close()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=Path);p.add_argument('--gpu-state',required=True,type=Path);a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2);torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    reporter=GPUReport(a.gpu_state)
    try:execute(a.run.resolve())
    finally:reporter.close()
