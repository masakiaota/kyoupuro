#!/usr/bin/env python3
"""新規公式盤面のPPOで、旧教師のみと選別逆教師併用を比較する。"""
import argparse
from datetime import datetime
import gc
import json
from pathlib import Path
import signal
import time

import numpy as np
import torch

from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from train_v090_board import Model, tensors
from train_v091_ppo import collect
from v090_data import Dataset, save, sha, status, now
from v091_env import load, build_library
from v092_stream import ROOT, BC_RUN, TeacherData, OfficialQueue, FreshEpisodes, excluded_inputs
from v096_selected import RUN,configuration,ReverseData,ReverseEngine,optimize


def start_model(root, device, config):
    saved=torch.load(root/'initial/checkpoint.pt',map_location='cpu',weights_only=False)
    model=Model(config['model_spec']).to(device);model.load_state_dict(saved['model'])
    optimizer=torch.optim.AdamW(model.parameters(),lr=config['initial_lr'],weight_decay=config['weight_decay'])
    optimizer.load_state_dict(saved['optimizer'])
    return model,optimizer


def export(root, model, iteration, steps, path):
    save(path,dict(parameters={k:v.detach().cpu().tolist() for k,v in model.state_dict().items()},
         spec=model.spec,size='small',epoch=iteration,rl_iterations=iteration,environment_steps=steps,
         dataset_sha256=sha(BC_RUN/'data/dataset.json'),initial_checkpoint_sha256=sha(root/'initial/checkpoint.pt'),
         seed=96096,method="PPO with selected reverse BC auxiliary" if root.name=="mixed" else "PPO with original BC auxiliary"))


