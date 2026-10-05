#!/usr/bin/env python3
"""学習用の4乱数と独立な4乱数で、報酬差と選択利益の再現性を記録する。"""
import argparse
import json
from pathlib import Path
import numpy as np
from v086_data import save


def analyze(run):
    dataset=json.loads((run/'dataset.json').read_text());cases=[];sufficient=np.zeros(6)
    for item in dataset['cases']:
        index=item['input']['index'];folder=run/'cases'/f'{index:06d}'
        if not item['statistics']['groups']:continue
        valid=np.load(folder/'valid.npy');outcome=np.load(folder/'outcome.npy');T=np.load(folder/'current_T.npy')
        sources=np.load(folder/'sources.npy');gains=np.where(outcome>0,np.maximum(0,T[:,None,None]-outcome),0)
        first=gains[:,:,:4].mean(-1);second=gains[:,:,4:].mean(-1);deploy=(sources&3)!=0
        picked=np.where(deploy,first,-np.inf).argmax(-1);old=((sources&4)!=0).argmax(-1);policy=((sources&8)!=0).argmax(-1)
        at=np.arange(len(T));row={'index':index,'role':item['input']['role'],'groups':len(T),
             'teacher_gain':float(second[at,picked].mean()),'v079_gain':float(second[at,old].mean()),
             'v086_gain':float(second[at,policy].mean()),'has_multiple_positive_sets':float(((first>0)&valid).sum(-1).__ge__(2).mean())}
        cases.append(row)
        # 局面ごとの難易度を除いた、候補間差の相関を測る。
        x=first-(first*valid).sum(-1,keepdims=True)/valid.sum(-1,keepdims=True)
        y=second-(second*valid).sum(-1,keepdims=True)/valid.sum(-1,keepdims=True)
        x=x[valid];y=y[valid];sufficient+=np.array([len(x),x.sum(),y.sum(),x@x,y@y,x@y])
    n,sx,sy,sxx,syy,sxy=sufficient;denom=np.sqrt((sxx-sx*sx/n)*(syy-sy*sy/n))
    result={'within_state_half_correlation':float((sxy-sx*sy/n)/denom) if denom else None,
            'roles':{},'per_input':cases,'statistics':dataset['statistics']}
    for role in ('train','validation'):
        subset=[r for r in cases if r['role']==role]
        result['roles'][role]={'inputs':len(subset),**{key:float(np.mean([r[key] for r in subset])) for key in ('teacher_gain','v079_gain','v086_gain','has_multiple_positive_sets')}}
    save(run/'teacher_quality.json',result)
    print(json.dumps({k:v for k,v in result.items() if k!='per_input'}),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);a=p.parse_args();analyze(a.run.resolve())
