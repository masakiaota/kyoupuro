#!/usr/bin/env python3
"""入力単位の交差検証で追加短縮を学ぶ。モデル容量と学習予算は固定する。"""
import argparse
import json
import math
import time
import numpy as np
import torch
from torch import nn
from build_v124_allocation import ROOT, RUN
from v089_data import save, sha, now, status

def load(path):return json.loads(path.read_text())
def model_new():return nn.Sequential(nn.Linear(18,16),nn.ReLU(),nn.Linear(16,1))


def dataset():
    assert load(RUN/'collection/exit.json')['exit_code']==0
    cases=[c for c in load(RUN/'input_manifest.json') if c['role']=='train']
    rows=[];identities={};coverage=[]
    for case in cases:
        count=[]
        for repeat in [124101,124102,124103,124104]:
            directory=RUN/'data'/f"{case['index']:04d}"/str(repeat)
            report=load(directory/'result.json');count.append(report['count'])
            path=directory/'labels.jsonl';identities[str(path.relative_to(RUN))]=sha(path)
            for line in path.read_text().splitlines():
                row=json.loads(line);row.pop('path');rows.append(dict(row,case=case['index'],repeat=repeat))
        coverage.append(dict(case=case['index'],counts=count,eligible=min(count)>=3))
    identity=RUN/'training/data_identity.json'
    if identity.exists():assert load(identity)==identities
    else:save(identity,identities)
    save(RUN/'training/coverage.json',coverage)
    features=torch.tensor([r['features'] for r in rows],dtype=torch.float32)
    targets=torch.tensor([(r['before']-r['after'])/20 for r in rows],dtype=torch.float32)
    order=np.random.default_rng(124003).permutation(128)
    folds=np.empty(128,dtype=np.int64)
    for i,group in enumerate(np.array_split(order,4)):folds[group]=i
    save(RUN/'training/folds.json',folds.tolist())
    return rows,features,targets,folds,coverage


def train_one(name,x,y):
    where=RUN/'training'/name;where.mkdir(parents=True,exist_ok=True)
    if (where/'model.json').exists():return load(where/'model.json')
    torch.manual_seed(124004);model=model_new();optimizer=torch.optim.AdamW(model.parameters(),lr=.001,weight_decay=.0001)
    checkpoint=where/'latest.pt';begin=0;history=[]
    if checkpoint.exists():
        state=torch.load(checkpoint,map_location='cpu',weights_only=False)
        model.load_state_dict(state['model']);optimizer.load_state_dict(state['optimizer']);begin=state['epoch'];history=state['history'];torch.set_rng_state(state['rng'])
    tick=time.monotonic()
    for epoch in range(begin,500):
        lr=.0001+.5*(.001-.0001)*(1+math.cos(math.pi*epoch/499))
        for g in optimizer.param_groups:g['lr']=lr
        order=torch.randperm(len(x));total=0.
        for ids in order.split(128):
            predicted=model(x[ids]).squeeze(1);loss=nn.functional.smooth_l1_loss(predicted,y[ids])
            optimizer.zero_grad(set_to_none=True);loss.backward();optimizer.step();total+=float(loss.detach())*len(ids)
        history.append(dict(epoch=epoch+1,loss=total/len(x),lr=lr))
        if (epoch+1)%50==0:
            torch.save(dict(model=model.state_dict(),optimizer=optimizer.state_dict(),epoch=epoch+1,history=history,rng=torch.get_rng_state()),where/'latest.tmp')
            (where/'latest.tmp').replace(checkpoint);status(RUN/'training','training',model=name,epoch=epoch+1)
    result=dict(parameters={k:v.tolist() for k,v in model.state_dict().items()},checkpoint_sha256=sha(checkpoint),
                architecture=[18,16,1],epochs=500,rows=len(x),seconds=time.monotonic()-tick,completed_at=now())
    save(where/'history.json',history);save(where/'model.json',result);return result


