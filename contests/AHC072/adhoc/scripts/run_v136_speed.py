#!/usr/bin/env python3
"""計算省略版を固定仕事量で確認し、保存v113との公式比較まで進める。"""
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
from build_v136_speed import ROOT, RUN, BASE, SOURCE, BASE_SHA, build
from check_v136_speed import check
from check_v089_board import trace
from run_v089_evaluation import validated_output, statistics
from v089_data import save, sha, now, status
from v111_portfolio import comparison, valid

PREVIOUS = ROOT / 'results/nn_rank/v130/20261004_rrt_studio'


def load(path):
    return json.loads(path.read_text())


def evaluate(label, source, role, cases=None):
    root = RUN
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
    name=f'v136_{label}_{role}';wrapper=ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag=f'v136_late_{label}_{role}_studio_j12';scratch=ROOT/'results/out'/name
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
    marker = RUN / 'input_manifest.json'
    if marker.exists():
        return load(marker)
    cases = load(PREVIOUS / 'input_manifest.json')
    assert len(cases) == len({c['sha256'] for c in cases}) == 256
    directory = RUN / 'inputs'
    directory.mkdir(exist_ok=True)
    for case in cases:
        original = PREVIOUS / case['path']
        assert sha(original) == case['sha256']
        target = directory / original.name
        shutil.copy2(original, target)
        case['path'] = str(target.relative_to(RUN))
    save(marker, cases)
    for name in ('in_reference.json', 'excluded_hashes.json'):
        shutil.copy2(PREVIOUS / name, RUN / name)
    return cases


def run():
    RUN.mkdir(parents=True, exist_ok=True)
    lock = (RUN / 'pipeline.lock').open('a')
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    if (RUN / 'pipeline/exit.json').exists() and load(RUN / 'pipeline/exit.json')['exit_code'] == 0:
        return
    started = time.monotonic()
    save(RUN / 'launch.json', dict(pid=os.getpid(), started_at=now()))
    try:
        build()
        cases = inputs()
        files = [Path(__file__), ROOT/'adhoc/scripts/build_v136_speed.py', ROOT/'adhoc/scripts/check_v136_speed.py']
        identity = dict(source_sha256=sha(SOURCE), parent_sha256=sha(BASE), jobs=12,
                        input_manifest_sha256=sha(RUN/'input_manifest.json'), scripts={str(p.relative_to(ROOT)):sha(p) for p in files})
        if (RUN/'config.json').exists():assert load(RUN/'config.json')==identity
        else:
            save(RUN/'config.json',identity)
            frozen=RUN/'frozen';frozen.mkdir(exist_ok=True)
            for p in [BASE,SOURCE,ROOT/'notes/experiments/v136.md',*files]:shutil.copy2(p,frozen/p.name)
        mechanism=check()
        if not mechanism['speed_passed']:
            result=dict(accepted=False,reason='fixed_work_speed_criterion',mechanism=mechanism,retained='v113',completed_at=now())
            save(RUN/'result.json',result);status(RUN/'pipeline','completed',accepted=False)
            save(RUN/'pipeline/exit.json',dict(exit_code=0,completed_at=now()))
            print(json.dumps(result,ensure_ascii=False),flush=True);return
        base_path = PREVIOUS / 'base/evaluation/validation/result.json'
        base = load(base_path)
        assert base['source_sha256'] == BASE_SHA
        assert {r['case'] for r in base['rows']} == {c['index'] for c in cases}
        save(RUN / 'base/evaluation/validation/result.json', base)
        current = evaluate('speed', SOURCE, 'validation', cases)
        difference=comparison(current,base)
        accepted=valid(base) and valid(current) and difference['mean_T_difference']<=0 and difference['mean_S_difference']<=0
        assessment=dict(accepted=accepted,difference=difference,speed=mechanism['speed'],
                        base_valid=valid(base),current_valid=valid(current),base_reused=str(base_path.relative_to(ROOT)),fixed_at=now())
        save(RUN/'assessment.json',assessment)
        result=dict(assessment=assessment,validation=dict(base=base,speed=current),mechanism_sha256=sha(RUN/'mechanism/result.json'))
        if accepted:
            candidate = dict(source=str(SOURCE.relative_to(ROOT)), source_sha256=sha(SOURCE),
                             fixed_at=now(), selection='fixed_validation_256')
            save(RUN / 'final/candidate.json', candidate)
            final_in = evaluate('final', SOURCE, 'final_in')
            reference = load(RUN / 'in_reference.json')
            final = dict(candidate=candidate, tools_in=final_in, final_valid=valid(final_in),
                fixed_reference_relative=sum(reference['values'][r['filename']]/r['T'] for r in final_in['rows']),
                reference_sha256=sha(RUN / 'in_reference.json'), completed_at=now())
            save(RUN / 'final/result.json', final)
            result['final'] = final
        else:
            result['retained'] = dict(version='v113', source=str(BASE.relative_to(ROOT)), sha256=sha(BASE))
        result['completed_at'] = now()
        save(RUN / 'result.json', result)
        status(RUN / 'pipeline', 'completed', accepted=accepted)
        save(RUN / 'pipeline/exit.json', dict(exit_code=0, seconds=time.monotonic()-started, completed_at=now()))
        print(json.dumps(assessment, ensure_ascii=False), flush=True)
    except BaseException as e:
        status(RUN / 'pipeline', 'failed', error=repr(e))
        save(RUN / 'pipeline/exit.json', dict(exit_code=1, error=repr(e), traceback=traceback.format_exc(), completed_at=now()))
        raise


if __name__ == '__main__':
    run()
