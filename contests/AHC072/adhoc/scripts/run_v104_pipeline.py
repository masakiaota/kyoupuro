#!/usr/bin/env python3
"""追加学習と事前に固定した3点の比較を終え、最終候補を保存する。"""
import argparse
import fcntl
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback
from v089_data import ROOT,save,sha,status,now
from v091_env import BC_RUN,load
from v104_extended import configuration,INITIAL_SHA,POINTS


def execute(root):
    pipe=root/'pipeline';pipe.mkdir(parents=True,exist_ok=True)
    lock=(pipe/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (pipe/'exit.json').exists() and load(pipe/'exit.json')['exit_code']==0:return
    assert load(root/'mechanism/result.json')['passed'] and sha(root/'initial/checkpoint.pt')==INITIAL_SHA
    names=['v104_extended.py','train_v104_extended.py','check_v104_extended.py','assess_v104_extended.py','run_v104_pipeline.py',
           'v102_learning.py','v101_compute.py','v099_complete.py','v092_stream.py','v091_env.py','v090_data.py','v089_data.py',
           'v096_selected.py','train_v091_ppo.py','train_v090_board.py','train_v089_board.py','train_v077_rank.py','train_v080_scaling.py',
           'assess_v099_complete.py','assess_v102_ablation.py','build_v090_board.py','check_v089_board.py','run_v089_evaluation.py',
           'v089_core.cpp.txt','v090_inference.cpp.txt','v090_search.cpp.txt','memory_guard.py']
    paths=[ROOT/'adhoc/scripts'/x for x in names]+[ROOT/'adhoc/bin/v091_environment.cpp',ROOT/'scripts/eval.py',ROOT/'scripts/build_solver.sh']
    config=configuration();identity=dict(config=config,initial_sha256=INITIAL_SHA,sources={str(p.relative_to(ROOT)):sha(p) for p in paths},
          dataset_sha256=sha(BC_RUN/'data/dataset.json'))
    if (pipe/'config.json').exists():assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity);save(root/'run_config.json',config)
        for path in paths+[ROOT/'notes/experiments/v104.md']:
            dst=pipe/'frozen'/path.relative_to(ROOT);dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,dst)
    def stage(name,script,args,marker):
        if marker.exists():return
        for path,fingerprint in identity['sources'].items():assert sha(ROOT/path)==fingerprint,f'changed source: {path}'
        status(pipe,name,pid=os.getpid(),marker=str(marker))
        with (pipe/(name+'.log')).open('a') as stream:
            process=subprocess.run([sys.executable,ROOT/'adhoc/scripts'/script]+list(map(str,args)),cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(pipe/(name+'_exit.json'),dict(exit_code=process.returncode,finished_at=now()))
        assert process.returncode==0 and marker.exists(),f'{name} failed; inspect saved log'
    started=time.monotonic()
    try:
        stage('train','train_v104_extended.py',['--run',root,'--config',root/'run_config.json','--gpu-state',root/'gpu_state.json'],root/'training/result.json')
        training=load(root/'training/result.json');assessments={}
        for point in POINTS:
            where=root/'points'/f'{point:04d}'
            if not (where/'snapshot.json').exists():continue
            stage(f'assess_{point:04d}','assess_v104_extended.py',['--run',root,'--point',point],where/'assessment.json')
            assessments[str(point)]=load(where/'assessment.json')
        complete=training['iterations']==config['iterations'];accepted=False
        if complete:
            m=assessments['320']['greedy']['metrics'];c=assessments['320']['comparison']
            accepted=m['complete']==256 and m['deadline_cases']==0 and c['mean_S_difference_all']<0 and c['mean_T_difference_on_common']<0
        selection=dict(accepted=accepted,complete_budget=complete,point=320 if accepted else 'v102',fixed_at=now(),
                       intermediate_points_used_for_selection=False)
        final=root/'final';final.mkdir(exist_ok=True)
        if (final/'selection.json').exists():
            previous=load(final/'selection.json');assert {k:v for k,v in previous.items() if k!='fixed_at'}=={k:v for k,v in selection.items() if k!='fixed_at'};selection=previous
        else:save(final/'selection.json',selection)
        if accepted:
            chosen=root/'points/0320';old_source=ROOT/'adhoc/bin/v104_point_0320.cpp';source=ROOT/'src/bin/v104_nn_extended.cpp'
            content='// '+source.name+'\n'+old_source.read_text().split('\n',1)[1]
            if source.exists():assert source.read_text()==content
            else:source.write_text(content)
            (final/'training').mkdir(exist_ok=True);shutil.copy2(chosen/'training/model.json',final/'training/model.json')
            candidate=dict(selection,source=str(source.relative_to(ROOT)),solver_sha256=sha(source),model_sha256=sha(final/'training/model.json'),
                           checkpoint_sha256=sha(chosen/'training/latest.pt'))
            if (final/'candidate.json').exists():assert load(final/'candidate.json')==candidate
            else:save(final/'candidate.json',candidate)
            numerical=load(chosen/'numerical/result.json');assert numerical['passed'] and numerical['model_sha256']==candidate['model_sha256']
            assert numerical['solver_sha256']==sha(old_source)
            save(final/'numerical/result.json',dict(numerical,solver_sha256=sha(source),reused_from=str(chosen/'numerical/result.json')))
            for role in ('test','final_in'):
                stage('final_'+role,'assess_v099_complete.py',['--run',final,'--source',source,'--stage',role],final/'evaluation'/role/'greedy/result.json')
            test=load(final/'evaluation/test/greedy/result.json');tools_in=load(final/'evaluation/final_in/greedy/result.json')
        else:
            previous=ROOT/'results/nn_rank/v103/20261004_self_imitation_studio';old=load(previous/'result.json')
            candidate=dict(selection,**{k:old['final'][k] for k in ('source','solver_sha256','model_sha256','checkpoint_sha256')},reused_from=str(previous))
            assert sha(ROOT/candidate['source'])==candidate['solver_sha256']
            save(final/'candidate.json',candidate);test=old['test'];tools_in=old['tools_in']
        result=dict(selection=selection,candidate=candidate,validation={n:a['comparison'] for n,a in assessments.items()},
                    final_stochastic=assessments.get('320',{}).get('stochastic',{}),test=test,tools_in=tools_in,completed_at=now())
        save(root/'result.json',result);save(pipe/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,finished_at=now()))
        status(pipe,'completed',accepted=accepted,next_stage='verify records, write notes, commit/push and stop heartbeat')
    except BaseException as error:
        save(pipe/'exit.json',dict(exit_code=1,error=repr(error),traceback=traceback.format_exc(),finished_at=now()))
        status(pipe,'failed',error=repr(error));raise


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=Path);a=p.parse_args();execute(a.run.resolve())
