#!/usr/bin/env python3
"""記録された合法完成費用を、固定120周の小型残差モデルで学習する。"""
import argparse
from datetime import datetime
import math
from pathlib import Path
import signal
import time

import numpy as np
import torch
from torch import nn
from torch.nn import functional as F

from train_v080_scaling import GPUReport
from v116_data import RUN,load,save,sha,now

SEED=116001
BATCH=8192
EPOCHS=120
FEATURES=['lower','old_estimate','jump','sources','destinations','unsettled','pieces','goal_pieces',
          'excess','towers','goal_towers','max_height','max_goal_height','height_squared','goal_height_squared',
          'matched_prefix','exact_towers','partial_towers','outside_goal','mono_towers','runs','singletons',
          'mixed_towers','required_colors','active_colors','colors_to_return','correct_top','correct_bottom',
          'on_nest','blocked_nest','weighted_target_distance','distinct_target_distance','max_target_distance',
          'top_target_distance','height_scaled_distance','empty_goal_cells','disjoint_goal_colors','matched_fraction',
          'shared_color_pairs','mean_shared_distance','min_shared_distance','support_under','neighbor_support',
          'background_pieces','background_cells','shared_pair_fraction','N','floor_fraction']
assert len(FEATURES)==48


class Model(nn.Module):
    def __init__(self,mean,scale):
        super().__init__()
        self.register_buffer('mean',torch.as_tensor(mean,dtype=torch.float32))
        self.register_buffer('scale',torch.as_tensor(scale,dtype=torch.float32))
        self.first=nn.Linear(48,32)
        self.last=nn.Linear(32,1)
        nn.init.zeros_(self.last.weight)
        nn.init.zeros_(self.last.bias)

    def forward(self,x):
        residual=self.last(F.relu(self.first((x-self.mean)/self.scale))).squeeze(-1)
        value=torch.maximum(x[:,0],x[:,1]+residual)
        return torch.where(x[:,0]==0,torch.zeros_like(value),value)


def partners(array):
    out=np.arange(len(array),dtype=np.int64)
    cuts=np.r_[0,np.flatnonzero(np.diff(array[:,50]))+1,len(array)]
    generator=np.random.default_rng(SEED+19)
    for a,b in zip(cuts[:-1],cuts[1:]):
        ids=generator.permutation(np.arange(a,b))
        out[ids]=np.roll(ids,1)
    return out


@torch.no_grad()
def evaluate(model,values,pair_ids):
    pred=[]
    model.eval()
    for start in range(0,len(values),BATCH):
        pred.append(model(values[start:start+BATCH,:48]).cpu())
    pred=torch.cat(pred).numpy()
    data=values.cpu().numpy()
    y=data[:,48]
    total=pred+data[:,49]
    observed=y+data[:,49]
    pair=np.asarray(pair_ids)
    mask=observed!=observed[pair]
    model.train()
    return dict(mae=float(np.abs(pred-y).mean()),mse=float(np.square(pred-y).mean()),
                baseline_mae=float(np.abs(data[:,1]-y).mean()),
                ranking_pairs=int(mask.sum()),
                ranking_accuracy=float(((total[mask]-total[pair[mask]])*(observed[mask]-observed[pair[mask]])>0).mean()) if mask.any() else None,
                baseline_ranking_accuracy=float(((data[mask,1]+data[mask,49]-data[pair[mask],1]-data[pair[mask],49])*(observed[mask]-observed[pair[mask]])>0).mean()) if mask.any() else None)


def checkpoint(path,value):
    temp=path.with_suffix('.tmp')
    torch.save(value,temp)
    temp.replace(path)


