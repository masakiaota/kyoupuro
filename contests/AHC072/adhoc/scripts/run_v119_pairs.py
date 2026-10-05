#!/usr/bin/env python3
"""固定32入力の短縮前後を採取し、独立再生と表現照合を入力単位で保存する。"""
from concurrent.futures import ThreadPoolExecutor,as_completed
from pathlib import Path
from datetime import datetime
import json,os,subprocess,time
import numpy as np
from v116_data import ROOT,BC,selected,save,load,sha,now

RUN=ROOT/'results/nn_rank/v119/20261004_matched_pairs_studio'
DEADLINE=datetime.fromisoformat('2026-10-04T20:06:00+09:00').timestamp()

def one(item,identity):
    assert time.time()<DEADLINE,'pilot time limit'
    case=RUN/'cases'/f"{item['index']:06d}";case.mkdir(parents=True,exist_ok=True)
    marker=case/'complete.json'
    if marker.exists():
        old=load(marker);assert old['identity']==identity
        for name,digest in old['sha256'].items():assert sha(case/name)==digest
        return old
    inp=BC/item['path'];assert sha(inp)==item['sha256']
    started=time.monotonic()
    pairs=case/'pairs.txt'
    if not pairs.exists():
        env=dict(os.environ,V119_PAIRS=str(case/'pairs.partial.txt'))
        process=subprocess.run([RUN/'frozen/recorder'],input=inp.read_bytes(),capture_output=True,env=env,cwd=ROOT,timeout=30)
        (case/'solver.err').write_bytes(process.stderr);(case/'answer.txt').write_bytes(process.stdout)
        assert process.returncode==0,('recorder',item['index'],process.returncode,process.stderr[-1000:])
        assert b'LNS diagnostic:' not in process.stderr,process.stderr[-2000:]
        (case/'pairs.partial.txt').replace(pairs)
    process=subprocess.run([RUN/'frozen/checker',inp,pairs,case],capture_output=True,text=True,cwd=ROOT,timeout=120)
    (case/'checker.err').write_text(process.stderr)
    assert process.returncode==0,('checker',item['index'],process.returncode,process.stderr[-1000:])
    stats=json.loads(process.stdout)
    raw=np.fromfile(case/'samples.raw',dtype='<f4').reshape(-1,52)
    assert len(raw)==stats.get('labels',0) and np.isfinite(raw).all()
    assert (raw[:,48]>=raw[:,0]).all()
    result={'input':item,'identity':identity,'statistics':stats,'seconds':time.monotonic()-started,
            'sha256':{name:sha(case/name) for name in ['pairs.txt','teachers.jsonl','samples.raw','rejections.jsonl','answer.txt']},'completed_at':now()}
    save(marker,result);return result

def main():
    items=selected()[:32]
    identity={'recorder_sha256':sha(RUN/'frozen/recorder'),'checker_sha256':sha(RUN/'frozen/checker'),
              'parent_sha256':sha(ROOT/'src/bin/v111_weight_portfolio.cpp'),'case_count':32,'parallelism':12}
    if (RUN/'identity.json').exists():assert load(RUN/'identity.json')==identity
    else:save(RUN/'identity.json',identity)
    save(RUN/'input_manifest.json',items)
    results=[];started=time.monotonic()
    with ThreadPoolExecutor(max_workers=12) as pool:
        for f in as_completed([pool.submit(one,x,identity) for x in items]):
            results.append(f.result());save(RUN/'status.json',{'stage':'collect_and_check','completed':len(results),'total':32,'seconds':time.monotonic()-started,'updated_at':now()})
    total={}
    for case in results:
        for k,v in case['statistics'].items():total[k]=total.get(k,0)+v
    cases_with=sum(x['statistics'].get('certified_pairs',0)>0 for x in results)
    beats_cases=sum(x['statistics'].get('teacher_beats_baseline',0)>0 for x in results)
    result={'cases':32,'statistics':total,'cases_with_teachers':cases_with,'cases_beating_baseline':beats_cases,
            'teacher_gate_passed':cases_with>=2 and total.get('certified_pairs',0)>=4,
            'guidance_signal_passed':total.get('teacher_beats_baseline',0)>=1,
            'seconds':time.monotonic()-started,'completed_at':now(),'identity':identity,
            'training_performed':False,'official_evaluation_performed':False}
    save(RUN/'result.json',result);save(RUN/'status.json',dict(stage='completed',**result));print(json.dumps(result,ensure_ascii=False,indent=2))

if __name__=='__main__':main()
