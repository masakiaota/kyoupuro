#!/usr/bin/env python3
"""集合の実測短縮量と失敗率を学び、独立乱数で選択利益を比較する。"""
import argparse
import json
import math
from pathlib import Path
import signal
import time

import numpy as np
import torch
from torch import nn
from torch.nn import functional as F
from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from train_v086_policy import Policy
from v087_data import Dataset, ENCODING
from v086_data import ROOT, save, sha, status, now

SEED=87001;BATCH=8;EPOCHS=60


class Value(nn.Module):
    def __init__(self):
        super().__init__();policy=Policy()
        stored=torch.load(ENCODING/'latest.pt',map_location='cpu',weights_only=False)
        policy.load_state_dict(stored['model'])
        self.node=policy.node;self.message=policy.message
        self.head=nn.Sequential(nn.Linear(102,64),nn.ReLU(),nn.Linear(64,2))
    def forward(self,batch):
        h,global_h=Policy.encode(self,batch['nodes'],batch['edges'],batch['valid'])
        mask=batch['mask'];count=mask.sum(-1,keepdim=True).clamp_min(1)
        mean=torch.matmul(mask,h)/count
        maximum=h[:,None].masked_fill(~mask[:,:,:,None].bool(),-1e9).max(2).values
        maximum=maximum*batch['candidate_valid'][:,:,None]
        # 候補内の関係を、候補に含まれる受け手の数で平均する。
        related=(torch.matmul(batch['edges'],mask.transpose(1,2)[:,None])*mask.transpose(1,2)[:,None]).sum(2).permute(0,2,1)/count
        return self.head(torch.cat((global_h[:,None].expand_as(mean),mean,maximum,related,count/12),-1))


def model_new(device):
    torch.manual_seed(SEED)
    if device=='mps':torch.mps.manual_seed(SEED)
    return Value().to(device)


def tensors(raw,device):
    return {k:torch.from_numpy(np.array(v,copy=True)).to(device) for k,v in raw.items() if k not in ('cases','sources','cost_ms')}


def losses(prediction,batch):
    valid=batch['candidate_valid'];denominator=valid.sum(-1).clamp_min(1)
    target=batch['gains'][:,:,:4].mean(-1);failure=batch['failed'][:,:,:4].mean(-1)
    q=prediction[:,:,0];failure_logits=prediction[:,:,1]
    regression=(F.smooth_l1_loss(q,target,reduction='none')*valid).sum(-1)/denominator
    distribution=torch.softmax(target.masked_fill(~valid,-1e9),-1)
    preference=-(distribution*torch.log_softmax(q.masked_fill(~valid,-1e9),-1)).sum(-1)
    auxiliary=(F.binary_cross_entropy_with_logits(failure_logits,failure,reduction='none')*valid).sum(-1)/denominator
    return regression+preference+.1*auxiliary


def update(model,opt,batch,scale):
    loss=(losses(model(batch),batch)*batch['weight']*scale).mean()
    if not math.isfinite(loss.item()):raise FloatingPointError('nonfinite reward loss')
    opt.zero_grad(set_to_none=True);loss.backward();torch.nn.utils.clip_grad_norm_(model.parameters(),5,error_if_nonfinite=True);opt.step()
    return float(loss.item())


@torch.no_grad()
def evaluate(model,data,role,device):
    sums={};ids=data.splits[role]
    for at in range(0,len(ids),BATCH):
        raw=data.batch(ids[at:at+BATCH]);batch=tensors(raw,device);pred=model(batch)
        q=pred[:,:,0].cpu().numpy();loss=losses(pred,batch).cpu().numpy()
        deployment=(raw['sources']&3)!=0
        chosen=np.where(deployment,q,-np.inf).argmax(-1)
        old=((raw['sources']&4)!=0).argmax(-1);policy=((raw['sources']&8)!=0).argmax(-1)
        fit_gain=raw['gains'][:,:,:4].mean(-1);audit_gain=raw['gains'][:,:,4:].mean(-1)
        oracle=np.where(deployment,fit_gain,-np.inf).argmax(-1)
        for b,case in enumerate(raw['cases']):
            record=sums.setdefault(int(case),{'weight':0.,'loss':0.,'model_gain':0.,'v079_gain':0.,'v086_gain':0.,
                 'teacher_selected_gain':0.,'model_failure':0.,'model_cost_ms':0.,'v079_cost_ms':0.,'v086_cost_ms':0.})
            w=float(raw['weight'][b]);record['weight']+=w;record['loss']+=w*float(loss[b])
            for name,index in (('model',chosen[b]),('v079',old[b]),('v086',policy[b])):
                record[name+'_gain']+=w*float(audit_gain[b,index])
                record[name+'_cost_ms']+=w*float(raw['cost_ms'][b,index,4:].mean())
            record['teacher_selected_gain']+=w*float(audit_gain[b,oracle[b]])
            record['model_failure']+=w*float(raw['failed'][b,chosen[b],4:].mean())
    assert len(sums)==data.input_counts[role]
    for record in sums.values():
        for key in record:
            if key!='weight':record[key]/=record['weight']
    summary={key:float(np.mean([r[key] for r in sums.values()])) for key in next(iter(sums.values())) if key!='weight'}
    summary.update(inputs=len(sums),groups=len(ids),per_input=sums)
    return summary


