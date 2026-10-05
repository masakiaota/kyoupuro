#!/usr/bin/env python3
import difflib
import csv
import json
from concurrent.futures import ThreadPoolExecutor
import subprocess
from pathlib import Path

from build_v115_corridor import ROOT, RUN
from check_v089_board import compile_binary
from check_v113_integrated import block, normalize
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save, sha, now

SAVED=ROOT/'results/nn_rank/v113/20261004_integrated_studio/hr/evaluation/validation'

def load(path):return json.loads(path.read_text())

def check():
    where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True)
    sources={'base':ROOT/'src/bin/v113_integrated_nn_lns.cpp',
             **{k:ROOT/v['path'] for k,v in load(RUN/'sources.json')['sources'].items()}}
    if (where/'result.json').exists():
        result=load(where/'result.json')
        assert result['sources']=={k:sha(v) for k,v in sources.items()}
        return result
    def build(item):
        label,source=item;dest=where/label;dest.mkdir(exist_ok=True)
        for local in (True,False):
            mode='local' if local else 'judge';binary=dest/('solver_'+mode);expanded=dest/(mode+'.ii')
            if binary.exists() and expanded.exists() and min(binary.stat().st_mtime,expanded.stat().st_mtime)>source.stat().st_mtime:continue
            compile_binary(source.stem,binary,local);preprocess(source,expanded,local)
    with ThreadPoolExecutor(max_workers=3) as pool:list(pool.map(build,sources.items()))
    for local in (True,False):
        mode='local' if local else 'judge';reference=normalize((where/'base'/(mode+'.ii')).read_text())
        for label in ('r0','r1'):
            expanded=normalize((where/label/(mode+'.ii')).read_text())
            for name in ('namespace nn {','struct TimeKeeper','struct Board {','struct NNSharedDeadline',
                         'void grow_initial_solutions(','static void neural_extra_starts('):
                assert block(expanded,name)==block(reference,name),(label,mode,name)
            (where/label/(mode+'.patch')).write_text(''.join(difflib.unified_diff(reference.splitlines(True),expanded.splitlines(True))))
    binary=where/'probe';source=ROOT/'adhoc/bin/check_v115_corridor.cpp'
    compile_binary(source.stem,binary,True)
    cases=load(SAVED/'input_manifest.json');totals=[];outputs=0
    for index in (0,31,63,95,127,159,191,223):
        name=cases[index]['filename'];dest=where/'outputs'/name
        proc=subprocess.run([binary,SAVED/'outputs'/name,dest],input=(SAVED/'inputs'/name).read_text(),
                            text=True,capture_output=True,timeout=90)
        (where/(name+'.err')).write_text(proc.stderr)
        assert proc.returncode==0,(name,proc.stderr[-2000:])
        with (dest/'timing.csv').open() as stream:
            for row in csv.DictReader(stream):totals.append(dict(case=name,**{k:float(v) for k,v in row.items()}))
        for plan in dest.glob('*/*.txt'):
            assert validated_output(SAVED/'inputs'/name,plan.read_text())['E']==0
            outputs+=1
    assert len(totals)==256
    parent_cpu=sum(r['base_cpu'] for r in totals);parent_ok=sum(r['base_ok'] for r in totals)
    gates={}
    for label in ('r0','r1'):
        cpu=sum(r[label+'_cpu'] for r in totals);complete=sum(r[label+'_ok'] for r in totals)
        gates[label]=dict(cpu=cpu,parent_cpu=parent_cpu,cpu_ratio=cpu/parent_cpu,
                          complete=complete,parent_complete=parent_ok,completion_ratio=complete/parent_ok,
                          passed=cpu<=.85*parent_cpu and complete>=.50*parent_ok and complete>0)
    result=dict(sources={k:sha(v) for k,v in sources.items()},gates=gates,rows=totals,
                independent_outputs=outputs,completed_at=now())
    save(where/'result.json',result);return result

if __name__=='__main__':print(json.dumps(check(),ensure_ascii=False))