def train(root):
    directory=root/'training';directory.mkdir(parents=True,exist_ok=True)
    if (directory/'result.json').exists():return load(directory/'result.json')
    assert load(root/'data/pilot.json')['gate_passed']
    description=load(root/'data/dataset.json')
    arrays={role:np.load(root/'data'/f'{role}.npy') for role in ['train','development']}
    for role,array in arrays.items():
        assert sha(root/'data'/f'{role}.npy')==description['arrays'][role]
        assert array.shape[1]==51 and len(array)>0 and np.isfinite(array).all()
    mean=arrays['train'][:,:48].mean(0,dtype=np.float64).astype(np.float32)
    scale=arrays['train'][:,:48].std(0,dtype=np.float64).astype(np.float32)
    scale[scale<1e-5]=1
    torch.manual_seed(SEED);np.random.seed(SEED)
    model=Model(mean,scale).to('mps')
    optimizer=torch.optim.AdamW(model.parameters(),lr=1e-3,weight_decay=1e-4)
    pairs={role:partners(array) for role,array in arrays.items()}
    values={role:torch.from_numpy(array).to('mps') for role,array in arrays.items()}
    train_pairs=torch.from_numpy(pairs['train']).to('mps')
    identity=dict(dataset_sha256=sha(root/'data/dataset.json'),source_sha256=sha(Path(__file__)),
                  seed=SEED,batch=BATCH,epochs=EPOCHS,features=FEATURES,parameters=sum(x.numel() for x in model.parameters()),
                  lr_start=.001,lr_end=.00003,weight_decay=.0001,rank_coefficient=.25,device='mps')
    if (directory/'config.json').exists():assert load(directory/'config.json')==identity
    else:save(directory/'config.json',identity)
    epoch=offset=steps=0;elapsed=0.;history=[]
    latest=directory/'latest.pt'
    if latest.exists():
        stored=torch.load(latest,map_location='cpu',weights_only=False)
        assert stored['identity']==identity
        model.load_state_dict(stored['model']);optimizer.load_state_dict(stored['optimizer'])
        epoch,offset,steps,elapsed,history=(stored[k] for k in ['epoch','offset','steps','elapsed','history'])
        torch.set_rng_state(stored['torch_rng']);torch.mps.set_rng_state(stored['mps_rng'])
    else:
        initial=evaluate(model,values['development'],pairs['development'])
        save(directory/'initial_metrics.json',initial)
    initial_elapsed=elapsed;started=time.monotonic();last_report=started
    stop=False
    def interrupted(signum,frame):
        nonlocal stop
        stop=True
    signal.signal(signal.SIGTERM,interrupted);signal.signal(signal.SIGINT,interrupted)
    n=len(values['train']);total_steps=math.ceil(n/BATCH)*EPOCHS
    deadline=datetime.fromisoformat('2026-10-04T23:30:00+09:00').timestamp()
    def persist():
        checkpoint(latest,dict(identity=identity,model={k:v.detach().cpu() for k,v in model.state_dict().items()},
                              optimizer=optimizer.state_dict(),epoch=epoch,offset=offset,steps=steps,
                              elapsed=initial_elapsed+time.monotonic()-started,history=history,
                              torch_rng=torch.get_rng_state(),mps_rng=torch.mps.get_rng_state()))
    if not latest.exists():persist()
    while epoch<EPOCHS:
        order=np.random.default_rng(SEED+epoch).permutation(n)
        total_loss=torch.zeros((),device='mps');count=0
        while offset<n:
            if stop or time.time()>=deadline:
                persist();raise TimeoutError('registered_deadline_or_signal')
            ids=torch.from_numpy(order[offset:offset+BATCH]).to('mps')
            a=values['train'][ids];b=values['train'][train_pairs[ids]]
            fraction=steps/max(1,total_steps-1)
            lr=.00003+(.001-.00003)*.5*(1+math.cos(math.pi*fraction))
            for group in optimizer.param_groups:group['lr']=lr
            pred=model(a[:,:48]);other=model(b[:,:48])
            regression=F.huber_loss(pred,a[:,48],delta=1.)
            difference=(a[:,48]+a[:,49])-(b[:,48]+b[:,49])
            weight=torch.clamp(torch.abs(difference),max=4.)
            margin=((pred+a[:,49])-(other+b[:,49]))*torch.sign(difference)
            rank=(F.softplus(-margin)*weight).sum()/torch.clamp(weight.sum(),min=1.)
            loss=regression+.25*rank
            optimizer.zero_grad(set_to_none=True);loss.backward();optimizer.step()
            total_loss+=loss.detach();count+=1;offset+=len(ids);steps+=1
            if steps%512==0:persist()
            if time.monotonic()-last_report>=30:
                duration=initial_elapsed+time.monotonic()-started
                save(directory/'status.json',dict(stage='training',epoch=epoch+1,epochs=EPOCHS,steps=steps,total_steps=total_steps,
                      mean_loss=float(total_loss/count),learning_rate=lr,elapsed_seconds=duration,
                      remaining_seconds=duration*(total_steps-steps)/max(1,steps),device='mps',updated_at=now()))
                last_report=time.monotonic()
        epoch+=1;offset=0
        if epoch%10==0 or epoch==1:
            metrics=dict(epoch=epoch,steps=steps,train_loss=float(total_loss/max(1,count)),
                         development=evaluate(model,values['development'],pairs['development']),
                         elapsed_seconds=initial_elapsed+time.monotonic()-started)
            history.append(metrics);save(directory/f'epoch{epoch:03d}_metrics.json',metrics)
        persist()
    final=evaluate(model,values['development'],pairs['development'])
    first=model.first.weight.detach().cpu().numpy().astype(np.float64)
    bias=model.first.bias.detach().cpu().numpy().astype(np.float64)
    cpp_w=(first/scale).astype(np.float32)
    cpp_b=(bias-(first*mean/scale).sum(1)).astype(np.float32)
    export=dict(identity=identity,features=FEATURES,parameters={k:v.detach().cpu().tolist() for k,v in model.state_dict().items()},
                cpp=dict(w1=cpp_w.tolist(),b1=cpp_b.tolist(),w2=model.last.weight.detach().cpu()[0].tolist(),b2=float(model.last.bias.detach().cpu()[0])),
                epochs=epoch,dataset_sha256=identity['dataset_sha256'])
    save(directory/'model.json',export)
    result=dict(epochs=epoch,updates=steps,metrics=final,mae_improved=final['mae']<final['baseline_mae'],history=history,
                checkpoint_sha256=sha(latest),seconds=initial_elapsed+time.monotonic()-started,completed_at=now())
    save(directory/'result.json',result);save(directory/'status.json',dict(stage='completed',**result))
    return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);p.add_argument('--gpu-state',type=Path,required=True)
    a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2)
    assert torch.backends.mps.is_available()
    torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    report=GPUReport(a.gpu_state)
    try:train(a.run)
    finally:report.close()
