#!/usr/bin/env python3
"""容量比較を固定予算で行い、学習入力の自走まで自動で確認する。"""
import argparse
import fcntl
from pathlib import Path
import os
import shutil
import subprocess
import sys
import time
import traceback

from v089_data import ROOT, save, sha, status, now
from v091_env import load, BC_RUN
from v107_capacity import RUN, INITIAL, INITIAL_SHA, prepare


def execute(root):
    pipe=root/'pipeline'; pipe.mkdir(parents=True, exist_ok=True)
    lock=(pipe/'lock').open('a'); fcntl.flock(lock, fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (root/'diagnostic/result.json').exists():
        return
    assert sha(INITIAL)==INITIAL_SHA and load(root/'mechanism/result.json')['passed']
    paths=[ROOT/'adhoc/scripts'/n for n in ('v107_capacity.py','train_v107_capacity.py','run_v107_diagnostic.py',
           'train_v103_imitation.py','train_v090_board.py','v101_compute.py','v091_env.py','v089_data.py',
           'assess_v099_complete.py','build_v090_board.py','check_v089_board.py','run_v089_evaluation.py',
           'v089_core.cpp.txt','v090_inference.cpp.txt','v090_search.cpp.txt','memory_guard.py')]
    identity=dict(initial_sha256=INITIAL_SHA,sources={str(p.relative_to(ROOT)):sha(p) for p in paths})
    if (pipe/'config.json').exists():
        assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity)
        for p in paths+[ROOT/'notes/experiments/v107.md']:
            destination=pipe/'frozen'/p.relative_to(ROOT); destination.parent.mkdir(parents=True,exist_ok=True); shutil.copy2(p,destination)
    def stage(name, command, marker):
        if marker.exists():
            return
        for path,digest in identity['sources'].items():
            assert sha(ROOT/path)==digest, path
        status(pipe,name,pid=os.getpid())
        with (pipe/(name+'.log')).open('a') as stream:
            proc=subprocess.run([sys.executable]+list(map(str,command)),cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(pipe/(name+'_exit.json'),dict(exit_code=proc.returncode,finished_at=now()))
        assert proc.returncode==0 and marker.exists(), name
    started=time.monotonic()
    try:
        for capacity in ('small','wide'):
            where=root/'diagnostic'/capacity
            stage(capacity+'_train',[ROOT/'adhoc/scripts/train_v107_capacity.py','--run',where,'--data',root/'diagnostic/data',
                  '--capacity',capacity,'--diagnostic','--gpu-state',root/'gpu_state.json'],where/'training/result.json')
        stage('assess',[Path(__file__),'--run',root,'--assess'],root/'diagnostic/result.json')
        save(pipe/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,finished_at=now()))
        status(pipe,'diagnostic_completed',next_stage='v106 learning diagnostic, then preregistered qualifying main training')
    except BaseException as e:
        save(pipe/'exit.json',dict(exit_code=1,error=repr(e),traceback=traceback.format_exc(),finished_at=now()))
        status(pipe,'failed',error=repr(e)); raise


def assess(root):
    import numpy as np
    import torch
    from assess_v099_complete import numerical, evaluate
    torch.set_num_threads(2); torch.set_num_interop_threads(2)
    def compare(current, before):
        old={r['case']:r for r in before['rows']}
        assert set(old)=={r['case'] for r in current['rows']}
        assert len(current['rows'])==len(old)==64
        common=[(old[r['case']],r) for r in current['rows'] if old[r['case']]['E']==r['E']==0]
        return dict(baseline=before['metrics'],current=current['metrics'],common_completed=len(common),
            mean_T_difference_on_common=float(np.mean([b['T']-a['T'] for a,b in common])) if common else None,
            mean_S_difference_all=current['metrics']['mean_S_all']-before['metrics']['mean_S_all'],
            completion_difference=current['metrics']['complete']-before['metrics']['complete'],
            wins_on_common=sum(b['T']<a['T'] for a,b in common),ties_on_common=sum(b['T']==a['T'] for a,b in common),
            losses_on_common=sum(b['T']>a['T'] for a,b in common))
    description=prepare(root)
    cases=[dict(c,filename=f"{c['index']:06d}.txt") for c in description['cases']]
    baseline=root/'diagnostic/baseline'
    (baseline/'training').mkdir(parents=True,exist_ok=True)
    model=ROOT/'results/nn_rank/v105/20261004_nn_lns_studio/training/model.json'
    target=baseline/'training/model.json'
    if target.exists():
        assert sha(target)==sha(model)
    else:
        shutil.copy2(model,target)
    # 同じ165回の量子化重みと実行経路。診断入力は今回だけ測る。
    source=ROOT/'adhoc/bin/v105_nn.cpp'
    before=evaluate(baseline,'capacity_train',cases,source)
    results={}; eligible=[]
    for capacity in ('small','wide'):
        where=root/'diagnostic'/capacity
        source=ROOT/'adhoc/bin'/f'v107_{capacity}_diagnostic.cpp'
        numerical(where,source)
        result=evaluate(where,'capacity_train',cases,source)
        comparison=compare(result,before)
        learning=load(where/'training/result.json')
        ratio=learning['final']['ce']/learning['initial']['ce']
        m=result['metrics']
        passed=(learning['complete_budget'] and m['complete']==len(cases) and m['deadline_cases']==0
                and comparison['mean_T_difference_on_common']<=-1 and ratio<=.9)
        results[capacity]=dict(rollout=result,comparison=comparison,learning=learning,ce_ratio=ratio,gate_passed=passed)
        save(where/'assessment.json',results[capacity])
        if passed:
            eligible.append(capacity)
    chosen=min(eligible,key=lambda c:(results[c]['rollout']['metrics']['mean_T_completed'],results[c]['learning']['final']['ce'],
                                     26866 if c=='small' else 52738)) if eligible else None
    pair=compare(results['wide']['rollout'],results['small']['rollout'])
    supported=(results['wide']['gate_passed'] and pair['mean_T_difference_on_common']<=-1
               and results['wide']['learning']['final']['ce']<=.95*results['small']['learning']['final']['ce'])
    save(root/'diagnostic/result.json',dict(diagnostic_only=True,baseline=before,results=results,chosen_capacity=chosen,
         capacity_supported=supported,wide_vs_small=pair,completed_at=now()))


if __name__=='__main__':
    p=argparse.ArgumentParser(); p.add_argument('--run',type=Path,default=RUN); p.add_argument('--assess',action='store_true')
    a=p.parse_args()
    assess(a.run) if a.assess else execute(a.run)
