#!/usr/bin/env python3
import difflib
import json
from concurrent.futures import ThreadPoolExecutor
import subprocess
from build_v117_pickup import ROOT, RUN
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
    identity={k:sha(v) for k,v in sources.items()}
    if (where/'result.json').exists():
        result=load(where/'result.json');assert result['sources']==identity;return result
    def build(item):
        label,source=item;dest=where/label;dest.mkdir(exist_ok=True)
        for local in (True,False):
            mode='local' if local else 'judge';binary=dest/('solver_'+mode);expanded=dest/(mode+'.ii')
            if binary.exists() and expanded.exists() and min(binary.stat().st_mtime,expanded.stat().st_mtime)>source.stat().st_mtime:continue
            compile_binary(source.stem,binary,local);preprocess(source,expanded,local)
    with ThreadPoolExecutor(max_workers=2) as pool:list(pool.map(build,sources.items()))
    for local in (True,False):
        mode='local' if local else 'judge';reference=normalize((where/'base'/(mode+'.ii')).read_text())
        expanded=normalize((where/'pickup'/(mode+'.ii')).read_text())
        for name in ('namespace nn {','struct TimeKeeper','struct Board {','struct NNSharedDeadline',
                     'void grow_initial_solutions(','static void neural_extra_starts(',
                     'bool insert_at(','bool insert('):
            assert block(expanded,name)==block(reference,name),(mode,name)
        (where/'pickup'/(mode+'.patch')).write_text(''.join(difflib.unified_diff(reference.splitlines(True),expanded.splitlines(True))))
    fixture=where/'fixture';fixture.mkdir(exist_ok=True)
    cells=[list('.'*12) for _ in range(12)]
    for i,j,c in [(5,3,'a'),(5,4,'b'),(5,8,'A'),(5,7,'B'),(8,7,'c'),(8,8,'C'),(9,7,'d'),(9,8,'D')]:cells[i][j]=c
    (fixture/'input.txt').write_text('12 4\n'+'\n'.join(''.join(r) for r in cells)+'\n')
    commands=['5 4 0 R 1','5 5 0 R 1','5 6 0 R 1','8 7 0 R 1','9 7 0 R 1']
    commands += [f'5 {j} 0 R 1' for j in range(3,8)]
    (fixture/'saved.txt').write_text('\n'.join(commands)+'\n')
    assert validated_output(fixture/'input.txt',(fixture/'saved.txt').read_text())['E']==0
    cases=load(SAVED/'input_manifest.json')
    cases=[(c['filename'],SAVED/'inputs'/c['filename'],SAVED/'outputs'/c['filename'],False)
           for c in [cases[i] for i in (0,31,63,95,127,159,191,223)]]
    cases.append(('fixture',fixture/'input.txt',fixture/'saved.txt',True))
    binary=where/'probe';compile_binary('check_v117_pickup',binary,True)
    totals={};outputs=0
    for name,input_path,saved,is_fixture in cases:
        dest=where/'outputs'/name
        proc=subprocess.run([binary,saved,dest]+(['fixture'] if is_fixture else []),
                            input=input_path.read_text(),text=True,capture_output=True,timeout=120)
        (where/(name+'.err')).write_text(proc.stderr)
        assert proc.returncode==0,(name,proc.stderr[-2000:])
        if not is_fixture:totals[name]=load(dest/'result.json')
        else:fixtures=load(dest/'fixture.json')
        for output in dest.glob('*.txt'):
            assert validated_output(input_path,output.read_text())['E']==0
            outputs+=1
    actual=sum(v['pickup'] for v in totals.values())
    result=dict(sources=identity,totals=totals,fixtures=fixtures,independent_outputs=outputs,
                natural_pickup=actual,gates={'pickup':dict(passed=actual>0)},completed_at=now())
    save(where/'result.json',result);return result

if __name__=='__main__':
    result=check();print(json.dumps({k:v for k,v in result.items() if k!='totals'},ensure_ascii=False))
