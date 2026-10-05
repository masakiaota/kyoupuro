#!/usr/bin/env python3
"""診断、交差検証、新規256入力、候補固定後のin確認を順に進める。"""
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import traceback
from build_v124_allocation import ROOT, RUN, BASE, BASE_SHA
from check_v124_allocation import check
from check_v113_integrated import normalize
from check_v089_board import trace, compile_binary
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output, statistics
from v089_data import save, sha, now, status
from v111_portfolio import comparison, valid


def load(path):return json.loads(path.read_text())


def evaluate(root,label,source,role='validation',cases=None):
    where=root/label/'evaluation'/role;where.mkdir(parents=True,exist_ok=True)
    marker=where/'result.json'
    if marker.exists():
        result=load(marker);assert result['source_sha256']==sha(source);return result
    if role=='final_in':
        cases=[dict(index=i,filename=p.name,path=str(p),sha256=sha(p))
               for i,p in enumerate(sorted((ROOT/'tools/in').glob('*.txt')))]
    else:
        assert cases is not None
        cases=[dict(c,filename=f"{c['index']:04d}.txt") for c in cases]
    assert len(cases)>0
    if role=='final_in':assert len(cases)==100
    inputs=where/'inputs';inputs.mkdir(exist_ok=True)
    for case in cases:
        original=Path(case['path']) if role=='final_in' else RUN/case['path']
        assert sha(original)==case['sha256'];shutil.copy2(original,inputs/case['filename'])
    save(where/'input_manifest.json',cases)
    name=f'v124_{label}_{role}';wrapper=ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag=f'v124_allocation_{label}_{role}_studio_j12';scratch=ROOT/'results/out'/name
    identity=dict(source_sha256=sha(source),label=tag,jobs=12,manifest_sha256=sha(where/'input_manifest.json'))
    started=where/'started.json'
    if not started.exists():
        assert not scratch.exists(),scratch
        save(started,identity);status(root/'pipeline','evaluating',condition=label,role=role,pid=os.getpid())
        with (where/'eval.log').open('w') as stream:
            proc=subprocess.run([sys.executable,ROOT/'scripts/eval.py',name,inputs,'-j','12','--wait-lock','--label',tag],
                                cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(where/'exit.json',dict(exit_code=proc.returncode,finished_at=now()))
    else:
        assert load(started)==identity
    assert load(where/'exit.json')['exit_code']==0,'inspect partial evaluation before resuming'
    records=[]
    with (ROOT/'results/eval_records.jsonl').open() as stream:
        for line in stream:
            row=json.loads(line)
            if row['label']==tag:records.append(row)
    assert len(records)==len({r['case_name'] for r in records})==len(cases)
    assert len({r['run_id'] for r in records})==1
    save(where/'official_records.json',records)
    outputs=where/'outputs'
    if not outputs.exists():shutil.copytree(scratch,outputs)
    indexed={r['case_name']:r for r in records};rows=[]
    for case in cases:
        name=case['filename'];record=indexed[name]
        assert record['status']=='ok' and record['local']
        result=validated_output(inputs/name,(outputs/name).read_text());assert result['S']==record['score']
        stderr=(outputs/(name+'.err')).read_text();counts=trace(stderr)
        custom={}
        for key,value in re.findall(r'\b(v31\d_\w+|pre_lns|lns_attempts)=(-?\d+(?:\.\d*)?(?:e[+-]?\d+)?)',stderr):
            custom[key]=float(value) if any(c in value for c in '.e') else int(value)
        for key,diagnostic in (('inferences','v312_inferences'),('depth','v312_nn_steps'),('deadline','v312_nn_deadline')):
            counts.setdefault(key,int(custom.get(diagnostic,0)))
        rows.append(dict(case=case['index'],filename=name,elapsed_ms=record['elapsed'],trace=counts,construction=custom,**result))
    result=dict(metrics=statistics(rows),rows=rows,source_sha256=sha(source),all_legal=True,completed_at=now())
    save(marker,result);return result


def train(full=False):
    script=ROOT/'adhoc/scripts/train_v124_allocation.py'
    marker=RUN/'training/full/model.json' if full else RUN/'training/result.json'
    if marker.exists():return
    status(RUN/'pipeline','training_full' if full else 'training_folds',pid=os.getpid())
    subprocess.run([sys.executable,script]+(['--full'] if full else []),cwd=ROOT,check=True)


def combine(results):
    rows=sorted([row for result in results for row in result['rows']],key=lambda r:r['case'])
    assert len(rows)==len({r['case'] for r in rows})==128
    return dict(metrics=statistics(rows),rows=rows,all_legal=True)


def finish(source,assessment):
    where=RUN/'final';where.mkdir(exist_ok=True)
    if (where/'result.json').exists():return load(where/'result.json')
    target=ROOT/'src/bin/v124_learned_allocation.cpp'
    content='// '+target.name+'\n'+source.read_text().split('\n',1)[1]
    if target.exists():assert target.read_text()==content
    else:target.write_text(content)
    candidate=dict(source=str(target.relative_to(ROOT)),source_sha256=sha(target),parent_sha256=BASE_SHA,
                   model_sha256=sha(RUN/'training/full/model.json'),fixed_at=now(),selection='new_validation_256')
    save(where/'candidate.json',candidate)
    for local in (True,False):
        mode='local' if local else 'judge';compile_binary(target.stem,where/('solver_'+mode),local)
        before=normalize((RUN/'checks/full'/(mode+'.ii')).read_text())
        actual=normalize(preprocess(target,where/(mode+'.ii'),local));assert actual==before
    result=evaluate(RUN,'final',target,'final_in')
    reference=load(RUN/'in_reference.json');values=reference['values']
    relative=100*sum(values[r['filename']]/r['T'] for r in result['rows'])/100
    final=dict(candidate=candidate,tools_in=result,final_valid=valid(result),fixed_reference_relative=relative,
               reference_sha256=sha(RUN/'in_reference.json'),goal_190=valid(result) and result['metrics']['mean_T_completed']<190,completed_at=now())
    save(where/'result.json',final);return final


def run():
    lock=(RUN/'pipeline.lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (RUN/'pipeline/exit.json').exists() and load(RUN/'pipeline/exit.json')['exit_code']==0:return
    began=time.monotonic();save(RUN/'launch_pipeline.json',dict(pid=os.getpid(),started_at=now()))
    try:
        assert sha(BASE)==BASE_SHA
        files=['run_v124_pipeline.py','train_v124_allocation.py','check_v124_allocation.py','export_v124_allocation.py','v124_features.cpp.txt']
        identity=dict(scripts={n:sha(ROOT/'adhoc/scripts'/n) for n in files},base_sha256=BASE_SHA,jobs=12,
                      diagnostic_mean_improvement=.25,cv_folds=4,epochs=500)
        if (RUN/'pipeline_config.json').exists():assert load(RUN/'pipeline_config.json')==identity
        else:
            save(RUN/'pipeline_config.json',identity);frozen=RUN/'pipeline_frozen';frozen.mkdir(exist_ok=True)
            for n in files:shutil.copy2(ROOT/'adhoc/scripts'/n,frozen/n)
        status(RUN/'pipeline','waiting_collection',pid=os.getpid())
        while not (RUN/'collection/exit.json').exists():
            os.kill(load(RUN/'dispatch.json')['pid'],0)
            if time.monotonic()-began>3600:raise RuntimeError('collection exceeded one hour')
            time.sleep(10)
        assert load(RUN/'collection/exit.json')['exit_code']==0
        train();diagnostic=load(RUN/'training/result.json')
        if not diagnostic['passed']:
            save(RUN/'result.json',dict(decision='diagnostic_failed',retained='v113',diagnostic=diagnostic,completed_at=now()))
            status(RUN/'pipeline','completed',decision='diagnostic_failed')
        else:
            manifest=load(RUN/'input_manifest.json');training=[c for c in manifest if c['role']=='train']
            folds=load(RUN/'training/folds.json');fold_results=[];models={}
            for i in range(4):
                name=f'fold_{i}';models[name]=check(name)
                source=ROOT/models[name]['source'];subset=[c for c in training if folds[c['index']]==i]
                assert len(subset)==32
                fold_results.append(evaluate(RUN,name,source,'cross_validation',subset))
            learned=combine(fold_results);baseline=evaluate(RUN,'base',BASE,'cross_validation',training)
            difference=comparison(learned,baseline)
            gate=valid(learned) and valid(baseline) and difference['mean_T_difference']<0 and difference['mean_S_difference']<0
            active=sum(r['trace'].get('allocation_selected_longer',0) for r in learned['rows'])
            assert active>0,'learned allocation never selected a longer plan'
            cv=dict(passed=gate,learned=learned,base=baseline,difference=difference,models=models,active_longer_selections=active,completed_at=now())
            save(RUN/'cross_validation/result.json',cv)
            if not gate:
                save(RUN/'result.json',dict(decision='cross_validation_failed',retained='v113',diagnostic=diagnostic,cross_validation=cv,completed_at=now()))
                status(RUN/'pipeline','completed',decision='cross_validation_failed')
            else:
                train(full=True);model=check('full');source=ROOT/model['source']
                validation=[c for c in manifest if c['role']=='validation'];assert len(validation)==256
                # 別の新規入力はモデル固定後にだけ実行する。
                baseline=evaluate(RUN,'base',BASE,'validation',validation)
                learned=evaluate(RUN,'learned',source,'validation',validation);difference=comparison(learned,baseline)
                accepted=valid(learned) and valid(baseline) and difference['mean_T_difference']<0 and difference['mean_S_difference']<0
                assessment=dict(accepted=accepted,base=baseline,learned=learned,difference=difference,model=model,fixed_at=now())
                save(RUN/'assessment.json',assessment)
                result=dict(decision='accepted' if accepted else 'validation_failed',diagnostic=diagnostic,cross_validation=cv,assessment=assessment,completed_at=now())
                if accepted:result['final']=finish(source,assessment)
                else:result['retained']='v113'
                save(RUN/'result.json',result);status(RUN/'pipeline','completed',decision=result['decision'])
        save(RUN/'pipeline/exit.json',dict(exit_code=0,seconds=time.monotonic()-began,completed_at=now()))
    except BaseException as e:
        status(RUN/'pipeline','failed',error=repr(e));save(RUN/'pipeline/exit.json',dict(exit_code=1,error=repr(e),traceback=traceback.format_exc(),completed_at=now()));raise


if __name__=='__main__':run()