def check(run,data):
    # 同じ短縮量の2候補を等しく扱い、短縮0へ確率を寄せない勾配を確認する。
    prediction=torch.zeros((1,4,2),requires_grad=True)
    target=torch.tensor([0.,4.,4.,0.])[None,:,None].expand(1,4,8)
    batch={'gains':target,'failed':torch.zeros_like(target),'candidate_valid':torch.ones((1,4),dtype=torch.bool)}
    losses(prediction,batch).sum().backward();gradient=prediction.grad[0,:,0]
    assert gradient[1]==gradient[2] and gradient[1]<0 and gradient[0]>0 and gradient[3]>0
    ids=[];seen=set()
    for i in data.splits['train']:
        case=data.rows[i][0]
        if case in seen:continue
        raw=data.batch([i]);targets=raw['gains'][0,:,:4].mean(-1)[raw['candidate_valid'][0]]
        if targets.max()-targets.min()>=.5:ids.append(i);seen.add(case)
        if len(ids)==4:break
    assert len(ids)==4
    raw=data.batch(ids);raw['weight'][:]=1;batch=tensors(raw,'mps');model=model_new('mps')
    opt=torch.optim.Adam(model.parameters(),lr=.001);fit=None
    for step in range(2000):
        update(model,opt,batch,1)
        if (step+1)%100==0:
            with torch.no_grad():
                q=model(batch)[:,:,0].masked_fill(~batch['candidate_valid'],-1e9)
                target=batch['gains'][:,:,:4].mean(-1)
                regret=target.max(-1).values-target.gather(1,q.argmax(-1)[:,None])[:,0]
            if (regret<=.1).all().item():fit={'steps':step+1,'max_regret':float(regret.max().item())};break
    assert fit is not None,'real_four_fit_failed'
    # 保存後の更新をCPU上で一致させる。機構用の重みは本学習へ持ち込まない。
    model=model.to('cpu');cpu=tensors(raw,'cpu');a=torch.optim.Adam(model.parameters(),lr=.001)
    update(model,a,cpu,1);path=run/'mechanism.pt';checkpoint(path,{'model':model.state_dict(),'optimizer':a.state_dict()})
    other=model_new('cpu');stored=torch.load(path,weights_only=False);other.load_state_dict(stored['model'])
    b=torch.optim.Adam(other.parameters(),lr=.001);b.load_state_dict(stored['optimizer'])
    update(model,a,cpu,1);update(other,b,cpu,1)
    assert all(torch.equal(v,other.state_dict()[k]) for k,v in model.state_dict().items())
    with torch.no_grad():expected=model(cpu).numpy();actual=model.to('mps')(tensors(raw,'mps')).cpu().numpy()
    np.testing.assert_allclose(actual,expected,rtol=2e-5,atol=2e-4)
    result={'passed':True,'multiple_good_targets':True,'real_four':fit,'checkpoint_next_update_exact':True,
            'cpu_mps_max_error':float(abs(actual-expected).max()),'indices':ids}
    positions=np.linspace(0,len(data.splits['train'])-1,BATCH,dtype=int)
    sample=tensors(data.batch([data.splits['train'][i] for i in positions]),'mps')
    model=model_new('mps');opt=torch.optim.Adam(model.parameters(),lr=.001)
    for step in range(50):
        if step==10:torch.mps.synchronize();started=time.monotonic()
        update(model,opt,sample,len(data.splits['train'])/data.input_counts['train'])
    torch.mps.synchronize();result['seconds_per_update']=(time.monotonic()-started)/40
    result['estimated_training_seconds']=result['seconds_per_update']*math.ceil(len(data.splits['train'])/BATCH)*EPOCHS
    save(run/'learning_check.json',result);print(json.dumps(result),flush=True)