def train(root,device,teacher_root,mechanism=False):
    directory=root/'training';directory.mkdir(parents=True,exist_ok=True)
    if (directory/'result.json').exists():return load(directory/'result.json')
    config=configuration(root.name,mechanism)
    identity=dict(config=config,initial_checkpoint_sha256=sha(root/'initial/checkpoint.pt'),
                  teacher_dataset_sha256=sha(BC_RUN/'data/dataset.json'),reverse_dataset_sha256=sha(teacher_root/'teachers/dataset.json'),device=device)
    if (directory/'config.json').exists():assert load(directory/'config.json')==identity
    else:save(directory/'config.json',identity)
    data=TeacherData(BC_RUN/'data');reverse=ReverseData(teacher_root);engine=ReverseEngine(build_library(root/'environment'),data,reverse,config['workers']);data.engine=engine
    model,optimizer=start_model(root,device,config);rng=np.random.default_rng(config['seed'])
    torch.manual_seed(config['seed'])
    if device=='mps':torch.mps.manual_seed(config['seed'])
    saved=None;iteration=steps=episode_count=successes=0;previous_seconds=0.;history=[]
    if (directory/'latest.pt').exists():
        saved=torch.load(directory/'latest.pt',map_location='cpu',weights_only=False);assert saved['identity']==identity
        model.load_state_dict(saved['model']);optimizer.load_state_dict(saved['optimizer'])
        rng.bit_generator.state=saved['rng'];torch.set_rng_state(saved['torch_rng'])
        if device=='mps':torch.mps.set_rng_state(saved['mps_rng'])
        iteration,steps,episode_count,successes,previous_seconds,history=(saved[k] for k in ('iteration','steps','episode_count','successes','seconds','history'))
        for item in saved['active_inputs']:engine.register(engine.from_text(item))
    blocked=excluded_inputs();save(directory/'excluded_sha256.json',sorted(blocked))
    queue=OfficialQueue(root,engine,blocked,config,saved['queue'] if saved else None)
    episodes=FreshEpisodes(engine,queue,rng,config['environments'],saved['episodes'] if saved else None)
    queue.wait_full()
    del saved
    started=time.monotonic();starting_iteration=iteration;stopped=False;last_report=0.
    def seconds():return previous_seconds+time.monotonic()-started
    def stop(signum,frame):
        nonlocal stopped
        stopped=True
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    def persist():
        queue_state=queue.snapshot()
        try:
            checkpoint(directory/'latest.pt',dict(identity=identity,model={k:v.detach().cpu() for k,v in model.state_dict().items()},
                optimizer=optimizer.state_dict(),rng=rng.bit_generator.state,torch_rng=torch.get_rng_state(),
                mps_rng=torch.mps.get_rng_state() if device=='mps' else None,queue=queue_state,episodes=episodes.state_dict(),
                active_inputs=engine.serializable(),iteration=iteration,steps=steps,seconds=seconds(),history=history,
                episode_count=episode_count,successes=successes))
        finally:queue.resume()
    def report(phase,part):
        nonlocal last_report
        if time.monotonic()-last_report<30:return
        status(directory,phase,iteration=iteration,steps=steps,part=part,seconds=seconds(),
               remaining_seconds=max(0.,config['seconds']-seconds()),queue_ready=len(queue.ready),
               active_geometries=len(engine.live),bc_cache=len(engine.handles),device=str(next(model.parameters()).device))
        last_report=time.monotonic()
    reason='iteration_budget'
    try:
        while iteration<config['iterations']:
            if stopped:persist();raise InterruptedError('stopped at checkpoint boundary')
            if seconds()>=config['seconds']:reason='time_budget';break
            if datetime.now().astimezone()>=datetime.fromisoformat(config['final_deadline']):reason='deadline';break
            queue.set_update_phase(False);tick=time.monotonic();buffer,finished=collect(model,engine,episodes,rng,device,config,lambda n:report('collecting',n))
            rollout_seconds=time.monotonic()-tick;queue.set_update_phase(True);tick=time.monotonic()
            progress=min(1.,max(iteration/config['iterations'],seconds()/config['seconds']))
            metrics=optimize(model,optimizer,engine,data,buffer,rng,device,config,progress,lambda n:report('updating',n))
            update_seconds=time.monotonic()-tick;iteration+=1;steps+=len(buffer['ids']);episode_count+=len(finished)
            complete=[r for r in finished if r['E']==0];successes+=len(complete)
            for row in finished:
                row.pop('start_remaining_teacher',None);row['seed']=engine.live[row['case']]['seed']
            engine.release_except(episodes.ids)
            assert len(engine.live)==config['environments'] and len(engine.handles)<=config['bc_cache_capacity'] and len(engine.reverse_handles)<=128
            row=dict(iteration=iteration,steps=steps,seconds=seconds(),rollout_seconds=rollout_seconds,update_seconds=update_seconds,
                     episodes=len(finished),successes=len(complete),mean_T_completed=float(np.mean([r['T'] for r in complete])) if complete else None,
                     mean_E_finished=float(np.mean([r['E'] for r in finished])) if finished else None,queue_wait_seconds=queue.wait_seconds,
                     queue_ready=len(queue.ready),active_geometries=len(engine.live),bc_cache=len(engine.handles),**metrics)
            history.append(row);persist()
            with (directory/'metrics.jsonl').open('a') as stream:stream.write(json.dumps(row,allow_nan=False)+'\n')
            with (directory/'episodes.jsonl').open('a') as stream:
                for item in finished:stream.write(json.dumps(dict(iteration=iteration,**item))+'\n')
            status(directory,'iteration_completed',**row,remaining_seconds=min(max(0.,config['seconds']-seconds()),
                (config['iterations']-iteration)*(seconds()-previous_seconds)/(iteration-starting_iteration)))
            print(json.dumps(row,allow_nan=False),flush=True);del buffer
        assert iteration>0
        persist();export(root,model,iteration,steps,directory/'model.json')
        result=dict(iterations=iteration,environment_steps=steps,seconds=seconds(),reason=reason,episodes=episode_count,successes=successes,
                    history=history,final=history[-1],checkpoint_sha256=sha(directory/'latest.pt'),model_sha256=sha(directory/'model.json'),completed_at=now())
        save(directory/'result.json',result);status(directory,'training_completed',iterations=iteration,steps=steps,reason=reason)
        return result
    finally:queue.close();engine.close()


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);p.add_argument('--mechanism',action='store_true')
    p.add_argument('--device',choices=('cpu','mps'),default='mps');p.add_argument('--teachers',type=Path,default=RUN);args=p.parse_args();root=args.run.resolve()
    torch.set_num_threads(2);torch.set_num_interop_threads(2)
    if args.device=='mps':torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    reporter=GPUReport(root.parent/'gpu_state.json') if args.device=='mps' else None
    try:train(root,args.device,args.teachers.resolve(),args.mechanism)
    finally:
        if reporter:reporter.close()
