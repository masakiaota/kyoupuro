#!/usr/bin/env python3
"""固定した7統合条件を評価し、事前規則で1本を選び最終比較を保存する。"""
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

from build_v113_integrated import ROOT, RUN, CONDITIONS
from check_v089_board import trace, compile_binary
from check_v113_integrated import check, normalize
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output, statistics
from v089_data import save, sha, now, status
from v090_data import RUN as BC_RUN
from v111_portfolio import comparison, valid

CONTROL = ROOT/'results/nn_rank/v111/20261004_weight_portfolio_studio'


def load(path):
    return json.loads(path.read_text())


def evaluate(root,label,source,role='validation'):
    where=root/label/'evaluation'/role;where.mkdir(parents=True,exist_ok=True)
    marker=where/'result.json'
    if marker.exists():
        result=load(marker);assert result['source_sha256']==sha(source);return result
    if role=='final_in':
        cases=[dict(index=i,filename=p.name,path=str(p),sha256=sha(p))
               for i,p in enumerate(sorted((ROOT/'tools/in').glob('*.txt')))]
    else:
        cases=[dict(c,filename=f"{c['index']:06d}.txt") for c in load(BC_RUN/'input_manifest.json') if c['role']==role]
    assert len(cases)==(100 if role=='final_in' else 256)
    inputs=where/'inputs';inputs.mkdir(exist_ok=True)
    for case in cases:
        original=Path(case['path']) if role=='final_in' else BC_RUN/case['path']
        assert sha(original)==case['sha256'];shutil.copy2(original,inputs/case['filename'])
    save(where/'input_manifest.json',cases)
    name=f'v113_{label}_{role}';wrapper=ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag=f'v113_integration_{label}_{role}_studio_j12';scratch=ROOT/'results/out'/name
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


def activation(label,result):
    keys={'h':['v318_history_builds','v318_history_queries','v318_feature_reuses'],
          'r':['v315_slices','v315_challenges'],
          's':['v314_calls','v314_conditioned_completed','v314_accepted'],
          'p':[]}
    totals={key:sum(r['construction'].get(key,0) for r in result['rows'])
            for feature in label for key in keys[feature]}
    if 'p' in label:
        actor={k:sum(r['trace'].get(k,0) for r in result['rows'])
               for k in result['rows'][0]['trace'] if 'actor' in k}
        assert actor and any('hits' in k and v>0 for k,v in actor.items()),actor
        totals.update(actor)
    assert all(v>0 for k,v in totals.items() if 'actor' not in k),(label,totals)
    return totals


def finish(root,selected,assessment):
    directory=root/'final';directory.mkdir(exist_ok=True)
    if (directory/'result.json').exists():return load(directory/'result.json')
    if selected=='base':
        old=load(CONTROL/'final/result.json')
        result=dict(accepted=False,retained='v111',candidate=old['candidate'],test=old['test'],tools_in=old['tools_in'],
                    reused_control=True,completed_at=now())
        save(directory/'result.json',result);return result
    source=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
    original=ROOT/assessment['sources'][selected]['path']
    content='// '+source.name+'\n'+original.read_text().split('\n',1)[1]
    if source.exists():assert source.read_text()==content
    else:source.write_text(content)
    candidate=dict(condition=selected,source=str(source.relative_to(ROOT)),source_sha256=sha(source),
                   model_parent='v111',fixed_at=assessment['fixed_at'])
    if (directory/'candidate.json').exists():assert load(directory/'candidate.json')==candidate
    else:save(directory/'candidate.json',candidate)
    for local in (True,False):
        mode='local' if local else 'judge'
        compile_binary(source.stem,directory/('solver_'+mode),local)
        before=normalize((root/'mechanism'/selected/(mode+'.ii')).read_text())
        after=normalize(preprocess(source,directory/(mode+'.ii'),local))
        assert before==after,(mode,'selected standalone code changed')
    test=evaluate(root,'final',source,'test');official=evaluate(root,'final',source,'final_in')
    old=load(CONTROL/'final/result.json')
    result=dict(accepted=True,candidate=candidate,test=test,tools_in=official,
                final_valid=valid(test) and valid(official),
                differences=dict(test=comparison(test,old['test']),tools_in=comparison(official,old['tools_in'])),
                completed_at=now())
    save(directory/'result.json',result);return result


def run(root=RUN):
    root.mkdir(parents=True,exist_ok=True)
    lock=(root/'pipeline.lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (root/'pipeline/exit.json').exists() and load(root/'pipeline/exit.json')['exit_code']==0:return
    begin=time.monotonic()
    try:
        manifest=load(root/'sources.json')
        identity=dict(conditions=CONDITIONS,sources=manifest['sources'],jobs=12,local_ratio=.80,
                      baseline_sha256=sha(CONTROL/'learned/evaluation/validation/result.json'),
                      scripts={name:sha(ROOT/'adhoc/scripts'/name) for name in
                               ['build_v113_integrated.py','check_v113_integrated.py','run_v113_integrated.py']},
                      checker_sha256=sha(ROOT/'adhoc/bin/check_v113_components.cpp'))
        if (root/'config.json').exists():assert load(root/'config.json')==identity
        else:
            save(root/'config.json',identity)
            for name in identity['scripts']:
                shutil.copy2(ROOT/'adhoc/scripts'/name,root/'frozen'/name)
            shutil.copy2(ROOT/'adhoc/bin/check_v113_components.cpp',root/'frozen/check_v113_components.cpp')
        mechanism=check(root)
        baseline=load(CONTROL/'learned/evaluation/validation/result.json');assert valid(baseline)
        results={'base':baseline};differences={};activated={}
        for label in CONDITIONS:
            source=ROOT/manifest['sources'][label]['path'];assert sha(source)==manifest['sources'][label]['sha256']
            result=evaluate(root,label,source);results[label]=result
            activated[label]=activation(label,result);differences[label]=comparison(result,baseline)
        eligible=[k for k in CONDITIONS if valid(results[k]) and differences[k]['mean_T_difference']<0 and differences[k]['mean_S_difference']<0]
        selected=min(eligible,key=lambda k:(results[k]['metrics']['mean_S_all'],results[k]['metrics']['mean_T_completed'],len(k),k)) if eligible else 'base'
        assessment=dict(selected=selected,accepted=bool(eligible),eligible=eligible,sources=manifest['sources'],
                        validation=results,differences=differences,activation=activated,mechanism=mechanism,fixed_at=now())
        save(root/'assessment.json',assessment)
        result=dict(assessment=assessment,final=finish(root,selected,assessment),completed_at=now())
        save(root/'result.json',result)
        status(root/'pipeline','completed',selected=selected)
        save(root/'pipeline/exit.json',dict(exit_code=0,seconds=time.monotonic()-begin,completed_at=now()))
    except BaseException as error:
        status(root/'pipeline','failed',error=repr(error))
        save(root/'pipeline/exit.json',dict(exit_code=1,error=repr(error),traceback=traceback.format_exc(),completed_at=now()))
        raise


if __name__=='__main__':
    run()
