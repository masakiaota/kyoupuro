#!/usr/bin/env python3
"""同じ保存操作列・重みで、形状固定と先読みの速度および勾配を照合する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
import time
from pathlib import Path
import numpy as np
import torch
from v089_data import save, sha, now
from v091_env import load, build_library
from v092_stream import TeacherData, describe
from v099_complete import RUN, BC_RUN, INITIAL, INITIAL_SHA, configuration, start_model, FreshEngine, AuxiliaryPool
from v101_compute import distribution, loss, imitation, owned
from train_v080_scaling import GPUReport


def replay(engine, saved):
    for item in saved['inputs']: engine.register(engine.from_text(item))
    rows = saved['rows']
    ids = np.array([r['case'] for r in rows], np.int32)
    groups = np.array([r['group'] for r in rows], np.int32)
    states = np.stack([describe(engine.live[int(i)]['text'])[0] for i in ids])
    lengths = np.array([r['T'] for r in rows])
    parts = {k: [] for k in ('states', 'ids', 'groups', 'codes', 'advantages', 'returns')}
    slices = []
    for t in range(int(lengths.max())):
        slots = np.flatnonzero(lengths > t)
        current = states[slots].copy()
        codes = np.array([rows[i]['actions'][t] for i in slots], np.uint32)
        item = dict(states=current.copy(), ids=ids[slots], groups=groups[slots], codes=codes,
                    advantages=np.array([rows[i]['group_advantage'] for i in slots], np.float32),
                    returns=(-(lengths[slots]-t)/100.).astype(np.float32))
        if t % 8 == 0 and len(slices) < 64: slices.append(item)
        for key, value in item.items(): parts[key].append(value)
        left, dead = engine.step(ids[slots], current, codes)
        states[slots] = current
        assert not dead.any()
        for j, i in enumerate(slots):
            if lengths[i] == t+1: assert left[j] == rows[i]['E'] == 0
    assert not states.any()
    # 短い保存列でも64個に揃える。新しい方策の試行はしない。
    slices = (slices * ((64+len(slices)-1)//len(slices)))[:64]
    return {k: np.concatenate(v) for k, v in parts.items()}, slices


def grad_vector(model):
    return torch.cat([p.grad.detach().flatten().cpu() if p.grad is not None else torch.zeros(p.numel())
                      for p in model.parameters()])


def execute(out):
    out.mkdir(parents=True, exist_ok=True)
    assert not (out/'result.json').exists(), 'completed benchmark must not be repeated'
    source = RUN/'mc_ppo/training/episodes/0001.json'
    assert sha(INITIAL) == INITIAL_SHA
    config = configuration('group')
    data = TeacherData(BC_RUN/'data')
    engine = FreshEngine(build_library(RUN/'mc_ppo/environment'), data, 8)
    data.engine = engine
    started = time.monotonic()
    try:
        buffer, inference = replay(engine, load(source))
        order = np.random.default_rng(101004).permutation(len(buffer['ids']))[:32*256].reshape(32,256)
        model, _ = start_model(INITIAL, 'mps', config)
        cpu, _ = start_model(INITIAL, 'cpu', config)
        initial = {k: v.detach().cpu().clone() for k, v in model.state_dict().items()}
        scale = len(buffer['ids'])/(128*config['loss_reference_steps'])
        def observe(item):
            raw, codes, _ = engine.observe(item['ids'], item['states'], item['groups'])
            match = (codes == item['codes'][:,None]) & raw['valid']
            assert (match.sum(1) == 1).all()
            return dict(raw=owned(raw), action=match.argmax(1), advantages=item['advantages'], returns=item['returns'])
        def producer():
            pool = AuxiliaryPool(data, engine, np.random.default_rng(101005), config)
            for indices in order:
                item = observe({k:v[indices] for k,v in buffer.items()})
                item['bc'] = owned(pool.original())
                yield item
        def compute(m, item, device, bucket):
            m.zero_grad(set_to_none=True)
            total, _ = loss(m, item['raw'], item['action'], None, item['advantages'], item['returns'], device, config, scale, bucket)
            total = total + config['bc_coef'] * imitation(m, item['bc'], device, bucket)
            total.backward()
            if device == 'mps': torch.mps.synchronize()
            return total.detach()
        if (out/'numerical.json').exists():
            numerical=load(out/'numerical.json');errors=numerical['errors'];adam_error=numerical['adam_max_error']
        else:
            errors = []
            for item in list_from(producer(), 4):
                a = compute(model,item,'mps',False); ga = grad_vector(model)
                b = compute(model,item,'mps',True); gb = grad_vector(model)
                c = compute(cpu,item,'cpu',True); gc = grad_vector(cpu)
                errors.append(dict(loss_bucket=float(abs(a-b)), loss_cpu=float(abs(a.cpu()-c)),
                                   gradient_bucket=float(torch.linalg.vector_norm(ga-gb)/torch.linalg.vector_norm(ga)),
                                   gradient_cpu=float(torch.linalg.vector_norm(ga-gc)/torch.linalg.vector_norm(ga))))
            # AdamWの実際の1更新も、同じ状態から照合する。
            other, opt_other = start_model(INITIAL,'mps',config)
            first, opt_first = start_model(INITIAL,'mps',config)
            item = next(producer())
            for m,opt,bucket in [(first,opt_first,False),(other,opt_other,True)]:
                compute(m,item,'mps',bucket)
                torch.nn.utils.clip_grad_norm_(m.parameters(),config['gradient_clip'])
                opt.step()
            adam_error = max(float((v-other.state_dict()[k]).abs().max()) for k,v in first.state_dict().items())
            del first, other, opt_first, opt_other
            torch.mps.empty_cache()
            numerical = dict(errors=errors, adam_max_error=adam_error)
            save(out/'numerical.json',numerical)
        assert max(r['gradient_bucket'] for r in errors)<1e-3 and max(r['loss_bucket'] for r in errors)<1e-4
        assert adam_error<1e-5
        cpu_eligible=max(r['gradient_cpu'] for r in errors)<1e-3 and max(r['loss_cpu'] for r in errors)<1e-4
        inference_results = {}
        reference = []
        for bucket in (False,True):
            tick=time.monotonic(); times=[]; shapes=[]; top1_errors=0; prob_error=0.
            for i,item in enumerate(inference):
                t=time.monotonic(); observed=observe(item)
                with torch.no_grad(): lp,val=distribution(model,observed['raw'],'mps',bucket)
                lp=lp.cpu().numpy(); val=val.cpu().numpy()
                n,L=observed['raw']['valid'].shape
                if not bucket: reference.append((lp.copy(),val.copy()))
                else:
                    orig,_=reference[i]; gap=np.sort(orig,axis=1)[:,-1]-np.sort(orig,axis=1)[:,-2]
                    top1_errors+=int(((lp.argmax(1)!=orig.argmax(1)) & (gap>1e-4)).sum())
                    prob_error=max(prob_error,float(np.max(np.abs(np.exp(lp[:,:L])-np.exp(orig)))))
                shapes.append([n,L]);times.append(time.monotonic()-t)
                engine.buffers.clear()
            inference_results[str(bucket)] = dict(seconds=time.monotonic()-tick,times=times,shapes=shapes,
                                                  top1_errors=top1_errors,probability_max_error=prob_error)
            print('inference',bucket,inference_results[str(bucket)]['seconds'],flush=True)
        assert inference_results['True']['top1_errors']==0 and inference_results['True']['probability_max_error']<1e-4
        save(out/'inference.json',inference_results)
        results={}
        for label,bucket,prefetch in [('baseline',False,False),('bucket',True,False),('prefetch',True,True)]:
            tick=time.monotonic(); times=[]
            batches=producer()
            with ThreadPoolExecutor(max_workers=1) as pool:
                future=pool.submit(next,batches) if prefetch else None
                for i in range(32):
                    t=time.monotonic()
                    item=future.result() if prefetch else next(batches)
                    if prefetch and i<31: future=pool.submit(next,batches)
                    compute(model,item,'mps',bucket)
                    times.append(time.monotonic()-t)
            results[label]=dict(seconds=time.monotonic()-tick,times=times)
            print('update',label,results[label]['seconds'],flush=True)
        def sliced(item,start,end):
            return {k:({q:v[start:end] for q,v in value.items()} if k=='raw' else
                       {q:v[start//4:end//4] for q,v in value.items()} if k=='bc' else value[start:end])
                    for k,value in item.items()}
        # 両デバイスで同じ重みを使い、CPU分を件数で重み付けして集約する。
        tick=time.monotonic(); times=[]; hybrid_error=0.
        with ThreadPoolExecutor(max_workers=1) as pool:
            for i,item in enumerate(producer()):
                t=time.monotonic()
                cpu.load_state_dict({k:v.detach().cpu() for k,v in model.state_dict().items()})
                future=pool.submit(compute,cpu,sliced(item,192,256),'cpu',True)
                compute(model,sliced(item,0,192),'mps',True); future.result()
                for p,q in zip(model.parameters(),cpu.parameters()):
                    if p.grad is not None: p.grad.mul_(.75).add_(q.grad.to('mps'),alpha=.25)
                torch.mps.synchronize()
                times.append(time.monotonic()-t)
                if i==0:
                    hybrid=grad_vector(model)
                    compute(model,item,'mps',True)
                    normal=grad_vector(model)
                    hybrid_error=float(torch.linalg.vector_norm(hybrid-normal)/torch.linalg.vector_norm(normal))
        results['hybrid']=dict(seconds=time.monotonic()-tick,times=times,gradient_relative_error=hybrid_error)
        cpu_eligible = cpu_eligible and hybrid_error<1e-3
        assert all(torch.equal(initial[k],v.cpu()) for k,v in model.state_dict().items())
        base_roll=inference_results['False']['seconds']; new_roll=inference_results['True']['seconds']
        # 時間短縮率はフェーズ別の所要時間比から計算する。
        ratios={label:1/(.46*(new_roll/base_roll)+.54*(row['seconds']/results['baseline']['seconds']))
                for label,row in results.items() if label!='baseline'}
        eligible={k:v for k,v in ratios.items() if k!='hybrid' or cpu_eligible}
        best=max(eligible,key=eligible.get)
        elapsed=time.monotonic()-started
        saved_seconds=7200*(1-1/ratios[best])
        preparation_seconds=time.time()-datetime.fromisoformat('2026-10-04T02:35:00+09:00').timestamp()
        accepted=ratios[best]>=1.15 and saved_seconds>preparation_seconds
        result=dict(passed=True,accepted=accepted,selected=best if accepted else 'baseline',
                    projected_total_speedup=ratios,projected_saved_seconds_in_two_hours=saved_seconds,
                    preparation_seconds=preparation_seconds,cpu_eligible=cpu_eligible,
                    updates=results,inference=inference_results,numerical=numerical,
                    seconds=elapsed,initial_sha256=INITIAL_SHA,source_sha256=sha(source),
                    no_new_policy_rollouts=True,weights_unchanged=True,completed_at=now())
        save(out/'result.json',result);print({k:v for k,v in result.items() if k not in ('updates','inference','numerical')},flush=True)
    finally: engine.close()


def list_from(iterator,n):
    return [next(iterator) for _ in range(n)]


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    torch.set_num_threads(8);torch.set_num_interop_threads(2)
    torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    reporter=GPUReport(a.out.resolve().parent/'gpu_state.json')
    try: execute(a.out.resolve())
    finally: reporter.close()
