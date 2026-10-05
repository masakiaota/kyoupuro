#!/usr/bin/env python3
"""事前固定した入力・乱数で計画ごとの残予算の完成費用を採取する。"""
from concurrent.futures import ThreadPoolExecutor, as_completed
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

from build_v124_allocation import ROOT, RUN, BASE, BASE_SHA, build_collector
from check_v089_board import compile_binary, trace
from check_v113_integrated import block, normalize
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from collect_v122_orders import codes_to_text
from v089_data import save, sha, now, status
from v090_data import RUN as BC_RUN

REPEATS=[124101,124102,124103,124104]
def load(path):return json.loads(path.read_text())


def prepare_inputs():
    marker=RUN/'input_manifest.json'
    if marker.exists():return load(marker)
    blocked=set(load(ROOT/'results/nn_rank/v123/20261004_order_studio/input_exclusion.json')['excluded_hashes'])
    blocked.update(c['sha256'] for c in load(BC_RUN/'input_manifest.json'))
    # validation1には触れず、保存済みの生成記録と入力マニフェストだけ照合する。
    paths=subprocess.check_output(['rg','--files','--hidden','--no-ignore','results/nn_rank','-g','input_manifest.json','-g','inputs.json','-g','generated.jsonl'],cwd=ROOT,text=True).splitlines()
    def remember(value):
        if isinstance(value,dict):
            for k,v in value.items():
                if k in ['sha256','input_sha256','fingerprint'] and isinstance(v,str) and re.fullmatch('[0-9a-f]{64}',v):blocked.add(v)
                elif isinstance(v,(list,dict)):remember(v)
        elif isinstance(value,list):
            for v in value:remember(v)
    for name in paths:
        path=ROOT/name
        if path.suffix=='.jsonl':
            with path.open() as stream:
                for line in stream:
                    if line.endswith('\n'):remember(json.loads(line))
        else:remember(load(path))
    save(RUN/'excluded_hashes.json',sorted(blocked))
    generator=RUN/'official_gen';shutil.copy2(ROOT/'tools/target/release/gen',generator)
    dest=RUN/'inputs';dest.mkdir(exist_ok=True);temp=RUN/'generation';temp.mkdir(exist_ok=True)
    cases=[]
    for i in range(384):
        role='train' if i<128 else 'validation';index=i if i<128 else i-128
        base=1240000000000 if i<128 else 1241000000000
        for offset in range(64):
            seed=base+index*64+offset;(temp/'seed.txt').write_text(str(seed)+'\n')
            subprocess.run([generator,temp/'seed.txt','--dir',temp/'output'],check=True,capture_output=True,timeout=30)
            path=dest/f'{i:04d}.txt';shutil.copy2(temp/'output/0000.txt',path)
            fingerprint=sha(path)
            if fingerprint not in blocked:break
        else:raise RuntimeError('seed range exhausted')
        blocked.add(fingerprint);cases.append(dict(index=i,role=role,seed=seed,path=str(path.relative_to(RUN)),sha256=fingerprint))
    save(marker,cases);save(RUN/'generator.json',dict(sha256=sha(generator),excluded=len(blocked)-384));return cases


def collect_one(case,repeat,binary,where,original=None):
    marker=where/'result.json';path=original or RUN/case['path']
    identity=dict(input_sha256=sha(path),binary_sha256=sha(binary),repeat=repeat)
    if marker.exists():
        result=load(marker);assert result['identity']==identity;return result
    where.mkdir(parents=True,exist_ok=True)
    began=time.monotonic()
    with path.open() as inp,(where/'output.txt').open('w') as out,(where/'stderr.log').open('w') as err:
        subprocess.run([binary,str(repeat),where/'labels.jsonl'],stdin=inp,stdout=out,stderr=err,check=True,timeout=180)
    checked=validated_output(path,(where/'output.txt').read_text());counts=trace((where/'stderr.log').read_text())
    assert checked['E']==0 and counts['state_pool_free_at_end']==4
    assert all(counts.get(k,0)==0 for k in ['construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery'])
    rows=[json.loads(line) for line in (where/'labels.jsonl').read_text().splitlines()]
    assert len({(r['phase'],r['candidate']) for r in rows})==len(rows)
    phases=sorted({r['phase'] for r in rows});replays=0
    for phase in phases:
        part=[r for r in rows if r['phase']==phase]
        assert len(part)==part[0]['count'] and all(r['after']<=r['before'] for r in part)
        # C++は全列を別表現で再生済み。Pythonでも各観測の1列を独立確認する。
        row=part[0];replayed=validated_output(path,codes_to_text(row['path']))
        assert replayed['E']==0 and replayed['T']==row['after'];replays+=1
    result=dict(identity=identity,case=case['index'],repeat=repeat,rows=len(rows),phases=phases,
                count=counts['race_seed_count'],cpp_replays=len(rows),python_replays=replays,
                parent_result=checked,parent_initial_hash=counts['nn_initial_hash'],seconds=time.monotonic()-began,completed_at=now())
    save(marker,result);return result


