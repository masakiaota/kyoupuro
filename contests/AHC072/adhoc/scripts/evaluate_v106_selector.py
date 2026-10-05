#!/usr/bin/env python3
"""固定モデルのLNS全体比較と、採用時だけの最終評価。"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
from v089_data import ROOT, save, sha, now, status
from v090_data import RUN as BC_RUN
from v106_data import RUN
from check_v089_board import compile_binary,trace
from run_v089_evaluation import validated_output,statistics


def load(p):return json.loads(p.read_text())


def evaluate(root,role):
    source=ROOT/'src/bin/v106_nn_lns.cpp';where=root/'integration/evaluation'/role;where.mkdir(parents=True,exist_ok=True)
    marker=where/'result.json'
    if marker.exists():
        result=load(marker);assert result['source_sha256']==sha(source);return result
    if role=='final_in':
        cases=[dict(index=i,filename=p.name,sha256=sha(p),path=str(p)) for i,p in enumerate(sorted((ROOT/'tools/in').glob('*.txt')))]
    else:
        cases=[dict(c,filename=f"{c['index']:06d}.txt") for c in load(BC_RUN/'input_manifest.json') if c['role']==role]
    assert len(cases)==(100 if role=='final_in' else 256)
    inputs=where/'inputs';inputs.mkdir(exist_ok=True)
    for c in cases:
        path=Path(c['path']) if role=='final_in' else BC_RUN/c['path']
        assert sha(path)==c['sha256'];shutil.copy2(path,inputs/c['filename'])
    save(where/'input_manifest.json',cases)
    jobs=12 if role=='validation' else 20
    name=f'v106_nn_lns_{role}';wrapper=ROOT/'adhoc/bin'/f'{name}.cpp'
    wrapper.write_text(f'// {name}.cpp\n#include "../../src/bin/v106_nn_lns.cpp"\n')
    label=f'v106_selector_{role}_studio_j{jobs}';scratch=ROOT/'results/out'/name
    identity=dict(source_sha256=sha(source),model_sha256=sha(root/'main/model.json'),manifest_sha256=sha(where/'input_manifest.json'),jobs=jobs,label=label)
    started=where/'started.json'
    if not started.exists():
        assert not scratch.exists(),f'prior scratch exists: {scratch}'
        for local in (True,False):compile_binary(name,where/('solver_local' if local else 'solver_judge'),local)
        save(started,identity);status(root/'integration','evaluating',role=role,cases=len(cases),jobs=jobs)
        with (where/'eval.log').open('w') as f:
            p=subprocess.run([sys.executable,ROOT/'scripts/eval.py',name,inputs,'-j',str(jobs),'--wait-lock','--label',label],cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
        save(where/'exit.json',dict(exit_code=p.returncode,finished_at=now()))
    else:assert load(started)==identity
    assert load(where/'exit.json')['exit_code']==0,'inspect saved records before resuming a failed evaluation'
    records=[]
    with (ROOT/'results/eval_records.jsonl').open() as f:
        for line in f:
            record=json.loads(line)
            if record['label']==label:records.append(record)
    assert len(records)==len({r['case_name'] for r in records})==len(cases)
    assert len({r['run_id'] for r in records})==1
    save(where/'official_records.json',records);outputs=where/'outputs'
    if not outputs.exists():shutil.copytree(scratch,outputs)
    indexed={r['case_name']:r for r in records};rows=[]
    for c in cases:
        record=indexed[c['filename']];assert record['status']=='ok' and record['local']
        result=validated_output(inputs/c['filename'],(outputs/c['filename']).read_text())
        assert record['score']==result['S']
        counts=trace((outputs/(c['filename']+'.err')).read_text())
        rows.append(dict(case=c['index'],filename=c['filename'],elapsed_ms=record['elapsed'],trace=counts,**result))
    result=dict(metrics=statistics(rows),rows=rows,all_legal=True,source_sha256=sha(source),jobs=jobs,completed_at=now())
    save(marker,result);return result


def run(root):
    mechanism=load(root/'integration/mechanism.json');assert mechanism['passed']
    source=ROOT/'src/bin/v106_nn_lns.cpp'
    assert mechanism['source_sha256']==sha(source) and mechanism['model_sha256']==sha(root/'main/model.json')
    result=evaluate(root,'validation')
    before=load(ROOT/'results/nn_rank/v105/20261004_nn_lns_studio/evaluation/hybrid/result.json')
    old={r['case']:r for r in before['rows']};assert set(old)=={r['case'] for r in result['rows']}
    common=[(old[r['case']],r) for r in result['rows'] if old[r['case']]['E']==r['E']==0]
    delta=sum(b['T']-a['T'] for a,b in common)/len(common) if common else None
    structural=all(r['trace'].get('state_pool_free_at_end')==4 and r['trace'].get('nn_calls',0)>0
        and all(r['trace'].get(k,0)==0 for k in ('lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery')) for r in result['rows'])
    metric=result['metrics'];score_delta=metric['mean_S_all']-before['metrics']['mean_S_all']
    passed=structural and metric['complete']==256 and metric['deadline_cases']==0 and metric['max_elapsed_ms']<=2000 and delta<0 and score_delta<0
    record=dict(adopted=passed,structural_passed=structural,comparison=dict(mean_T_difference_on_common=delta,
                common_completed=len(common),mean_S_difference_all=score_delta,baseline=before['metrics'],current=metric),
                source_sha256=sha(source),model_sha256=sha(root/'main/model.json'),completed_at=now())
    save(root/'integration/result.json',record)
    if passed:
        final=root/'final';final.mkdir(exist_ok=True)
        candidate=dict(source=str(source.relative_to(ROOT)),source_sha256=sha(source),model_sha256=sha(root/'main/model.json'),frozen_at=now())
        if (final/'candidate.json').exists():
            previous=load(final/'candidate.json');assert previous['source_sha256']==candidate['source_sha256'] and previous['model_sha256']==candidate['model_sha256']
        else:save(final/'candidate.json',candidate);shutil.copy2(source,final/source.name)
        test=evaluate(root,'test');inputs=evaluate(root,'final_in')
        save(final/'result.json',dict(candidate=load(final/'candidate.json'),test=test,final_in=inputs,
             comparison_set_notice='過去にも評価済みの固定比較集合。学習と条件選別には使わない。',completed_at=now()))
    status(root/'integration','completed',adopted=passed,mean_T_difference_on_common=delta,mean_S_difference_all=score_delta)
    print(json.dumps(record),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args();run(a.run)
