#!/usr/bin/env python3
"""逆生成の合法性・最短手数との差・固定方策との比較を先に判定する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import fcntl
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time
import traceback
import numpy as np
from v089_data import Geometry,remaining,unpack,save,sha,now,status,ROOT
from v092_stream import excluded_inputs,normalized,digest,describe
from v091_env import load
from check_v089_board import trace

RUN=ROOT/'results/nn_rank/v093/20261003_reverse_studio'
PPO=ROOT/'results/nn_rank/v092/20261003_fresh_studio'
CONFIG=dict(pilot=64,depths=[32,100,256,512],short_examples=48,candidates=64,attempts=4096,
            bulk=1024,bulk_depth=512,bulk_seconds=5400,workers=8,seed=93003,official_seed_base=930000000000,
            exact_states=2000000,max_policy_steps=2048)


def prepare_inputs(root,role,count,offset):
    directory=root/role;directory.mkdir(parents=True,exist_ok=True);marker=directory/'inputs.json'
    if marker.exists():return load(marker)
    blocked=excluded_inputs();seeds=[CONFIG['official_seed_base']+offset+i for i in range(count)]
    seed_file=directory/'seeds.txt';seed_file.write_text(''.join(f'{seed}\n' for seed in seeds))
    subprocess.run([root/'generator',seed_file,'--dir',directory/'inputs'],check=True,stdout=subprocess.DEVNULL)
    rows=[];seen=set()
    for i,seed in enumerate(seeds):
        path=directory/'inputs'/f'{i:04d}.txt';text=normalized(path.read_text());fingerprint=digest(text)
        assert fingerprint not in blocked and fingerprint not in seen;seen.add(fingerprint)
        _,distribution=describe(text)
        rows.append(dict(index=i,seed=seed,path=str(path.relative_to(root)),sha256=sha(path),**distribution))
    save(marker,rows);return rows


def one(root,role,item,depth,exact=False):
    directory=root/role/'cases'/f"{item['index']:04d}";marker=directory/'result.json'
    if marker.exists():return load(marker)
    input_path=root/item['path'];assert sha(input_path)==item['sha256']
    started=time.monotonic()
    if (directory/'generated.json').exists():meta=load(directory/'generated.json')
    else:
        proc=subprocess.run([root/'reverse_generator',input_path,directory,str(depth),str(CONFIG['seed']+item['index']),str(int(exact))],
                            text=True,capture_output=True,check=True,timeout=180.)
        meta=json.loads(proc.stdout);save(directory/'generated.json',meta)
    T=meta['depth'];assert T>0
    states=np.fromfile(directory/'states.raw',dtype='<u4').reshape(T+1,400)
    actions=np.fromfile(directory/'actions.raw',dtype='<u4');lines=(directory/'solution.txt').read_text().splitlines()
    assert len(actions)==len(lines)==T
    geometry=Geometry(input_path);state=states[0].copy();seen=set()
    for t,line in enumerate(lines):
        assert np.array_equal(state,states[t]);key=state.tobytes();assert key not in seen;seen.add(key)
        assert geometry.apply(state,line)==actions[t]
        assert np.array_equal(state,states[t+1]),(item['index'],t)
    assert remaining(state)==0 and not np.any(state) and meta['E']==remaining(states[0])
    counts=[sum(unpack(int(bits)).count(c+1) for bits in states[0]) for c in range(geometry.K)]
    assert counts==meta['color_counts']
    if meta['singleton_start']:
        official=Geometry(directory/'singleton_input.txt');assert np.array_equal(official.initial,states[0])
        assert meta['E']==meta['target_M'] and meta['on_nests']==0
    terrain='\n'.join(''.join('.' if c.islower() else c for c in line) for line in geometry.C)
    fingerprint=hashlib.sha256(terrain.encode()+states[0].tobytes()).hexdigest()
    result=dict(item,**meta,target_color_counts=item.get('color_counts'),seconds=time.monotonic()-started,input_state_sha256=fingerprint,
                states_sha256=sha(directory/'states.raw'),actions_sha256=sha(directory/'actions.raw'),
                independent_replay=True,chain_duplicates=0,remaining_label='length of the generated solution, not an optimal distance')
    save(marker,result);return result


def policy(root,item):
    directory=root/'pilot/cases'/f"{item['index']:04d}";marker=directory/'policy.json'
    if marker.exists():return load(marker)
    binary=PPO/'numerical/start/checker_local';assert binary.exists()
    started=directory/'policy_started.json';assert not started.exists(),'partial policy evaluation needs inspection'
    save(started,dict(binary_sha256=sha(binary),started_at=now()))
    tick=time.monotonic();proc=subprocess.run([binary,directory/'snapshot.txt'],input=(root/item['path']).read_text(),
                                              text=True,capture_output=True,check=True,timeout=10.)
    elapsed=time.monotonic()-tick;(directory/'policy.txt').write_text(proc.stdout);(directory/'policy.err').write_text(proc.stderr)
    state=np.fromfile(directory/'states.raw',dtype='<u4',count=400);geometry=Geometry(root/item['path'])
    lines=[line for line in proc.stdout.splitlines() if line.strip()]
    for line in lines:geometry.apply(state,line)
    counters=trace(proc.stderr);E=remaining(state);assert counters['E']==E and counters['T']==len(lines)
    result=dict(index=item['index'],E=E,T=len(lines),elapsed_seconds=elapsed,all_legal=True,trace=counters)
    save(marker,result);return result


def exact_examples(root):
    directory=root/'exact';directory.mkdir(parents=True,exist_ok=True)
    rows=[]
    for i in range(CONFIG['short_examples']):
        # 小さな連結床。4色各1匹、巣は4つで、元入力の個体数を復元上限にする。
        lines=['AaBb'+'#'*8,'cCdD'+'#'*8]+['#'*12]*10
        path=directory/f'{i:04d}.txt';path.write_text('12 4\n'+'\n'.join(lines)+'\n')
        rows.append(dict(index=i,path=str(path.relative_to(root)),sha256=sha(path)))
    with ThreadPoolExecutor(max_workers=CONFIG['workers']) as pool:
        results=list(pool.map(lambda item:one(root,'exact',item,1+item['index']//8,True),rows))
    save(directory/'results.json',results);return results


def quality(root,pilot,exact,policies):
    assert all(row['independent_replay'] for row in pilot+exact)
    assert len({row['input_state_sha256'] for row in pilot})==len(pilot)
    proven=[row for row in exact if row['optimal_steps']>0]
    ratios=[row['depth']/row['optimal_steps'] for row in proven]
    paired=[(row,pol) for row,pol in zip(pilot,policies) if pol['E']==0]
    assert all(row['index']==pol['index'] for row,pol in paired)
    differences=[row['depth']-pol['T'] for row,pol in paired]
    exact_ok=len(proven)==len(exact) and float(np.median(ratios))<=1.5 and float(np.quantile(ratios,.9))<=2
    policy_ok=bool(paired) and np.mean(differences)<=0 and sum(d<=0 for d in differences)>=len(paired)/2
    distributions={key:dict(min=min(r[key] for r in pilot),max=max(r[key] for r in pilot),mean=float(np.mean([r[key] for r in pilot])))
                   for key in ('N','K','M','walls','E','depth','occupied','on_nests','distance_sum')}
    result=dict(teacher_eligible=bool(exact_ok and policy_ok),all_legal=True,all_teachers_complete=True,pilot_count=len(pilot),
        exact_proven=len(proven),exact_total=len(exact),depth_to_optimal_median=float(np.median(ratios)),depth_to_optimal_p90=float(np.quantile(ratios,.9)),
        exact_gate=bool(exact_ok),policy_gate=bool(policy_ok),policy_completed=len(paired),
        mean_teacher_minus_policy_T=float(np.mean(differences)) if paired else None,
        teacher_no_longer_count=sum(d<=0 for d in differences),full_inventory=sum(r['E']==r['M'] for r in pilot),
        singleton_starts=sum(r['singleton_start'] for r in pilot),distribution=distributions,
        height_totals=np.array([r['heights'] for r in pilot]).sum(0).tolist(),
        mean_restored_fraction=float(np.mean([r['E']/r['M'] for r in pilot])),completed_at=now())
    save(root/'pilot/result.json',result);print(json.dumps(result),flush=True);return result


def bulk(root):
    directory=root/'bulk';directory.mkdir(parents=True,exist_ok=True)
    if (directory/'result.json').exists():return load(directory/'result.json')
    inputs=prepare_inputs(root,'bulk',CONFIG['bulk'],10000);started=time.monotonic()
    def prepare(item):
        if time.monotonic()-started>CONFIG['bulk_seconds']:return None
        return one(root,'bulk',item,CONFIG['bulk_depth'])
    with ThreadPoolExecutor(max_workers=CONFIG['workers']) as pool:
        records=[r for r in pool.map(prepare,inputs) if r is not None]
    assert len({r['input_state_sha256'] for r in records})==len(records)
    save(directory/'result.json',dict(cases=records,frames=sum(r['depth'] for r in records),all_legal=True,all_complete=True,
                                    seconds=time.monotonic()-started,completed_at=now()))


def execute(root):
    root.mkdir(parents=True,exist_ok=True);lock=(root/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    paths=[ROOT/'adhoc/bin/generate_v093_reverse.cpp',ROOT/'adhoc/scripts/run_v093_reverse.py',ROOT/'adhoc/scripts/v089_core.cpp.txt',ROOT/'adhoc/scripts/v089_data.py']
    identity=dict(config=CONFIG,sources={str(p.relative_to(ROOT)):sha(p) for p in paths},
                  policy_checkpoint_sha256=sha(PPO/'initial/checkpoint.pt'),official_generator_sha256=sha(ROOT/'tools/target/release/gen'))
    if (root/'config.json').exists():assert load(root/'config.json')==identity
    else:
        save(root/'config.json',identity)
        for path in paths+[ROOT/'notes/experiments/v093.md']:
            target=root/'frozen'/path.relative_to(ROOT);target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,target)
        shutil.copy2(ROOT/'tools/target/release/gen',root/'generator')
        shutil.copy2(ROOT/'target/release/generate_v093_reverse',root/'reverse_generator')
    if not (root/'pilot/result.json').exists():
        status(root,'pilot_generation');inputs=prepare_inputs(root,'pilot',CONFIG['pilot'],0)
        with ThreadPoolExecutor(max_workers=CONFIG['workers']) as pool:
            pilot=list(pool.map(lambda item:one(root,'pilot',item,CONFIG['depths'][item['index']//16]),inputs))
        save(root/'pilot/cases.json',pilot);status(root,'exact_shortest');exact=exact_examples(root)
        # 固定入力の評価と同時に競合するCPU処理を始めない。
        assert (PPO/'evaluation/start/comparison.json').exists(),'v092 initial evaluation must finish first'
        status(root,'fixed_policy_comparison')
        with ThreadPoolExecutor(max_workers=CONFIG['workers']) as pool:policies=list(pool.map(lambda item:policy(root,item),pilot))
        result=quality(root,pilot,exact,policies)
    else:result=load(root/'pilot/result.json')
    if result['teacher_eligible']:status(root,'bulk_generation');bulk(root)
    status(root,'completed',teacher_eligible=result['teacher_eligible'])
    save(root/'exit.json',dict(exit_code=0,finished_at=now()))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args()
    try:execute(a.run.resolve())
    except BaseException as error:
        save(a.run/'exit.json',dict(exit_code=1,error=repr(error),finished_at=now()));traceback.print_exc();raise
