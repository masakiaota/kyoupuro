#!/usr/bin/env python3
"""開発用の判定を通過した固定重みを、公式記録へ一度ずつ評価する。"""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import traceback

from build_v121_guidance import build
from check_v089_board import trace
from run_v089_evaluation import validated_output,statistics
from v111_portfolio import comparison,valid
from v116_data import ROOT,BC,save,load,sha,now
from v121_data import RUN

BASE=ROOT/'results/nn_rank/v111/20261004_weight_portfolio_studio'


def evaluate(source,role):
    where=RUN/'evaluation'/role;where.mkdir(parents=True,exist_ok=True)
    marker=where/'result.json'
    if marker.exists():
        result=load(marker);assert result['source_sha256']==sha(source);return result
    if role=='final_in':
        cases=[dict(index=i,filename=p.name,path=str(p),sha256=sha(p)) for i,p in enumerate(sorted((ROOT/'tools/in').glob('*.txt')))]
    else:
        cases=[dict(c,filename=f"{c['index']:06d}.txt") for c in load(BC/'input_manifest.json') if c['role']==role]
    assert len(cases)==(100 if role=='final_in' else 256)
    inputs=where/'inputs';inputs.mkdir(exist_ok=True)
    for case in cases:
        original=Path(case['path']) if role=='final_in' else BC/case['path']
        assert sha(original)==case['sha256'];shutil.copy2(original,inputs/case['filename'])
    save(where/'input_manifest.json',cases)
    name=f'v121_finite_value_{role}';wrapper=ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag=f'v121_finite_value_{role}_studio_j12';scratch=ROOT/'results/out'/name
    started=where/'started.json';identity=dict(source_sha256=sha(source),label=tag,jobs=12,manifest_sha256=sha(where/'input_manifest.json'))
    if not started.exists():
        assert not scratch.exists(),scratch
        save(started,identity)
        save(RUN/'pipeline/status.json',dict(stage='evaluating',role=role,pid=os.getpid(),updated_at=now()))
        with (where/'eval.log').open('w') as stream:
            proc=subprocess.run([sys.executable,ROOT/'scripts/eval.py',name,inputs,'-j','12','--wait-lock','--label',tag],
                                cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(where/'exit.json',dict(exit_code=proc.returncode,finished_at=now()))
    else:assert load(started)==identity
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
        name=case['filename'];record=indexed[name];assert record['status']=='ok' and record['local']
        outcome=validated_output(inputs/name,(outputs/name).read_text());assert outcome['S']==record['score']
        stderr=(outputs/(name+'.err')).read_text();counts=trace(stderr)
        custom={k:float(v) for k,v in re.findall(r'\b(v31[12]_\w+|pre_lns|lns_attempts)=(-?\d+(?:\.\d*)?(?:e[+-]?\d+)?)',stderr)}
        for key,diagnostic in [('inferences','v312_inferences'),('depth','v312_nn_steps'),('deadline','v312_nn_deadline')]:
            counts.setdefault(key,int(custom.get(diagnostic,0)))
        rows.append(dict(case=case['index'],filename=name,elapsed_ms=record['elapsed'],trace=counts,construction=custom,**outcome))
    result=dict(metrics=statistics(rows),rows=rows,source_sha256=sha(source),all_legal=True,completed_at=now())
    save(marker,result);return result


def main():
    marker=RUN/'result.json'
    if marker.exists():return load(marker)
    assert load(RUN/'development/result.json')['gate_passed']
    assert load(RUN/'mechanism/cpp.json')['passed']
    assert load(RUN/'training/result.json')['epochs']==120
    source=ROOT/'adhoc/bin/v121_finite_value.cpp'
    assert sha(source)==load(RUN/'mechanism/cpp.json')['source_sha256']
    learned=evaluate(source,'validation');baseline=load(BASE/'learned/evaluation/validation/result.json')
    difference=comparison(learned,baseline)
    activated=sum(r['trace'].get('finite_value_calls',0) for r in learned['rows'])
    assert activated>0,'learned ordering did not run'
    accepted=valid(learned) and difference['mean_T_difference']<0 and difference['mean_S_difference']<0
    assessment=dict(accepted=accepted,valid=valid(learned),comparison=difference,finite_value_calls=activated,
                    metrics=learned['metrics'],source_sha256=sha(source),completed_at=now())
    save(RUN/'assessment.json',assessment)
    finals={};candidate=None
    if accepted:
        submitted=ROOT/'src/bin/v121_nn_finite_value.cpp'
        assert not submitted.exists(),'inspect existing candidate before resuming'
        candidate=build(submitted,RUN/'training/model.json')
        assert source.read_text().split('\n',1)[1]==submitted.read_text().split('\n',1)[1]
        candidate.update(frozen_at=now(),selection_role='validation',test_used_for_selection=False)
        save(RUN/'final/candidate.json',candidate)
        for role in ['test','final_in']:
            result=evaluate(submitted,role)
            before=load(BASE/'final/evaluation'/role/'result.json')
            finals[role]=dict(metrics=result['metrics'],valid=valid(result),comparison=comparison(result,before))
        save(RUN/'final/result.json',dict(candidate=candidate,evaluations=finals,completed_at=now()))
    result=dict(experiment='v121',accepted=accepted,assessment=assessment,candidate=candidate,final_evaluations=finals,
                retained_if_rejected='v111 and other previously saved candidates',completed_at=now())
    save(marker,result);return result


if __name__=='__main__':
    try:
        result=main();save(RUN/'pipeline/status.json',dict(stage='completed',accepted=result['accepted'],updated_at=now()))
        save(RUN/'pipeline/exit.json',dict(exit_code=0,finished_at=now()));print(json.dumps(result,indent=2))
    except BaseException:
        save(RUN/'pipeline/exit.json',dict(exit_code=1,error=traceback.format_exc(),finished_at=now()));raise
