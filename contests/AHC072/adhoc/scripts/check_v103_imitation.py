#!/usr/bin/env python3
"""新教師の操作対応と、模倣更新・先読み標本からの再開を照合する。"""
import argparse
import copy
from pathlib import Path
import numpy as np
import torch
import torch.nn.functional as F
from train_v077_rank import checkpoint
from train_v103_imitation import Teacher,TeacherEngine,initial_model,configuration
from v091_env import build_library
from v089_data import Geometry,save,sha,now
from v101_compute import distribution
from check_v098_complete import action_line


def check(root):
    directory=root/'mechanism';directory.mkdir(parents=True,exist_ok=True)
    data=Teacher(root/'diagnostic/data/shortened');engine=TeacherEngine(build_library(directory/'environment'),data)
    rng=np.random.default_rng(103099);checks=0
    try:
        for case in data.cases[:4]:
            fid=case['frame_start']+case['frames']//2
            for group in range(8):
                state=np.array(data.states[fid],copy=True);original=state.copy();code=int(data.actions[fid])
                raw,codes,_=engine.observe([case['index']],state[None],np.array([group],np.int32))
                assert np.sum((codes[0]==code)&raw['valid'][0])==1
                engine.step([case['index']],state[None],np.array([code],np.uint32))
                Geometry(data.root/case['path']).apply(original,action_line(code))
                assert np.array_equal(original,state);checks+=1
        device='mps';config=configuration(True);model=initial_model(device)
        optimizer=torch.optim.AdamW((p for p in model.parameters() if p.requires_grad),lr=config['initial_lr'],weight_decay=config['weight_decay'])
        critic={k:v.detach().cpu().clone() for k,v in model.critic.state_dict().items()}
        def update(net,opt,selection):
            raw=data.batch(engine,selection);logp,_=distribution(net,raw,device);loss=F.nll_loss(logp,torch.as_tensor(raw['target'],device=device))
            opt.zero_grad(set_to_none=True);loss.backward();grad=torch.nn.utils.clip_grad_norm_(net.parameters(),1.,error_if_nonfinite=True);opt.step()
            return float(loss.detach()),float(grad)
        first=update(model,optimizer,data.choose(rng));pending=data.choose(rng)
        checkpoint(directory/'checkpoint.pt',dict(model={k:v.detach().cpu() for k,v in model.state_dict().items()},optimizer=optimizer.state_dict(),pending=pending,rng=rng.bit_generator.state))
        second=update(model,optimizer,pending);next_selection=data.choose(rng)
        saved=torch.load(directory/'checkpoint.pt',map_location='cpu',weights_only=False);restored=initial_model(device);restored.load_state_dict(saved['model'])
        opt=torch.optim.AdamW((p for p in restored.parameters() if p.requires_grad),lr=config['initial_lr'],weight_decay=config['weight_decay']);opt.load_state_dict(saved['optimizer'])
        resumed=update(restored,opt,saved['pending']);rng.bit_generator.state=saved['rng'];repeated=data.choose(rng)
        for key in next_selection:assert np.array_equal(next_selection[key],repeated[key])
        maximum=max(float((v-restored.state_dict()[k]).abs().max().detach().cpu()) for k,v in model.state_dict().items())
        assert maximum<2e-6 and abs(second[0]-resumed[0])<2e-6
        assert any(float((v-saved['model'][k].to(device)).abs().max().detach().cpu())>0 for k,v in model.state_dict().items())
        for k,v in model.critic.state_dict().items():assert torch.equal(v.cpu(),critic[k])
        result=dict(passed=True,transition_checks=checks,d4_groups=8,first_update=first,second_update=second,
                    resumed_update=resumed,resume_weight_max_error=maximum,prefetch_rng_identical=True,critic_unchanged=True,
                    device=device,dataset_sha256=sha(data.directory/'dataset.json'),completed_at=now())
        save(directory/'result.json',result);return result
    finally:engine.close()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=Path);a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2);torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    print(check(a.run.resolve()))