def run(full=False):
    torch.set_num_threads(4);torch.set_num_interop_threads(1)
    rows,x,y,folds,coverage=dataset()
    if full:
        assert load(RUN/'cross_validation/result.json')['passed']
        train_one('full',x,y);return
    if sum(c['eligible'] for c in coverage)<96:
        result=dict(passed=False,reason='insufficient_distinct_plans',eligible_cases=sum(c['eligible'] for c in coverage),
                    cases=128,rows=len(rows),completed_at=now())
        save(RUN/'training/result.json',result);status(RUN/'training','completed',**result);return
    predictions=np.zeros(len(rows))
    for fold in range(4):
        mask=torch.tensor([folds[r['case']]==fold for r in rows])
        trained=train_one(f'fold_{fold}',x[~mask],y[~mask]);model=model_new()
        model.load_state_dict({k:torch.tensor(v) for k,v in trained['parameters'].items()});model.eval()
        with torch.no_grad():predictions[mask.numpy()]=20*model(x[mask]).squeeze(1).numpy()
    predictions=np.clip(predictions,0,np.array([r['before'] for r in rows]))
    groups={}
    for i,r in enumerate(rows):
        if r['phase']==0:groups.setdefault((r['case'],r['repeat']),[]).append(i)
    pairs=[]
    for (case,repeat),indices in groups.items():
        base=min(indices,key=lambda i:(rows[i]['before'],rows[i]['candidate']))
        chosen=min(indices,key=lambda i:(rows[i]['before']-predictions[i],rows[i]['before'],rows[i]['candidate']))
        incumbent=rows[base]['shortest']
        original=min(incumbent,rows[base]['after']);current=min(incumbent,rows[chosen]['after'])
        order=sorted(indices,key=lambda i:(rows[i]['before'],rows[i]['candidate']))
        discarded=order[(len(indices)+1)//2:]
        oracle=min(incumbent,min(rows[i]['after'] for i in indices))
        pairs.append(dict(case=case,repeat=repeat,fold=int(folds[case]),baseline=original,learned=current,
                          difference=current-original,oracle=oracle,changed=chosen!=base,
                          chosen_discarded=chosen in discarded,discarded_oracle=min([incumbent]+[rows[i]['after'] for i in discarded])))
    by_case=[]
    for case in range(128):
        subset=[p for p in pairs if p['case']==case]
        by_case.append(dict(case=case,fold=int(folds[case]),difference=float(np.mean([p['difference'] for p in subset])) if subset else 0.,groups=len(subset)))
    fold_differences=[float(np.mean([r['difference'] for r in by_case if r['fold']==f])) for f in range(4)]
    mean=float(np.mean([r['difference'] for r in by_case]));eligible=sum(c['eligible'] for c in coverage)
    result=dict(passed=eligible>=96 and mean<=-.25 and sum(d<0 for d in fold_differences)>=3,
                eligible_cases=eligible,cases=128,rows=len(rows),groups=len(pairs),mean_difference=mean,fold_differences=fold_differences,
                prediction_mae=float(np.mean(np.abs(predictions-20*y.numpy()))),
                oracle_mean_difference=float(np.mean([p['oracle']-p['baseline'] for p in pairs])) if pairs else 0.,
                changed=sum(p['changed'] for p in pairs),chosen_discarded=sum(p['chosen_discarded'] for p in pairs),
                features=18,parameters=sum(p.numel() for p in model_new().parameters()),completed_at=now())
    save(RUN/'training/diagnostic_pairs.json',pairs);save(RUN/'training/diagnostic_cases.json',by_case)
    save(RUN/'training/result.json',result);status(RUN/'training','completed',**result)
    np.savez_compressed(RUN/'training/predictions.npz',predictions=predictions,features=x.numpy())
    print(json.dumps(result,ensure_ascii=False))


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--full',action='store_true');run(parser.parse_args().full)
