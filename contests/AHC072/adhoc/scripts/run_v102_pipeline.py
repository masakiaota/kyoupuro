#!/usr/bin/env python3
"""同じ40採取の補助有無比較と、事前登録した固定検証を再開可能に実行する。"""
import argparse
import fcntl
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback
from v089_data import save,sha,now,status
from v091_env import load
from v099_complete import ROOT,INITIAL,INITIAL_SHA,configuration,BC_RUN
from assess_v099_complete import compare


def configs():
    return {name:dict(configuration('group'),seed=102004,seed_base=1020000000000,iterations=40,seconds=3600,
                      bc_coef=.05 if name=='with_bc' else 0.,bc_batch=64,reverse_batch=0,reverse_coef=0.)
            for name in ('with_bc','without_bc')}


def execute(root):
    pipe=root/'pipeline';pipe.mkdir(parents=True,exist_ok=True)
    lock=(pipe/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (pipe/'exit.json').exists() and load(pipe/'exit.json')['exit_code']==0:return
    assert load(root/'mechanism/result.json')['passed'] and sha(INITIAL)==INITIAL_SHA
    names=['run_v102_pipeline.py','train_v102_ablation.py','v102_learning.py','v101_compute.py','assess_v102_ablation.py',
           'v099_complete.py','v092_stream.py','v091_env.py','v090_data.py','train_v090_board.py','train_v089_board.py',
           'train_v077_rank.py','train_v080_scaling.py','build_v090_board.py','v090_search.cpp.txt','v090_inference.cpp.txt',
           'v089_core.cpp.txt','v089_data.py','check_v089_board.py','run_v089_evaluation.py','memory_guard.py']
    paths=[ROOT/'adhoc/scripts'/x for x in names]+[ROOT/'adhoc/bin/v091_environment.cpp',ROOT/'scripts/eval.py',ROOT/'scripts/build_solver.sh']
    identity=dict(initial_sha256=INITIAL_SHA,configs=configs(),sources={str(p.relative_to(ROOT)):sha(p) for p in paths},
                  dataset_sha256=sha(BC_RUN/'data/dataset.json'))
    if (pipe/'config.json').exists():assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity)
        for path in paths+[ROOT/'notes/experiments/v102.md']:
            dest=pipe/'frozen'/path.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,dest)
    def stage(name,script,args,marker):
        if marker.exists():return
        for path,fingerprint in identity['sources'].items():assert sha(ROOT/path)==fingerprint,f'changed source: {path}'
        status(pipe,name,pid=os.getpid(),marker=str(marker))
        with (pipe/f'{name}.log').open('a') as stream:
            proc=subprocess.run([sys.executable,ROOT/'adhoc/scripts'/script]+list(map(str,args)),cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(pipe/f'{name}_exit.json',dict(exit_code=proc.returncode,finished_at=now()))
        assert proc.returncode==0 and marker.exists(),f'{name} failed; inspect saved log'
    started=time.monotonic()
    try:
        # 学習中に検証結果を使わず、両方の最終重みを先に確定する。
        for name,config in configs().items():
            where=root/name;(where/'initial').mkdir(parents=True,exist_ok=True)
            path=where/'initial/checkpoint.pt'
            if not path.exists():shutil.copy2(INITIAL,path)
            assert sha(path)==INITIAL_SHA
            path=where/'run_config.json'
            if path.exists():assert load(path)==config
            else:save(path,config)
            stage(name+'_train','train_v102_ablation.py',['--run',where,'--config',path,'--gpu-state',root/'gpu_state.json'],where/'training/result.json')
        for name in ('baseline','with_bc','without_bc'):
            where=root/name
            stage(name+'_assess','assess_v102_ablation.py',['--run',where,'--name',name],where/'assessment.json')
        assessments={name:load(root/name/'assessment.json') for name in ('baseline','with_bc','without_bc')}
        complete_budget=all(load(root/name/'training/result.json')['iterations']==40 for name in configs())
        greedy={name:compare(a['greedy'],assessments['baseline']['greedy']) for name,a in assessments.items() if name!='baseline'}
        pair=compare(assessments['without_bc']['greedy'],assessments['with_bc']['greedy'])
        # 確率的4列は入力×乱数で対応づけ、完走率と全入力費用を別に比較する。
        stochastic={}
        for name in ('with_bc','without_bc'):
            base=assessments['baseline']['stochastic'];cur=assessments[name]['stochastic']
            old={r['case']:r for r in base['rows']};common=[(old[r['case']],r) for r in cur['rows'] if old[r['case']]['E']==r['E']==0]
            stochastic[name]=dict(baseline=base['metrics'],current=cur['metrics'],common_completed=len(common),
                mean_T_difference_on_common=sum(b['T']-a['T'] for a,b in common)/len(common) if common else None,
                mean_S_difference_all=cur['metrics']['mean_S_all']-base['metrics']['mean_S_all'])
        eligible=[]
        for name,comparison in greedy.items():
            m=assessments[name]['greedy']['metrics']
            if complete_budget and m['complete']==256 and m['deadline_cases']==0 and comparison['mean_S_difference_all']<0 and comparison['mean_T_difference_on_common']<0:
                eligible.append(name)
        candidate=min(eligible,key=lambda n:(assessments[n]['greedy']['metrics']['mean_S_all'],assessments[n]['greedy']['metrics']['mean_elapsed_ms'],list(configs()).index(n))) if eligible else 'baseline'
        supported=complete_budget and pair['completion_difference']>=0 and pair['mean_S_difference_all']<0 and pair['mean_T_difference_on_common'] is not None and pair['mean_T_difference_on_common']<0
        result=dict(complete_budget=complete_budget,greedy=greedy,without_vs_with=pair,stochastic=stochastic,
                    removing_auxiliary_supported=supported,candidate=candidate,independent_test_used=False,tools_in_used=False,completed_at=now())
        save(root/'result.json',result)
        save(pipe/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,finished_at=now()))
        status(pipe,'completed',result=str(root/'result.json'),next_stage='verify and record; continue approved B-148')
    except BaseException as error:
        save(pipe/'exit.json',dict(exit_code=1,error=repr(error),traceback=traceback.format_exc(),finished_at=now()))
        status(pipe,'failed',error=repr(error));raise


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);a=p.parse_args();execute(a.run.resolve())
