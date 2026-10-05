#!/usr/bin/env python3
"""学習した再挿入順を固定検証で比較し、採用時だけ最終集合を測る。"""
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
from build_v123_order import ROOT, RUN
CONDITIONS={"learned":1}
from check_v123_order import check
from check_v113_integrated import normalize
from check_v089_board import trace, compile_binary
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output, statistics
from v089_data import save, sha, now, status
from v090_data import RUN as BC_RUN
from v111_portfolio import comparison, valid
CONTROL=ROOT/'results/nn_rank/v115/20261004_corridor_studio'
def load(path):return json.loads(path.read_text())

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
    name=f'v123_{label}_{role}';wrapper=ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag=f'v123_order_{label}_{role}_studio_j12';scratch=ROOT/'results/out'/name
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


def finish(selected,assessment):
    directory=RUN/'final';directory.mkdir(exist_ok=True)
    if (directory/'result.json').exists():return load(directory/'result.json')
    old=load(CONTROL/'final/result.json')
    if selected=='base':
        result=dict(accepted=False,retained='v115',candidate=old['candidate'],test=old['test'],
                    tools_in=old['tools_in'],reused_control=True,completed_at=now())
        save(directory/'result.json',result);return result
    source=ROOT/'src/bin/v123_learned_order.cpp'
    original=ROOT/assessment['sources'][selected]['path']
    content='// '+source.name+'\n'+original.read_text().split('\n',1)[1]
    if source.exists():assert source.read_text()==content
    else:source.write_text(content)
    candidate=dict(condition=selected,source=str(source.relative_to(ROOT)),source_sha256=sha(source),
                   model_parent='v111',order_model_sha256=sha(RUN/'training/model.json'),
                   fixed_at=assessment['fixed_at'])
    save(directory/'candidate.json',candidate)
    for local in (True,False):
        mode='local' if local else 'judge'
        compile_binary(source.stem,directory/('solver_'+mode),local)
        before=normalize((RUN/'mechanism'/(mode+'.ii')).read_text())
        after=normalize(preprocess(source,directory/(mode+'.ii'),local))
        assert before==after,(mode,'selected standalone changed')
    test=evaluate(RUN,'final',source,'test');official=evaluate(RUN,'final',source,'final_in')
    result=dict(accepted=True,candidate=candidate,test=test,tools_in=official,
                final_valid=valid(test) and valid(official),
                differences=dict(test=comparison(test,old['test']),tools_in=comparison(official,old['tools_in'])),
                goal_190=valid(official) and official['metrics']['mean_T_completed']<190,
                completed_at=now())
    save(directory/'result.json',result);return result


def run():
    lock=(RUN/'pipeline.lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (RUN/'pipeline/exit.json').exists() and load(RUN/'pipeline/exit.json')['exit_code']==0:return
    began=time.monotonic()
    try:
        manifest=load(RUN/'sources.json')
        identity=dict(conditions=CONDITIONS,sources=manifest['sources'],jobs=12,local_ratio=.80,
                      baseline_sha256=sha(CONTROL/'r0/evaluation/validation/result.json'),
                      scripts={name:sha(ROOT/'adhoc/scripts'/name) for name in
                               ['build_v123_order.py','export_v123_order.py','check_v123_order.py','run_v123_order.py']})
        if (RUN/'evaluation_config.json').exists():assert load(RUN/'evaluation_config.json')==identity
        else:
            save(RUN/'evaluation_config.json',identity);(RUN/'evaluation_frozen').mkdir(exist_ok=True)
            for name in identity['scripts']:shutil.copy2(ROOT/'adhoc/scripts'/name,RUN/'evaluation_frozen'/name)
        mechanism=check();assert mechanism['source_sha256']==manifest['sources']['learned']['sha256'];baseline=load(CONTROL/'r0/evaluation/validation/result.json');assert valid(baseline)
        labels=[k for k in CONDITIONS if mechanism['gates'][k]['passed']]
        if not labels:
            save(RUN/'result.json',dict(mechanism=mechanism,assessment='mechanism_scope_gate_failed',final=finish('base',{}),completed_at=now()))
            status(RUN/'pipeline','completed',selected='base',gate=False)
        else:
            results={'base':baseline};differences={};activation={}
            for label in labels:
                source=ROOT/manifest['sources'][label]['path'];assert sha(source)==manifest['sources'][label]['sha256']
                result=evaluate(RUN,label,source);results[label]=result;differences[label]=comparison(result,baseline)
                activation[label]={k:sum(r['trace'].get(k,0) for r in result['rows'])
                                   for k in result['rows'][0]['trace'] if k.startswith('order_nn_')}
                assert activation[label]['order_nn_calls']>0,(label,'not activated')
            eligible=[k for k in labels if valid(results[k]) and differences[k]['mean_T_difference']<0
                      and differences[k]['mean_S_difference']<0 and all(r['trace'].get('race_seed_count',99)<=4 for r in results[k]['rows'])]
            selected=min(eligible,key=lambda k:(results[k]['metrics']['mean_S_all'],-CONDITIONS[k])) if eligible else 'base'
            assessment=dict(selected=selected,eligible=eligible,sources=manifest['sources'],validation=results,
                            differences=differences,activation=activation,mechanism=mechanism,fixed_at=now())
            save(RUN/'assessment.json',assessment)
            save(RUN/'result.json',dict(assessment=assessment,final=finish(selected,assessment),completed_at=now()))
            status(RUN/'pipeline','completed',selected=selected)
        save(RUN/'pipeline/exit.json',dict(exit_code=0,seconds=time.monotonic()-began,completed_at=now()))
    except BaseException as error:
        status(RUN/'pipeline','failed',error=repr(error))
        save(RUN/'pipeline/exit.json',dict(exit_code=1,error=repr(error),traceback=traceback.format_exc(),completed_at=now()))
        raise

if __name__=='__main__':run()