def mechanism():
    where=RUN/'mechanism';where.mkdir(exist_ok=True);marker=where/'result.json'
    if marker.exists():return load(marker)
    source=build_collector()
    for local in (True,False):
        mode='local' if local else 'judge';compile_binary(source.stem,where/('collector_'+mode),local)
        actual=normalize(preprocess(source,where/(mode+'.ii'),local));before=normalize(preprocess(BASE,where/('parent_'+mode+'.ii'),local))
        for name in ['namespace nn {','struct TimeKeeper','class TemporalLNS {','struct SearchReductions',
                     'static void neural_extra_starts(','struct InitialSolutions {']:
            assert block(actual,name)==block(before,name),(mode,name)
    cases=[c for c in load(BC_RUN/'input_manifest.json') if c['role']=='train' and c['M']>=80][:2]
    results=[]
    for c in cases:
        results.append(collect_one(c,REPEATS[0],where/'collector_local',where/str(c['index']),BC_RUN/c['path']))
    assert sum(r['rows'] for r in results)>=24
    result=dict(passed=True,cases=results,source_sha256=sha(source),parent_sha256=sha(BASE),completed_at=now())
    save(marker,result);return result


def run():
    RUN.mkdir(exist_ok=True);lock=(RUN/'collection.lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (RUN/'collection/exit.json').exists() and load(RUN/'collection/exit.json')['exit_code']==0:return
    began=time.monotonic();save(RUN/'launch.json',dict(pid=os.getpid(),stage='collection',started_at=now()))
    try:
        status(RUN/'collection','mechanism',pid=os.getpid());mechanism()
        cases=prepare_inputs();binary=RUN/'mechanism/collector_local'
        frozen=RUN/'frozen';frozen.mkdir(exist_ok=True)
        files=['build_v124_allocation.py','run_v124_collection.py','v124_features.cpp.txt','v124_collect.cpp.txt']
        identity=dict(base_sha256=BASE_SHA,source_sha256=sha(ROOT/'adhoc/bin/collect_v124_allocation.cpp'),
                      scripts={n:sha(ROOT/'adhoc/scripts'/n) for n in files},inputs_sha256=sha(RUN/'input_manifest.json'),repeats=REPEATS,jobs=12)
        if (RUN/'collection_config.json').exists():assert load(RUN/'collection_config.json')==identity
        else:
            save(RUN/'collection_config.json',identity)
            for name in files:shutil.copy2(ROOT/'adhoc/scripts'/name,frozen/name)
            shutil.copy2(ROOT/'adhoc/bin/collect_v124_allocation.cpp',frozen/'collect_v124_allocation.cpp')
        jobs=[(c,repeat) for c in cases if c['role']=='train' for repeat in REPEATS]
        with ThreadPoolExecutor(max_workers=12) as pool:
            pending=[pool.submit(collect_one,c,repeat,binary,RUN/'data'/f"{c['index']:04d}"/str(repeat)) for c,repeat in jobs]
            for i,f in enumerate(as_completed(pending),1):
                f.result();status(RUN/'collection','collecting',completed=i,total=len(jobs),seconds=time.monotonic()-began,pid=os.getpid())
        save(RUN/'collection/exit.json',dict(exit_code=0,seconds=time.monotonic()-began,completed_at=now()))
        status(RUN/'collection','completed',completed=len(jobs),total=len(jobs))
    except BaseException as e:
        status(RUN/'collection','failed',error=repr(e));save(RUN/'collection/exit.json',dict(exit_code=1,error=repr(e),traceback=traceback.format_exc(),completed_at=now()));raise


if __name__=='__main__':run()
