#!/usr/bin/env python3
"""短縮教師の品質・少数列の学習・本比較・固定後の最終評価を順に実行する。"""
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
from v103_data import INITIAL,INITIAL_SHA,PARENT
from assess_v099_complete import compare


def execute(root):
    pipe=root/'pipeline';pipe.mkdir(parents=True,exist_ok=True)
    lock=(pipe/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (pipe/'exit.json').exists() and load(pipe/'exit.json')['exit_code']==0:return
    assert sha(INITIAL)==INITIAL_SHA and load(root/'mechanism/result.json')['passed']
    files=['v103_data.py','train_v103_imitation.py','check_v103_imitation.py','assess_v103_imitation.py','run_v103_pipeline.py',
           'v101_compute.py','v091_env.py','v092_stream.py','v089_data.py','v090_data.py','train_v090_board.py',
           'train_v089_board.py','train_v077_rank.py','train_v080_scaling.py','assess_v099_complete.py','build_v090_board.py',
           'check_v089_board.py','run_v089_evaluation.py','v089_core.cpp.txt','v090_inference.cpp.txt','v090_search.cpp.txt','memory_guard.py']
    paths=[ROOT/'adhoc/scripts'/name for name in files]+[ROOT/'adhoc/bin'/name for name in ('v103_shorten.cpp','v091_environment.cpp','v102_without_bc.cpp')]
    paths += [ROOT/'scripts/eval.py',ROOT/'scripts/build_solver.sh']
    identity=dict(initial_sha256=INITIAL_SHA,sources={str(p.relative_to(ROOT)):sha(p) for p in paths})
    if (pipe/'config.json').exists():assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity)
        for path in paths+[ROOT/'notes/experiments/v103.md']:
            dest=pipe/'frozen'/path.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,dest)
        (root/'initial').mkdir(exist_ok=True);shutil.copy2(INITIAL,root/'initial/checkpoint.pt')
    def stage(name,script,args,marker):
        if marker.exists():return
        for path,fingerprint in identity['sources'].items():assert sha(ROOT/path)==fingerprint,f'changed source: {path}'
        status(pipe,name,pid=os.getpid(),marker=str(marker))
        with (pipe/(name+'.log')).open('a') as stream:
            proc=subprocess.run([sys.executable,ROOT/'adhoc/scripts'/script]+list(map(str,args)),cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(pipe/(name+'_exit.json'),dict(exit_code=proc.returncode,finished_at=now()))
        assert proc.returncode==0 and marker.exists(),f'{name} failed; inspect saved log'
    started=time.monotonic()
    try:
        reason='teacher_quality_gate';quality=load(root/'diagnostic/data/result.json')['quality_gate']
        diagnostic=False;main=False;supported=False;eligible=[];assessments={}
        if quality:
            stage('diagnostic_train','train_v103_imitation.py',['--run',root/'diagnostic','--data',root/'diagnostic/data/shortened',
                  '--diagnostic','--gpu-state',root/'gpu_state.json'],root/'diagnostic/training/result.json')
            stage('diagnostic_assess','assess_v103_imitation.py',['--run',root,'--variant','diagnostic'],root/'diagnostic/assessment.json')
            diagnostic=load(root/'diagnostic/assessment.json')['passed'];reason='diagnostic_learning_gate'
        if diagnostic:
            stage('main_teachers','v103_data.py',['--run',root,'--phase','main'],root/'main/data/result.json')
            main=load(root/'main/data/result.json')['quality_gate'];reason='main_teacher_gate'
        if main:
            # 両条件を学習し終えてから検証結果を見る。条件間の変更を追加しない。
            for variant in ('original','shortened'):
                stage(variant+'_train','train_v103_imitation.py',['--run',root/'main'/variant,'--data',root/'main/data'/variant,
                      '--gpu-state',root/'gpu_state.json'],root/'main'/variant/'training/result.json')
            for variant in ('original','shortened'):
                stage(variant+'_assess','assess_v103_imitation.py',['--run',root,'--variant',variant],root/'main'/variant/'assessment.json')
                assessments[variant]=load(root/'main'/variant/'assessment.json')
            budgets=all(load(root/'main'/n/'training/result.json')['complete_budget'] for n in assessments)
            for variant,a in assessments.items():
                m=a['greedy']['metrics'];c=a['comparison']
                if budgets and m['complete']==256 and m['deadline_cases']==0 and c['mean_S_difference_all']<0 and c['mean_T_difference_on_common']<0:
                    eligible.append(variant)
            pair=compare(assessments['shortened']['greedy'],assessments['original']['greedy'])
            supported='shortened' in eligible and pair['mean_S_difference_all']<0 and pair['mean_T_difference_on_common']<0
            save(root/'comparison.json',dict(against_initial={n:a['comparison'] for n,a in assessments.items()},shortened_vs_original=pair,
                 complete_budget=budgets,shortened_teacher_supported=supported))
            reason='comparison_completed'
        candidate=min(eligible,key=lambda n:(assessments[n]['greedy']['metrics']['mean_S_all'],assessments[n]['greedy']['metrics']['mean_elapsed_ms'],('original','shortened').index(n))) if eligible else 'v102'
        chosen=PARENT if candidate=='v102' else root/'main'/candidate
        old_source=ROOT/'adhoc/bin'/('v102_without_bc.cpp' if candidate=='v102' else 'v103_'+candidate+'.cpp')
        source=ROOT/'src/bin'/('v102_nn_group.cpp' if candidate=='v102' else 'v103_nn_self_imitation.cpp')
        content='// '+source.name+'\n'+old_source.read_text().split('\n',1)[1]
        if source.exists():assert source.read_text()==content
        else:source.write_text(content)
        final=root/'final';(final/'training').mkdir(parents=True,exist_ok=True)
        model_path=chosen/'training/model.json';model_target=final/'training/model.json'
        if model_target.exists():assert sha(model_target)==sha(model_path)
        else:shutil.copy2(model_path,model_target)
        frozen=dict(candidate=candidate,source=str(source.relative_to(ROOT)),solver_sha256=sha(source),model_sha256=sha(model_path),
                    checkpoint_sha256=sha(chosen/'training/latest.pt'),checkpoint=str((chosen/'training/latest.pt').relative_to(ROOT)),
                    quality_gate=quality,diagnostic_gate=diagnostic,main_gate=main,shortened_teacher_supported=supported)
        if (final/'candidate.json').exists():assert load(final/'candidate.json')==frozen
        else:save(final/'candidate.json',frozen)
        # 改名は先頭コメントだけ。保存済みの同じ重み・本体の数値照合を再利用する。
        assert content.split('\n',1)[1]==old_source.read_text().split('\n',1)[1]
        prior=load(chosen/'numerical/result.json');assert prior['passed'] and prior['model_sha256']==sha(model_target)
        save(final/'numerical/result.json',dict(prior,solver_sha256=sha(source),reused_from=str(chosen/'numerical/result.json')))
        for role in ('test','final_in'):
            stage('final_'+role,'assess_v099_complete.py',['--run',final,'--source',source,'--stage',role],final/'evaluation'/role/'greedy/result.json')
        save(root/'result.json',dict(reason=reason,final=frozen,test=load(final/'evaluation/test/greedy/result.json'),
             tools_in=load(final/'evaluation/final_in/greedy/result.json'),completed_at=now()))
        save(pipe/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,finished_at=now()))
        status(pipe,'completed',candidate=candidate,next_stage='verify, record, commit/push, stop heartbeat')
    except BaseException as error:
        save(pipe/'exit.json',dict(exit_code=1,error=repr(error),traceback=traceback.format_exc(),finished_at=now()))
        status(pipe,'failed',error=repr(error));raise


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=Path);a=p.parse_args();execute(a.run.resolve())
