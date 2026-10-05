#!/usr/bin/env python3
"""事前登録した完走学習2方式、逆教師2条件、固定候補の最終評価を順に行う。"""
import argparse
from datetime import datetime
import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback

from v089_data import save, sha, now, status
from v091_env import load
from v099_complete import (ROOT, RUN, REVERSE_RUN, BC_RUN, INITIAL, INITIAL_SHA, REVERSE_SOURCE,
                           configuration, start_model)
from train_v099_complete import export
from assess_v099_complete import compare


def evaluation(root): return load(root/'evaluation/validation/greedy/result.json')


def acceptable(result):
    m=result['metrics']
    return result['all_legal'] and m['complete']==256 and m['deadline_cases']==0


def improvement(current, baseline):
    c=compare(current,baseline)
    return (c['completion_difference']>=0 and c['mean_S_difference_all']<0 and
            c['mean_T_difference_on_common'] is not None and c['mean_T_difference_on_common']<0)


def ordering(result, tie):
    m=result['metrics']
    return (-m['complete'],m['mean_S_all'],m['mean_elapsed_ms'],tie)


def execute(root, reverse_root):
    pipe=root/'pipeline';pipe.mkdir(parents=True,exist_ok=True)
    lock=(pipe/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (pipe/'exit.json').exists() and load(pipe/'exit.json')['exit_code']==0:return
    assert load(root/'mechanism/result.json')['passed'] and sha(INITIAL)==INITIAL_SHA
    names=('v099_complete.py','train_v099_complete.py','check_v099_complete.py','assess_v099_complete.py',
           'prepare_v100_selected.py','check_v100_auxiliary.py','run_v099_pipeline.py','v096_selected.py',
           'v092_stream.py','v091_env.py','train_v091_ppo.py','v090_data.py','train_v090_board.py',
           'build_v090_board.py','v090_inference.cpp.txt','v090_search.cpp.txt','v089_core.cpp.txt','v089_data.py',
           'train_v089_board.py','check_v089_board.py','run_v089_evaluation.py','train_v077_rank.py','train_v080_scaling.py',
           'check_v098_complete.py','memory_guard.py')
    paths=[ROOT/'adhoc/scripts'/n for n in names]
    paths += [ROOT/'adhoc/bin'/n for n in ('v091_environment.cpp','select_v100_teachers.cpp')]
    paths += [ROOT/'scripts/build_solver.sh',ROOT/'scripts/eval.py']
    identity=dict(initial_sha256=INITIAL_SHA,source_sha256={str(p.relative_to(ROOT)):sha(p) for p in paths},
                  configurations={m:configuration(m) for m in ('mc_ppo','group')},
                  reverse_configurations={m:{v:configuration(m,True,v=='mixed') for v in ('control','mixed')}
                                          for m in ('mc_ppo','group')},
                  teacher_sha256=sha(BC_RUN/'data/dataset.json'),reverse_sha256=sha(REVERSE_SOURCE/'teachers/dataset.json'))
    if (pipe/'config.json').exists():assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity)
        for path in paths+[ROOT/'notes/experiments/v099.md',ROOT/'notes/experiments/v100.md']:
            destination=pipe/'frozen'/path.relative_to(ROOT);destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,destination)
    started=time.monotonic()
    def stage(name,script,args,marker):
        if marker.exists():return
        for path,fingerprint in identity['source_sha256'].items():assert sha(ROOT/path)==fingerprint,f'code changed: {path}'
        status(pipe,name,pid=os.getpid(),marker=str(marker))
        with (pipe/f'{name}.log').open('a') as stream:
            process=subprocess.run([sys.executable,ROOT/'adhoc/scripts'/script]+[str(a) for a in args],cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(pipe/f'{name}_exit.json',dict(exit_code=process.returncode,finished_at=now()))
        assert process.returncode==0 and marker.exists(),f'{name} failed; inspect log before resuming'
    def prepare(where,initial,config):
        directory=where/'initial';directory.mkdir(parents=True,exist_ok=True);checkpoint=directory/'checkpoint.pt'
        if not checkpoint.exists():shutil.copy2(initial,checkpoint)
        assert sha(checkpoint)==sha(initial)
        path=where/'run_config.json'
        if path.exists():assert load(path)==config
        else:save(path,config)
    def train_assess(where,source,config):
        stage(source.stem+'_train','train_v099_complete.py',
              ['--run',where,'--config',where/'run_config.json','--gpu-state',root/'gpu_state.json','--teachers',reverse_root],where/'training/result.json')
        assess(where,source)
    def assess(where,source):
        stage(source.stem+'_numerical','assess_v099_complete.py',['--run',where,'--source',source,'--stage','numerical'],where/'numerical/result.json')
        stage(source.stem+'_validation','assess_v099_complete.py',['--run',where,'--source',source,'--stage','validation'],where/'evaluation/validation/greedy/result.json')
    baseline=root/'baseline';baseline_source=ROOT/'adhoc/bin/v099_baseline.cpp'
    prepare(baseline,INITIAL,configuration('group'))
    if not (baseline/'training/model.json').exists():
        model,_=start_model(INITIAL,'cpu',configuration('group'));export(baseline,model,203,3325952,configuration('group'));del model
    # 2方式の学習途中では検証結果に応じた変更を行わない。
    sources={m:ROOT/'adhoc/bin'/f'v099_{m}.cpp' for m in ('mc_ppo','group')}
    for method in ('mc_ppo','group'):
        where=root/method;config=configuration(method);prepare(where,INITIAL,config);train_assess(where,sources[method],config)
    assess(baseline,baseline_source)
    base=evaluation(baseline);reports={m:evaluation(root/m) for m in sources}
    improved=[m for m in sources if improvement(reports[m],base) and acceptable(reports[m])]
    ranking=sorted(sources,key=lambda m:ordering(reports[m],0 if m=='group' else 1))
    best=min(improved,key=lambda m:ordering(reports[m],0 if m=='group' else 1)) if improved else None
    method=best or ranking[0];origin=root/best if best else baseline
    origin_source=sources[best] if best else baseline_source
    choice=dict(method=method,root=str(origin),source=str(origin_source),
                checkpoint_sha256=sha(origin/'training/latest.pt') if best else INITIAL_SHA,
                model_sha256=sha(origin/'training/model.json'),improved=bool(best),ranking=ranking,
                comparisons={m:compare(reports[m],base) for m in sources},fixed_at=now())
    choice_path=reverse_root/'initial_choice.json'
    if choice_path.exists():
        previous=load(choice_path)
        for key in ('method','root','source','checkpoint_sha256','model_sha256','improved','ranking'):assert previous[key]==choice[key]
        choice=previous
    else:save(choice_path,choice)
    save(root/'comparison.json',choice)
    initial=origin/'training/latest.pt' if best else INITIAL
    stage('v100_reselect','prepare_v100_selected.py',['--run',reverse_root,'--reference-source',origin_source,
          '--reference-model',origin/'training/model.json'],reverse_root/'teachers/selection.json')
    selection=load(reverse_root/'teachers/selection.json')
    reverse_sources={v:ROOT/'adhoc/bin'/f'v100_{v}.cpp' for v in ('control','mixed')}
    candidates=[(baseline,baseline_source,base)]
    candidates += [(root/m,sources[m],reports[m]) for m in improved]
    if selection['training_eligible']:
        for variant in ('control','mixed'):
            prepare(reverse_root/variant,initial,configuration(method,True,variant=='mixed'))
        stage('v100_mechanism','check_v100_auxiliary.py',['--run',reverse_root,'--gpu-state',root/'gpu_state.json'],reverse_root/'mechanism/result.json')
        for variant in ('control','mixed'):
            train_assess(reverse_root/variant,reverse_sources[variant],configuration(method,True,variant=='mixed'))
        control=evaluation(reverse_root/'control');mixed=evaluation(reverse_root/'mixed')
        approved=improvement(mixed,control)
        save(reverse_root/'comparison.json',dict(comparison=compare(mixed,control),mixed_improved=approved,completed_at=now()))
        candidates.append((reverse_root/'control',reverse_sources['control'],control))
        if approved:candidates.append((reverse_root/'mixed',reverse_sources['mixed'],mixed))
    else:
        save(reverse_root/'comparison.json',dict(skipped_training=True,reason='preregistered teacher-quality gate not met',completed_at=now()))
    eligible=[(i,c) for i,c in enumerate(candidates) if acceptable(c[2])]
    final=root/'final';final.mkdir(exist_ok=True)
    if not eligible:
        save(final/'result.json',dict(accepted=False,reason='no all-home candidate without deadline cases',completed_at=now()))
    else:
        _,(winner,source,validation)=min(eligible,key=lambda entry:ordering(entry[1][2],entry[0]))
        selected=dict(root=str(winner),source=str(source),source_sha256=sha(source),model_sha256=sha(winner/'training/model.json'),
                      validation=validation['metrics'],fixed_at=now())
        fixed=final/'selection.json'
        if fixed.exists():
            previous=load(fixed)
            for key in ('root','source','source_sha256','model_sha256'):assert previous[key]==selected[key]
            selected=previous
        else:save(fixed,selected)
        for role in ('test','final_in'):
            stage('final_'+role,'assess_v099_complete.py',['--run',winner,'--source',source,'--stage',role],winner/f'evaluation/{role}/greedy/result.json')
        test=load(winner/'evaluation/test/greedy/result.json');final_in=load(winner/'evaluation/final_in/greedy/result.json')
        accepted=(test['metrics']['complete']==256 and final_in['metrics']['complete']==100 and
                  test['metrics']['deadline_cases']==0 and final_in['metrics']['deadline_cases']==0)
        version='v100' if winner.parent==reverse_root else 'v099'
        submission=ROOT/'src/bin'/f'{version}_nn_complete.cpp'
        content=source.read_text();content='// '+submission.name+'\n'+content.split('\n',1)[1]
        if submission.exists():assert submission.read_text()==content
        else:submission.write_text(content)
        save(final/'result.json',dict(selection=selected,accepted=accepted,test=test['metrics'],final_in=final_in['metrics'],
              submission=str(submission),submission_sha256=sha(submission),submitted_to_atcoder=False,completed_at=now()))
    save(reverse_root/'pipeline/exit.json',dict(exit_code=0,finished_at=now()))
    save(pipe/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,finished_at=now()))
    status(pipe,'completed',final_result=str(final/'result.json'),next_stage='verify records, update notes/backlog, commit and push; pause heartbeat')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);p.add_argument('--reverse-run',type=Path,default=REVERSE_RUN);a=p.parse_args()
    try:execute(a.run.resolve(),a.reverse_run.resolve())
    except BlockingIOError:print('pipeline already running',file=sys.stderr);sys.exit(76)
    except BaseException as error:
        save(a.run/'pipeline/exit.json',dict(exit_code=1,error=repr(error),finished_at=now()));traceback.print_exc();sys.exit(1)
