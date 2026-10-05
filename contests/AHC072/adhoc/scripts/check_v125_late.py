#!/usr/bin/env python3
"""前半の全域と後半の経路制限の発動、および既存処理の一致を確認する。"""
import json
import subprocess
import time
from build_v125_late import ROOT, RUN, BASE, CORRIDOR, build
from check_v113_integrated import block, normalize
from check_v089_board import compile_binary, trace
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save, sha, now
from v090_data import RUN as BC_RUN

def load(path):return json.loads(path.read_text())


def check():
    where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True);marker=where/'result.json'
    if marker.exists():return load(marker)
    source=build()
    for local in (True,False):
        mode='local' if local else 'judge';compile_binary(source.stem,where/('solver_'+mode),local)
        actual=normalize(preprocess(source,where/(mode+'.ii'),local))
        previous=normalize(preprocess(CORRIDOR,where/('corridor_'+mode+'.ii'),local))
        for name in ['namespace nn {','struct TimeKeeper','struct StatePool','class State {','struct SearchReductions',
                     'struct InitialSolutions {','static void neural_extra_starts(','void grow_initial_solutions(','int main() {']:
            assert block(actual,name)==block(previous,name),(mode,name)
        old=block(previous,'class TemporalLNS {');current=block(actual,'class TemporalLNS {')
        current=current.replace(block(current,'void optimize('),block(old,'void optimize('),1)
        assert current==old,(mode,'non-optimize changes')
    helper=ROOT/'adhoc/bin/check_v125_clock.cpp'
    helper.write_text('// '+helper.name+'\n#define main unused_solver_main\n#include "v125_late_corridor.cpp"\n#undef main\nint main(){double t,b,e;int i;while(cin>>t>>b>>e>>i)cout<<(late_corridor(t,b,e)&&i%4!=0)<<"\\n";return 0;}\n')
    compile_binary(helper.stem,where/'clock',False)
    fixtures=[]
    for begin in [0.,.125,.5]:
        for duration in [.125,.5,1.]:
            for fraction in [0.,.5-1/1024,.5,.5+1/1024,1.]:
                for iteration in range(1,9):fixtures.append((begin+duration*fraction,begin,begin+duration,iteration,fraction>=.5 and iteration%4!=0))
    payload='\n'.join(' '.join(map(str,row[:4])) for row in fixtures)+'\n'
    result=subprocess.run([where/'clock'],input=payload,text=True,capture_output=True,check=True)
    assert [int(v) for v in result.stdout.split()]==[int(r[4]) for r in fixtures]
    cases=[c for c in load(BC_RUN/'input_manifest.json') if c['role']=='train' and c['M']>=80][:4]
    reports=[]
    for case in cases:
        for local in (True,False):
            mode='local' if local else 'judge';path=BC_RUN/case['path'];tick=time.monotonic()
            proc=subprocess.run([where/('solver_'+mode)],input=path.read_text(),text=True,capture_output=True,check=True,timeout=10)
            elapsed=time.monotonic()-tick;checked=validated_output(path,proc.stdout);assert checked['E']==0
            counts=trace(proc.stderr)
            if local:
                assert counts['state_pool_free_at_end']==4
                assert all(counts.get(k,0)>0 for k in ['corridor_early_full','corridor_late_full','corridor_late_limited'])
                assert counts['corridor_restricted_groups']==counts['corridor_late_limited']
                assert counts['corridor_full_groups']==counts['corridor_late_full']+counts['corridor_early_full']
                assert all(counts.get(k,0)==0 for k in ['lns_errors','construction_errors','lns_invalid_candidates','baseline_recovery','final_recovery'])
            dest=where/f"{case['index']}_{mode}";dest.mkdir(exist_ok=True)
            (dest/'output.txt').write_text(proc.stdout);(dest/'stderr.log').write_text(proc.stderr)
            reports.append(dict(case=case['index'],mode=mode,seconds=elapsed,trace=counts,**checked))
    result=dict(passed=True,clock_cases=len(fixtures),source_sha256=sha(source),reports=reports,completed_at=now())
    save(marker,result);return result


if __name__=='__main__':check()
