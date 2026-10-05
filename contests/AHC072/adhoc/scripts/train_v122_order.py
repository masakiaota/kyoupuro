#!/usr/bin/env python3
"""固定した順序教師をCPUで200周学習し、最後の重みだけを診断する。"""
import gzip
import json
import math
import os
import time

import numpy as np
import torch
from torch import nn

from build_v122_order import ROOT, RUN
from v089_data import save, sha, now, status


def load(path):
    return json.loads(path.read_text())


def model_new():
    return nn.Sequential(nn.Linear(80, 16), nn.ReLU(), nn.Linear(16, 1))


def dataset():
    features, costs, masks, original_alt, roles, cases, allowance = [], [], [], [], [], [], []
    identities = {}
    for case in load(RUN / 'inputs.json'):
        name = f"{case['index']:06d}"
        where = RUN / 'data' / name
        identities[name] = sha(where / 'rows.jsonl.gz')
        with gzip.open(where / 'rows.jsonl.gz', 'rt') as stream:
            for line in stream:
                row = json.loads(line)
                if not row['extracted']:
                    continue
                count = len(row['costs'])
                f = np.zeros((24, 80), dtype=np.float32)
                f[:count] = row['features']
                c = np.full(24, row['allowance'] + 8, dtype=np.float32)
                c[:count] = row['costs']
                features.append(f);costs.append(c);masks.append(np.arange(24) < count)
                original_alt.append(row['alternate']);roles.append(case['order_role'])
                cases.append(case['index']);allowance.append(row['allowance'])
    data = dict(features=torch.from_numpy(np.asarray(features)), costs=torch.from_numpy(np.asarray(costs)),
                mask=torch.from_numpy(np.asarray(masks)), alternate=torch.tensor(original_alt),
                train=torch.tensor([r == 'train' for r in roles]), cases=cases, allowance=torch.tensor(allowance))
    marker=RUN/'training/data_identity.json'
    if marker.exists():assert load(marker)==identities
    else:save(marker,identities)
    return data


def assess(model, data, indices):
    predictions = []
    with torch.no_grad():
        for ids in indices.split(256):
            scores = model(data['features'][ids]).squeeze(-1)
            scores = scores.masked_fill(~data['mask'][ids], 1e9)
            scores[:, 0] = 1e9
            predictions.append(scores.argmin(-1))
    selected = torch.cat(predictions)
    cost = data['costs'][indices]
    base = torch.minimum(cost[:, 0], cost.gather(1, data['alternate'][indices, None]).squeeze(1))
    chosen = torch.minimum(cost[:, 0], cost.gather(1, selected[:, None]).squeeze(1))
    limits = data['allowance'][indices]
    valid_cost = cost.masked_fill(~data['mask'][indices], 1e9)
    oracle = valid_cost.min(-1).values
    delta = chosen - base
    result = dict(groups=len(indices),cases=len({data['cases'][i] for i in indices.tolist()}),
                  baseline_complete=int((base <= limits).sum()),model_complete=int((chosen <= limits).sum()),
                  mean_difference=float(delta.mean()),sum_difference=float(delta.sum()),
                  oracle_mean_difference=float((oracle-base).mean()),
                  wins=int((delta<0).sum()),ties=int((delta==0).sum()),losses=int((delta>0).sum()),
                  changed_alternate=int((selected != data['alternate'][indices]).sum()),
                  selected_best=int((chosen==oracle).sum()),
                  complete_regressions=int(((base<=limits)&(chosen>limits)).sum()),
                  complete_rescues=int(((base>limits)&(chosen<=limits)).sum()))
    result['passed'] = result['model_complete'] >= result['baseline_complete'] and result['mean_difference'] <= -.10
    return result, selected


def train():
    assert load(RUN / 'pilot/result.json')['passed']
    assert load(RUN / 'full/result.json')['cases'] == 320
    torch.set_num_threads(4);torch.set_num_interop_threads(1)
    torch.manual_seed(122003);np.random.seed(122003)
    directory=RUN/'training';directory.mkdir(exist_ok=True)
    config=dict(architecture=[80,16,1],activation='relu',device='cpu',threads=4,epochs=200,batch=128,
                initial_lr=.001,final_lr=.0001,weight_decay=.0001,seed=122003,temperature=1.)
    if (directory/'config.json').exists():assert load(directory/'config.json')==config
    else:save(directory/'config.json',config)
    data=dataset();model=model_new();optimizer=torch.optim.AdamW(model.parameters(),lr=.001,weight_decay=.0001)
    complete=data['costs'].min(-1).values<=data['allowance']
    training=torch.nonzero(data['train']&complete).flatten()
    development=torch.nonzero(~data['train']).flatten()
    begin=0;checkpoint=directory/'latest.pt';history=[]
    if checkpoint.exists():
        state=torch.load(checkpoint,map_location='cpu',weights_only=False)
        model.load_state_dict(state['model']);optimizer.load_state_dict(state['optimizer']);begin=state['epoch']
        torch.set_rng_state(state['rng']);history=state['history']
    started=time.monotonic()
    for epoch in range(begin,200):
        lr=.0001+.5*(.001-.0001)*(1+math.cos(math.pi*epoch/199))
        for group in optimizer.param_groups:group['lr']=lr
        shuffled=training[torch.randperm(len(training))];total=0.;count=0
        model.train()
        for ids in shuffled.split(128):
            values=model(data['features'][ids]).squeeze(-1)
            logits=(-values).masked_fill(~data['mask'][ids],-1e9)
            targets=torch.softmax((-data['costs'][ids]).masked_fill(~data['mask'][ids],-1e9),dim=-1)
            loss=-(targets*torch.log_softmax(logits,dim=-1)).sum(-1).mean()
            optimizer.zero_grad(set_to_none=True);loss.backward();optimizer.step()
            total+=float(loss.detach())*len(ids);count+=len(ids)
        history.append(dict(epoch=epoch+1,loss=total/count,lr=lr,seconds=time.monotonic()-started))
        if (epoch+1)%10==0:
            status(directory,'training',epoch=epoch+1,epochs=200,loss=total/count,pid=os.getpid())
            tmp=directory/'latest.tmp'
            torch.save(dict(model=model.state_dict(),optimizer=optimizer.state_dict(),epoch=epoch+1,
                            rng=torch.get_rng_state(),history=history,config=config),tmp);tmp.replace(checkpoint)
            save(directory/'history.json',history)
    model.eval()
    result={}
    for label,ids in [('train',torch.nonzero(data['train']).flatten()),('development',development)]:
        result[label],selected=assess(model,data,ids)
        np.savez_compressed(directory/(label+'_selection.npz'),indices=ids.numpy(),selected=selected.numpy())
    save(directory/'model.json',dict(parameters={k:v.tolist() for k,v in model.state_dict().items()},
                                     config=config,checkpoint_sha256=sha(checkpoint)))
    result.update(checkpoint_sha256=sha(checkpoint),parameters=sum(p.numel() for p in model.parameters()),
                  seconds=time.monotonic()-started,completed_at=now())
    save(directory/'result.json',result);status(directory,'completed',**result)


if __name__=='__main__':train()
