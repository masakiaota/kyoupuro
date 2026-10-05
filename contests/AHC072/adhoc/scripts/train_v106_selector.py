#!/usr/bin/env python3
"""事前登録した集合評価モデルを、NN由来の列で学習する。"""
import argparse
from datetime import datetime
import json
from pathlib import Path
import signal
import time

import numpy as np
import torch
from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from train_v087_value import model_new, tensors, update, evaluate, check, SEED, BATCH, EPOCHS
from v106_data import Dataset, teacher_quality
from v089_data import save, sha, status, now


def train(run,seconds):
    if (run/'result.json').exists():return
    data=Dataset(run);quality=teacher_quality(run)
    assert quality['gate_passed'], 'teacher gate must pass before learning'
    if not (run/'learning_check.json').exists():check(run,data)
    assert json.loads((run/'learning_check.json').read_text())['passed']
    fingerprint=sha(run/'dataset.json');model=model_new('mps');opt=torch.optim.Adam(model.parameters(),lr=.001)
    epoch=offset=steps=0;history=[];elapsed=0.;latest=run/'latest.pt';ids=data.splits['train']
    identity=dict(dataset_sha256=fingerprint,seed=SEED,batch=BATCH,epochs=EPOCHS,seconds=seconds,lr=.001,
                  source_sha256=sha(Path(__file__)))
    config=run/'training_config.json'
    if config.exists():assert json.loads(config.read_text())==identity
    else:save(config,identity)
    if latest.exists():
        stored=torch.load(latest,map_location='cpu',weights_only=False);assert stored['identity']==identity
        model.load_state_dict(stored['model']);opt.load_state_dict(stored['optimizer'])
        epoch,offset,steps,history,elapsed=(stored[k] for k in ('epoch','offset','steps','history','elapsed'))
        torch.set_rng_state(stored['torch_rng']);torch.mps.set_rng_state(stored['mps_rng'])
    else:
        save(run/'initial_metrics.json',{'train':evaluate(model,data,'train','mps'),'validation':evaluate(model,data,'validation','mps')})
    interrupted=False
    def stop(signum,frame):
        nonlocal interrupted
        interrupted=True
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    started=time.monotonic();initial_elapsed=elapsed;last_report=started;initial_steps=steps
    total_steps=int(np.ceil(len(ids)/BATCH))*EPOCHS
    deadline=datetime.fromisoformat('2026-10-04T18:00:00+09:00').timestamp()
    def persist(path=None):
        checkpoint(path or latest,dict(model={k:v.detach().cpu() for k,v in model.state_dict().items()},optimizer=opt.state_dict(),
             identity=identity,epoch=epoch,offset=offset,steps=steps,history=history,elapsed=initial_elapsed+time.monotonic()-started,
             torch_rng=torch.get_rng_state(),mps_rng=torch.mps.get_rng_state()))
    while epoch<EPOCHS:
        order=np.random.default_rng(SEED+epoch).permutation(ids)
        while offset<len(order):
            elapsed=initial_elapsed+time.monotonic()-started
            if interrupted or elapsed>=seconds or time.time()>=deadline:
                persist();raise TimeoutError('registered_deadline_or_signal')
            selected=order[offset:offset+BATCH];batch=tensors(data.batch(selected),'mps')
            loss=update(model,opt,batch,len(ids)/data.input_counts['train']);offset+=len(selected);steps+=1
            if steps%256==0:persist()
            if time.monotonic()-last_report>=30:
                rate=(steps-initial_steps)/(time.monotonic()-started)
                status(run,'training',epoch=epoch+1,steps=steps,total_steps=total_steps,loss=loss,
                       remaining_seconds=(total_steps-steps)/rate,device='mps')
                last_report=time.monotonic()
        epoch+=1;offset=0
        if epoch in (10,30,60):
            measured=dict(epoch=epoch,train=evaluate(model,data,'train','mps'),validation=evaluate(model,data,'validation','mps'))
            history.append(measured);save(run/f'metrics_epoch{epoch:02d}.json',measured);persist(run/f'epoch{epoch:02d}.pt')
        persist()
    final=history[-1]['validation'];rows=list(final['per_input'].values())
    # 共通の既存選択をv087のv079欄へ渡している。v086欄も同じ選択であり別対照ではない。
    diffs=np.array([r['model_gain']-r['v079_gain'] for r in rows]);n=len(diffs)
    draws=np.random.default_rng(106002).integers(0,n,(6000,n));ci=np.percentile(diffs[draws].mean(1),[2.5,97.5])
    gain=float(diffs.mean());passed=bool(gain>=.05 and ci[0]>0 and quality['gate_passed'])
    save(run/'model.json',dict(parameters={k:v.detach().cpu().tolist() for k,v in model.state_dict().items()},
                             mean=data.mean.tolist(),scale=data.scale.tolist(),epochs=EPOCHS,dataset_sha256=fingerprint))
    result=dict(final=final,additional_gain=gain,bootstrap95=ci.tolist(),gate_passed=passed,
                teacher_gate_passed=quality['gate_passed'],epochs=epoch,updates=steps,
                seconds=initial_elapsed+time.monotonic()-started,checkpoint_sha256=sha(latest),
                completed_at=now(),meaning='各入力の状態を平均した、一度の修復候補選択の利益。完成手数の改善とは区別する。')
    save(run/'result.json',result);status(run,'completed',additional_gain=gain,bootstrap95=ci.tolist(),gate_passed=passed)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);p.add_argument('--seconds',type=int,required=True)
    p.add_argument('--gpu-state',type=Path,required=True);a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2)
    torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    report=GPUReport(a.gpu_state)
    try:train(a.run,a.seconds)
    except BaseException as e:
        save(a.run/'failure.json',dict(error=repr(e),time=now()));status(a.run,'failed',error=repr(e));raise
    finally:report.close()
