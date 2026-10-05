#!/usr/bin/env python3
"""最初の保存軌跡と初期重みを固定し、損失項の勾配を更新せずに比較する。"""
import argparse
from pathlib import Path
import time
import numpy as np
import torch
import torch.nn.functional as F
from v089_data import save, sha, now
from v091_env import load, build_library
from v092_stream import TeacherData, describe
from train_v090_board import tensors
from v099_complete import RUN, BC_RUN, INITIAL, INITIAL_SHA, configuration, start_model, FreshEngine, AuxiliaryPool


def execute(out):
    out.mkdir(parents=True,exist_ok=False);started=time.monotonic()
    source=RUN/'mc_ppo/training/episodes/0001.json';saved=load(source);config=configuration('mc_ppo')
    assert sha(INITIAL)==INITIAL_SHA
    data=TeacherData(BC_RUN/'data');engine=FreshEngine(build_library(RUN/'mc_ppo/environment'),data,4);data.engine=engine
    model,_=start_model(INITIAL,'cpu',config);model.eval()
    original={k:v.detach().clone() for k,v in model.state_dict().items()}
    params=[p for n,p in model.named_parameters() if not n.startswith('critic.')]
    def gradient(loss,retain=True):
        values=torch.autograd.grad(loss,params,retain_graph=retain,allow_unused=True)
        return torch.cat([(g if g is not None else torch.zeros_like(p)).detach().flatten() for g,p in zip(values,params)]).numpy().astype(np.float64)
    try:
        for item in saved['inputs']:engine.register(engine.from_text(item))
        episodes=saved['rows'];ids=np.array([r['case'] for r in episodes],np.int32)
        states=np.stack([describe(engine.live[int(i)]['text'])[0] for i in ids])
        lengths=np.array([r['T'] for r in episodes]);groups=np.array([r['group'] for r in episodes],np.int32)
        parts={k:[] for k in ('states','ids','groups','codes','returns','group_advantage')}
        for t in range(int(lengths.max())):
            slots=np.flatnonzero(lengths>t);current=states[slots].copy()
            codes=np.array([episodes[i]['actions'][t] for i in slots],np.uint32)
            for k,v in dict(states=current.copy(),ids=ids[slots],groups=groups[slots],codes=codes,
                            returns=-(lengths[slots]-t)/100.,
                            group_advantage=np.array([episodes[i]['group_advantage'] for i in slots])).items():parts[k].append(v)
            left,dead=engine.step(ids[slots],current,codes);states[slots]=current
            for j,i in enumerate(slots):
                if lengths[i]==t+1:assert left[j]==episodes[i]['E']==0
        assert not states.any()
        buffer={k:np.concatenate(v) for k,v in parts.items()};del parts
        order=np.random.default_rng(config['seed']+2*1009).permutation(len(buffer['ids']))
        pool=AuxiliaryPool(data,engine,np.random.default_rng(config['seed']+3*1009),config)
        scale=len(order)/(len(episodes)*config['loss_reference_steps'])
        totals={};batches=[];advantages=[]
        for i in range(8):
            index=order[i*256:(i+1)*256]
            raw,codes,_=engine.observe(buffer['ids'][index],buffer['states'][index],buffer['groups'][index])
            match=(codes==buffer['codes'][index,None]) & raw['valid'];assert (match.sum(1)==1).all()
            batch=tensors(raw,'cpu');logits,costs=model(batch);lp=F.log_softmax(logits,-1)
            selected=lp.gather(1,torch.from_numpy(match.argmax(1))[:,None]).squeeze(1)
            returns=torch.from_numpy(buffer['returns'][index].astype(np.float32))
            mc_adv=returns+costs.detach()/100.;advantages.extend(mc_adv.tolist())
            losses=dict(mc_policy=-(selected*mc_adv).mean()*scale,
                        group_policy=-(selected*torch.from_numpy(buffer['group_advantage'][index].astype(np.float32))).mean()*scale,
                        value=.5*F.smooth_l1_loss(-costs/100.,returns),
                        entropy=.001*(lp.exp()*lp).sum(-1).mean())
            gradients={k:gradient(v) for k,v in losses.items()}
            bc=tensors(pool.original(),'cpu');bc_logits,_=model(bc)
            gradients['bc']=gradient(.05*F.cross_entropy(bc_logits,bc['target']),False)
            for k,g in gradients.items():totals[k]=totals.get(k,np.zeros_like(g))+g/8
            batches.append({k:float(np.linalg.norm(g)) for k,g in gradients.items()})
            print('batch',i+1,batches[-1],flush=True)
        assert all(torch.equal(original[k],v) for k,v in model.state_dict().items())
        def cosine(a,b):return float(np.dot(a,b)/(np.linalg.norm(a)*np.linalg.norm(b)))
        report={}
        for name in ('mc','group'):
            policy=totals[name+'_policy'];aux=totals['bc']+totals['entropy']+(totals['value'] if name=='mc' else 0)
            report[name]=dict(policy_norm=float(np.linalg.norm(policy)),auxiliary_norm=float(np.linalg.norm(aux)),
                             auxiliary_to_policy_norm=float(np.linalg.norm(aux)/np.linalg.norm(policy)),
                             policy_auxiliary_cosine=cosine(policy,aux),policy_total_cosine=cosine(policy,policy+aux),
                             component_cosines={k:cosine(policy,totals[k]) for k in ('bc','entropy','value')})
        result=dict(initial_sha256=INITIAL_SHA,episode_file_sha256=sha(source),transitions=2048,bc_frames=512,
                    weights_unchanged=True,optimizer_steps=0,new_policy_rollouts=0,device='cpu',
                    analyzed_parameters='shared trunk and actor; critic output parameters excluded',
                    component_mean_gradient_norms={k:float(np.linalg.norm(g)) for k,g in totals.items()},
                    mean_minibatch_gradient_norms={k:float(np.mean([r[k] for r in batches])) for k in totals},
                    mc_advantage_std=float(np.std(advantages)),
                    group_episode_advantage_std=float(np.std([r['group_advantage'] for r in episodes])),
                    comparisons=report,seconds=time.monotonic()-started,completed_at=now(),
                    limitation='Initial checkpoint and first cohort only; raw gradients before AdamW, not a causal performance test.')
        save(out/'result.json',result);np.savez(out/'mean_gradients.npz',**totals);print(result,flush=True)
    finally:engine.close()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2);execute(a.out.resolve())
