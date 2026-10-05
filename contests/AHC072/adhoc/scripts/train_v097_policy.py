#!/usr/bin/env python3
"""同じ教師抽出順で一括採点と三段階方策を短時間BCする。"""
import argparse
from pathlib import Path
import signal
import time
import numpy as np
import torch
import torch.nn.functional as F
from v097_policy import ROOT,RUN,PARENT,BC_RUN,CONFIG,ChoiceData,ChoiceEngine,build_library,initial,loss,tensors
from v091_env import load
from v090_data import save,sha,now,status
from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport


def metric_frames(root,data):
    p=root/'metric_frames.json'
    if p.exists():return load(p)
    rng=np.random.default_rng(97001);result={}
    for role in ('train','validation'):
        cases=[c for c in data.cases if c['role']==role][:256];ids=[]
        for c in cases:ids.extend(c['frame_start']+rng.integers(c['frames'],size=2))
        result[role]=dict(ids=list(map(int,ids)),groups=rng.integers(8,size=len(ids)).tolist())
    save(p,result);return result


@torch.no_grad()
def evaluate(model,data,frames,variant,device):
    model.eval();total=0;correct=np.zeros(4);losses=np.zeros(3)
    for start in range(0,len(frames['ids']),32):
        ids=frames['ids'][start:start+32];groups=frames['groups'][start:start+32];raw,codes,counts=data.raw(ids,groups);B=len(ids)
        if variant=='factor':
            f=data.factor(raw,codes,counts);batch=tensors(f,device);logits=model(batch)
            matches=[]
            for i,(logit,key) in enumerate(zip(logits,('p_target','k_target','dl_target'))):
                losses[i]+=float(F.cross_entropy(logit,batch[key],reduction='sum'))
                matches.append((logit.argmax(1)==batch[key]).cpu().numpy())
            for i,m in enumerate(matches):correct[i]+=m.sum()
            correct[3]+=(matches[0]&matches[1]&matches[2]).sum()
        else:
            logits,_=model(tensors(raw,device));logs=F.log_softmax(logits,-1).cpu().numpy();prob=np.exp(logs.astype(np.float64))
            for b in range(B):
                A=counts[b];target=int(raw['target'][b]);src=raw['src'][b,:A];ks=(codes[b,:A]>>9)&7;p=int(src[target]);k=int(ks[target]);v=prob[b,:A]
                pp=np.bincount(src,weights=v,minlength=400);mask=src==p;kp=np.bincount(ks[mask].astype(np.int64),weights=v[mask],minlength=8)
                dl=np.flatnonzero(mask&(ks==k));correct[0]+=pp.argmax()==p;correct[1]+=kp.argmax()==k
                correct[2]+=dl[logs[b,dl].argmax()]==target;correct[3]+=logs[b,:A].argmax()==target
                lp=np.log(max(pp[p],1e-300));lk=np.log(max(kp[k],1e-300))-lp;ld=float(logs[b,target])-lp-lk
                losses-=np.array([lp,lk,ld])
        total+=B
    return dict(samples=total,policy_ce=float(losses.sum()/total),tower_ce=losses[0]/total,split_ce=losses[1]/total,move_ce=losses[2]/total,
                tower_accuracy=correct[0]/total,split_accuracy_given_teacher_tower=correct[1]/total,
                move_accuracy_given_teacher_prefix=correct[2]/total,operation_accuracy=correct[3]/total)


def export(root,model,variant,step,path):
    save(path,dict(parameters={k:v.detach().cpu().tolist() for k,v in model.state_dict().items()},spec=model.spec,size='small',epoch=step,
                  dataset_sha256=sha(BC_RUN/'data/dataset.json'),initial_checkpoint_sha256=sha(PARENT/'initial/checkpoint.pt'),variant=variant,step=step))


def train(root,variant,device):
    d=root/variant;d.mkdir(parents=True,exist_ok=True)
    if (d/'result.json').exists():return load(d/'result.json')
    data=ChoiceData(BC_RUN/'data');engine=ChoiceEngine(build_library(root/'environment'),data);data.engine=engine
    model,optimizer=initial(variant,device);rng=np.random.default_rng(CONFIG['seed']);frames=metric_frames(root,data)
    identity=dict(config=CONFIG,variant=variant,parent_sha256=sha(PARENT/'initial/checkpoint.pt'),metric_frames_sha256=sha(root/'metric_frames.json'),device=device)
    if (d/'config.json').exists():assert load(d/'config.json')==identity
    else:save(d/'config.json',identity)
    steps=0;previous_seconds=0.;history=[];stop=False
    if (d/'latest.pt').exists():
        saved=torch.load(d/'latest.pt',map_location='cpu',weights_only=False);assert saved['identity']==identity
        model.load_state_dict(saved['model']);optimizer.load_state_dict(saved['optimizer']);rng.bit_generator.state=saved['rng']
        steps=saved['steps'];previous_seconds=saved['seconds'];history=saved['history']
    started=time.monotonic()
    def seconds():return previous_seconds+time.monotonic()-started
    def persist():
        checkpoint(d/'latest.pt',dict(identity=identity,model={k:v.detach().cpu() for k,v in model.state_dict().items()},optimizer=optimizer.state_dict(),
                                     rng=rng.bit_generator.state,steps=steps,seconds=seconds(),history=history))
    def measure():
        record=dict(step=steps,seconds=seconds(),**{role:evaluate(model,data,frames[role],variant,device) for role in frames})
        history.append(record);save(d/'curves.json',history);status(d,'classification_measured',step=steps,seconds=seconds())
    def interrupt(signum,frame):
        nonlocal stop
        stop=True
    signal.signal(signal.SIGTERM,interrupt);signal.signal(signal.SIGINT,interrupt)
    try:
        if not history:measure();export(root,model,variant,0,d/'initial_model.json')
        while steps<CONFIG['steps'] and seconds()<CONFIG['seconds']:
            if stop:persist();raise InterruptedError('BC stopped at checkpoint boundary')
            ids,groups=data.draw(rng,CONFIG['batch']);raw,codes,counts=data.raw(ids,groups)
            batch=tensors(data.factor(raw,codes,counts) if variant=='factor' else raw,device)
            model.train();objective=loss(model,batch,variant);optimizer.zero_grad(set_to_none=True);objective.backward()
            norm=torch.nn.utils.clip_grad_norm_(model.parameters(),CONFIG['gradient_clip'],error_if_nonfinite=True);optimizer.step();steps+=1
            if steps%200==0:
                persist();status(d,'training',step=steps,seconds=seconds(),loss=float(objective.detach()),gradient_norm=float(norm),remaining_seconds=max(0.,CONFIG['seconds']-seconds()))
            if steps%CONFIG['interval']==0:
                measure();persist();export(root,model,variant,steps,d/f'model_step{steps:05d}.json')
        persist();result=dict(variant=variant,steps=steps,seconds=seconds(),last_shared_milestone=steps//CONFIG['interval']*CONFIG['interval'],history=history,completed_at=now())
        save(d/'result.json',result);status(d,'completed',steps=steps,seconds=seconds());return result
    finally:engine.close()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);p.add_argument('--variant',choices=('flat','factor'),required=True);p.add_argument('--device',choices=('cpu','mps'),default='mps');a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2)
    if a.device=='mps':torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    report=GPUReport(a.run/'gpu_state.json') if a.device=='mps' else None
    try:train(a.run.resolve(),a.variant,a.device)
    finally:
        if report:report.close()
