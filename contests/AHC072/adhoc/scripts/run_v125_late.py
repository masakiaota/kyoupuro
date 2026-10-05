#!/usr/bin/env python3
"""全域・全期間制限・後半制限を未使用256入力で一度ずつ比較する。"""
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
from build_v125_late import ROOT, RUN, BASE, CORRIDOR
from check_v125_late import check
from check_v113_integrated import normalize
from check_v089_board import trace, compile_binary
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output, statistics
from v089_data import save, sha, now, status
from v111_portfolio import comparison, valid

PREVIOUS=ROOT/'results/nn_rank/v124/20261004_plan_allocation_studio'
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
    name=f'v125_{label}_{role}';wrapper=ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag=f'v125_late_{label}_{role}_studio_j12';scratch=ROOT/'results/out'/name
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


def inputs():
    marker=RUN/'input_manifest.json'
    if marker.exists():return load(marker)
    assert load(PREVIOUS/'result.json')['decision']=='diagnostic_failed'
    assert not (PREVIOUS/'base/evaluation/validation/result.json').exists()
    cases=[c for c in load(PREVIOUS/'input_manifest.json') if c['role']=='validation'];assert len(cases)==256
    directory=RUN/'inputs';directory.mkdir(exist_ok=True)
    for c in cases:
        old=PREVIOUS/c['path'];assert sha(old)==c['sha256'];new=directory/old.name
        shutil.copy2(old,new);c['path']=str(new.relative_to(RUN))
    save(marker,cases);shutil.copy2(PREVIOUS/'in_reference.json',RUN/'in_reference.json');return cases


def finish(source):
    where=RUN/'final';where.mkdir(exist_ok=True)
    if (where/'result.json').exists():return load(where/'result.json')
    target=ROOT/'src/bin/v125_late_corridor.cpp';content='// '+target.name+'\n'+source.read_text().split('\n',1)[1]
    if target.exists():assert target.read_text()==content
    else:target.write_text(content)
    candidate=dict(source=str(target.relative_to(ROOT)),source_sha256=sha(target),fixed_at=now())
    save(where/'candidate.json',candidate)
    for local in (True,False):
        mode='local' if local else 'judge';compile_binary(target.stem,where/('solver_'+mode),local)
        actual=normalize(preprocess(target,where/(mode+'.ii'),local));before=normalize((RUN/'mechanism'/(mode+'.ii')).read_text())
        assert actual==before
    result=evaluate(RUN,'final',target,'final_in')
    ref=load(RUN/'in_reference.json');relative=sum(ref['values'][r['filename']]/r['T'] for r in result['rows'])
    final=dict(candidate=candidate,tools_in=result,final_valid=valid(result),fixed_reference_relative=relative,
               reference_sha256=sha(RUN/'in_reference.json'),goal_190=valid(result) and result['metrics']['mean_T_completed']<190,completed_at=now())
    save(where/'result.json',final);return final


def run():
    lock=(RUN/'pipeline.lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (RUN/'pipeline/exit.json').exists() and load(RUN/'pipeline/exit.json')['exit_code']==0:return
    began=time.monotonic();save(RUN/'launch.json',dict(pid=os.getpid(),started_at=now()))
    try:
        mechanism=check();cases=inputs();source=ROOT/load(RUN/'source.json')['source']
        assert sha(source)==mechanism['source_sha256']
        sources={'base':BASE,'full':CORRIDOR,'late':source}
        scripts=['build_v125_late.py','check_v125_late.py','run_v125_late.py']
        identity=dict(sources={k:sha(v) for k,v in sources.items()},input_manifest_sha256=sha(RUN/'input_manifest.json'),
                      scripts={n:sha(ROOT/'adhoc/scripts'/n) for n in scripts},switch_fraction=.5,jobs=12)
        if (RUN/'config.json').exists():assert load(RUN/'config.json')==identity
        else:
            save(RUN/'config.json',identity);frozen=RUN/'frozen';frozen.mkdir(exist_ok=True)
            for n in scripts:shutil.copy2(ROOT/'adhoc/scripts'/n,frozen/n)
            for p in sources.values():shutil.copy2(p,frozen/p.name)
        results={label:evaluate(RUN,label,path,'validation',cases) for label,path in sources.items()}
        differences={label:comparison(results['late'],results[label]) for label in ['base','full']}
        rows=results['late']['rows'];activation={k:sum(r['trace'].get(k,0) for r in rows) for k in ['corridor_early_full','corridor_late_full','corridor_late_limited']}
        assert all(v>0 for v in activation.values())
        assert all(r['trace'].get('corridor_restricted_groups',0)==r['trace'].get('corridor_late_limited',0) for r in rows)
        accepted=all(valid(v) for v in results.values()) and all(d['mean_T_difference']<0 and d['mean_S_difference']<0 for d in differences.values())
        assessment=dict(accepted=accepted,validation=results,differences=differences,activation=activation,mechanism=mechanism,fixed_at=now())
        save(RUN/'assessment.json',assessment)
        result=dict(assessment=assessment,completed_at=now())
        if accepted:result['final']=finish(source)
        else:result['retained']=dict(version='v113',source=str(BASE.relative_to(ROOT)),sha256=sha(BASE))
        save(RUN/'result.json',result);status(RUN/'pipeline','completed',accepted=accepted)
        save(RUN/'pipeline/exit.json',dict(exit_code=0,seconds=time.monotonic()-began,completed_at=now()))
    except BaseException as e:
        status(RUN/'pipeline','failed',error=repr(e));save(RUN/'pipeline/exit.json',dict(exit_code=1,error=repr(e),traceback=traceback.format_exc(),completed_at=now()));raise


if __name__=='__main__':run()
