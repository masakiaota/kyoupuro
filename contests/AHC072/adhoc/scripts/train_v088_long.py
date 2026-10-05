#!/usr/bin/env python3
"""v086のAdam状態とデータを保持して、固定600周まで延長する。"""
import argparse
import json
from pathlib import Path
import signal
import time

import numpy as np
import torch
from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from train_v086_policy import model_new, tensors, update, evaluate, generated_sets
from v086_data import Dataset, ROOT, SEED, BATCH, save, sha, status, now

SOURCE=ROOT/'results/nn_rank/v086/20261002T181701_studio'
SOURCE_SHA='509c4e39beb9065c33d0746a6d6fe917499bb841aecaca4df69757c90f060d9c'


def train(run, until, seconds):
    run.mkdir(parents=True,exist_ok=True)
    assert until in (180,600)
    assert sha(SOURCE/'latest.pt')==SOURCE_SHA
    data=Dataset(SOURCE);fingerprint=sha(SOURCE/'dataset.json');ids=data.splits['train']
    model=model_new('mps');opt=torch.optim.Adam(model.parameters(),lr=.001)
    origin=run/'latest.pt' if (run/'latest.pt').exists() else SOURCE/'latest.pt'
    stored=torch.load(origin,map_location='cpu',weights_only=False)
    assert stored['dataset_sha256']==fingerprint
    epoch,offset,steps,history=(stored[k] for k in ('epoch','offset','steps','history'))
    if origin==SOURCE/'latest.pt':assert (epoch,offset,steps)==(60,0,18900)
    if epoch>=until:raise ValueError('requested endpoint already completed')
    model.load_state_dict(stored['model']);opt.load_state_dict(stored['optimizer'])
    torch.set_rng_state(stored['torch_rng']);torch.mps.set_rng_state(stored['mps_rng'])
    if not (run/'config.json').exists():
        save(run/'config.json',{'source':str(SOURCE),'source_checkpoint_sha256':SOURCE_SHA,'dataset_sha256':fingerprint,
                               'endpoints':[180,600],'save_every_epochs':10,'seed':SEED,'learning_rate':.001,'batch':BATCH,
                               'source_code_sha256':sha(Path(__file__))})
    interrupted=False
    def stop(signum,frame):
        nonlocal interrupted
        interrupted=True
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    def persist(path=None):
        checkpoint(path or run/'latest.pt',{'model':{k:v.detach().cpu() for k,v in model.state_dict().items()},
                       'optimizer':opt.state_dict(),'epoch':epoch,'offset':offset,'steps':steps,'history':history,
                       'dataset_sha256':fingerprint,'seed':SEED,'torch_rng':torch.get_rng_state(),'mps_rng':torch.mps.get_rng_state()})
    started=time.monotonic();deadline=started+seconds;start_steps=steps;last_report=started
    total_steps=int(np.ceil(len(ids)/BATCH))*until
    while epoch<until:
        order=np.random.default_rng(SEED+epoch).permutation(ids)
        while offset<len(order):
            if interrupted or time.monotonic()>=deadline:
                persist();status(run,'interrupted',epoch=epoch,offset=offset,steps=steps);raise TimeoutError('limit_or_signal')
            selected=order[offset:offset+BATCH];raw=data.batch(selected,epoch+1)
            loss=update(model,opt,tensors(raw,'mps'),len(ids)/data.description['contributing_inputs']['train'])
            offset+=len(selected);steps+=1
            if steps%512==0:persist()
            if time.monotonic()-last_report>=30:
                rate=(steps-start_steps)/(time.monotonic()-started)
                status(run,'training',epoch=epoch+1,steps=steps,total_steps=total_steps,loss=loss,
                       updates_per_second=rate,remaining_seconds=(total_steps-steps)/rate)
                last_report=time.monotonic()
        epoch+=1;offset=0
        if epoch%10==0:
            measured={'epoch':epoch,'steps':steps,'train':evaluate(model,data,'train','mps'),
                      'validation':evaluate(model,data,'validation','mps')}
            history.append(measured);save(run/f'metrics_epoch{epoch:03d}.json',measured)
            persist(run/f'epoch{epoch:03d}.pt')
        persist()
        if epoch in (180,600):
            save(run/f'model_epoch{epoch:03d}.json',{'parameters':{k:v.detach().cpu().tolist() for k,v in model.state_dict().items()},
                 'mean':data.description['mean'],'scale':data.description['scale'],'seed':SEED,'epochs':epoch,'dataset_sha256':fingerprint})
            save(run/f'result_epoch{epoch:03d}.json',{'final':history[-1],'generated_validation_sets':generated_sets(model,data),
                 'checkpoint_sha256':sha(run/f'epoch{epoch:03d}.pt'),'completed_at':now(),
                 'meaning':'固定した学習期間の比較。損失と模倣診断であり、実探索の短縮量ではない。'})
    status(run,'completed' if epoch==600 else 'checkpoint_180_completed',epoch=epoch,steps=steps,seconds=time.monotonic()-started)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);p.add_argument('--until',type=int,required=True)
    p.add_argument('--seconds',type=int,default=7000);a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2);torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    a.run.mkdir(parents=True,exist_ok=True)
    reporter=GPUReport(a.run/'gpu_state.json')
    try:train(a.run.resolve(),a.until,a.seconds)
    finally:reporter.close()
