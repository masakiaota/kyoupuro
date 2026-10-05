#!/usr/bin/env python3
"""診断列の再現と、事前登録した未学習256入力だけでBC候補を比較する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import subprocess
import time
import numpy as np
import torch
from assess_v099_complete import numerical,evaluate,compare
from check_v089_board import compile_binary,trace
from run_v089_evaluation import validated_output,statistics
from v089_data import ROOT,save,sha,now
from v091_env import BC_RUN,load
from v103_data import PARENT


def diagnostic(root):
    where=root/'diagnostic';marker=where/'assessment.json'
    if marker.exists():return load(marker)
    source=ROOT/'adhoc/bin/v103_diagnostic.cpp';numerical(where,source)
    binary=where/'binaries/solver';compile_binary(source.stem,binary)
    metadata=load(where/'data/shortened/dataset.json');directory=where/'rollouts';directory.mkdir(exist_ok=True)
    def one(case):
        output=directory/f"{case['index']:06d}.txt";record=directory/f"{case['index']:06d}.json"
        if record.exists():return load(record)
        assert not output.exists(),'partial rollout requires inspection'
        path=where/'data'/case['path'];start=time.monotonic()
        proc=subprocess.run([binary],input=path.read_text(),capture_output=True,text=True,check=True,timeout=30)
        elapsed=(time.monotonic()-start)*1000;output.write_text(proc.stdout);output.with_suffix('.err').write_text(proc.stderr)
        replay=validated_output(path,proc.stdout);counts=trace(proc.stderr)
        assert counts['T']==replay['T'] and counts['E']==replay['E']
        row=dict(case=case['index'],original_T=case['original_T'],teacher_T=case['T'],elapsed_ms=elapsed,trace=counts,**replay)
        save(record,row);return row
    with ThreadPoolExecutor(max_workers=20) as pool:rows=list(pool.map(one,metadata['cases']))
    training=load(where/'training/result.json');m=statistics(rows);diff=float(np.mean([r['T']-r['original_T'] for r in rows]))
    ratio=training['final']['ce']/training['initial']['ce']
    passed=training['complete_budget'] and m['complete']==len(rows) and m['deadline_cases']==0 and diff<=-1 and ratio<=.9
    result=dict(passed=bool(passed),diagnostic_only=True,metrics=m,rows=rows,mean_T_difference=diff,ce_ratio=ratio,
                initial=training['initial'],final=training['final'],solver_sha256=sha(source),completed_at=now())
    save(marker,result);return result


def assess(root,variant):
    where=root/'main'/variant;marker=where/'assessment.json'
    if marker.exists():return load(marker)
    source=ROOT/'adhoc/bin'/f'v103_{variant}.cpp';numerical(where,source)
    cases=[dict(c,filename=f"{c['index']:06d}.txt") for c in load(BC_RUN/'input_manifest.json') if c['role']=='validation']
    result=evaluate(where,'validation',cases,source)
    baseline=load(PARENT/'evaluation/validation/greedy/result.json')
    save(marker,dict(greedy=result,comparison=compare(result,baseline),completed_at=now()))
    return load(marker)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=Path);p.add_argument('--variant',choices=('diagnostic','original','shortened'),required=True)
    a=p.parse_args();torch.set_num_threads(2);torch.set_num_interop_threads(2)
    result=diagnostic(a.run.resolve()) if a.variant=='diagnostic' else assess(a.run.resolve(),a.variant)
    print({k:v for k,v in result.items() if k not in ('rows','greedy')})