def train(run,deadline):
    data=Dataset(run)
    if not (run/'learning_check.json').exists():check(run,data)
    assert json.loads((run/'learning_check.json').read_text())['passed']
    fingerprint=sha(run/'dataset.json');model=model_new('mps');opt=torch.optim.Adam(model.parameters(),lr=.001)
    epoch=offset=steps=0;history=[];latest=run/'latest.pt';ids=data.splits['train']
    if latest.exists():
        stored=torch.load(latest,map_location='cpu',weights_only=False);assert stored['dataset_sha256']==fingerprint
        model.load_state_dict(stored['model']);opt.load_state_dict(stored['optimizer'])
        epoch,offset,steps,history=(stored[k] for k in ('epoch','offset','steps','history'))
        torch.set_rng_state(stored['torch_rng']);torch.mps.set_rng_state(stored['mps_rng'])
    else:
        save(run/'initial_metrics.json',{'train':evaluate(model,data,'train','mps'),'validation':evaluate(model,data,'validation','mps')})
    interrupted=False
    def stop(signum,frame):
        nonlocal interrupted
        interrupted=True
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    def persist(path=None):
        checkpoint(path or latest,{'model':{k:v.detach().cpu() for k,v in model.state_dict().items()},'optimizer':opt.state_dict(),
             'epoch':epoch,'offset':offset,'steps':steps,'history':history,'dataset_sha256':fingerprint,
             'torch_rng':torch.get_rng_state(),'mps_rng':torch.mps.get_rng_state()})
    started=time.monotonic();start_steps=steps;last_report=started;total_steps=math.ceil(len(ids)/BATCH)*EPOCHS
    while epoch<EPOCHS:
        order=np.random.default_rng(SEED+epoch).permutation(ids)
        while offset<len(order):
            if interrupted or time.time()>=deadline:persist();raise TimeoutError('registered_deadline_or_signal')
            selected=order[offset:offset+BATCH];batch=tensors(data.batch(selected),'mps')
            loss=update(model,opt,batch,len(ids)/data.input_counts['train']);offset+=len(selected);steps+=1
            if steps%512==0:persist()
            if time.monotonic()-last_report>=30:
                rate=(steps-start_steps)/(time.monotonic()-started)
                status(run,'training',epoch=epoch+1,steps=steps,total_steps=total_steps,loss=loss,remaining_seconds=(total_steps-steps)/rate)
                last_report=time.monotonic()
        epoch+=1;offset=0
        if epoch in (10,30,60):
            measured={'epoch':epoch,'train':evaluate(model,data,'train','mps'),'validation':evaluate(model,data,'validation','mps')}
            history.append(measured);save(run/f'metrics_epoch{epoch:02d}.json',measured);persist(run/f'epoch{epoch:02d}.pt')
        persist()
    final=history[-1]['validation'];per_input=list(final['per_input'].values());comparisons={}
    for baseline in ('v079','v086'):
        saved=np.array([r['model_gain']-r[baseline+'_gain'] for r in per_input]);n=len(saved)
        draws=np.random.default_rng(87002).integers(0,n,(6000,n));ci=np.percentile(saved[draws].mean(1),[1.25,98.75])
        comparisons[baseline]={'additional_gain':float(saved.mean()),'bootstrap97_5':ci.tolist(),'passed':bool(saved.mean()>=.1 and ci[0]>0)}
    result={'final':final,'comparisons':comparisons,'gate_passed':all(c['passed'] for c in comparisons.values()),
            'completed_at':now(),'checkpoint_sha256':sha(latest),'meaning':'保存前列からの一段階の選択利益。通常探索全体の改善量ではない。'}
    save(run/'result.json',result)
    save(run/'model.json',{'parameters':{k:v.detach().cpu().tolist() for k,v in model.state_dict().items()},
                         'mean':data.mean.tolist(),'scale':data.scale.tolist(),'epochs':EPOCHS,'dataset_sha256':fingerprint})
    status(run,'completed',gate_passed=result['gate_passed'],comparisons=comparisons)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);p.add_argument('--seconds',type=int,default=14000)
    p.add_argument('--mode',choices=('check','train'),default='train');a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2);torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    report=GPUReport(a.run/'gpu_state.json')
    try:
        if a.mode=='check':check(a.run.resolve(),Dataset(a.run.resolve()))
        else:train(a.run.resolve(),time.time()+a.seconds)
    except BaseException as error:
        save(a.run/'failure.json',{'error':repr(error),'time':now()});status(a.run,'failed',error=repr(error));raise
    finally:report.close()
