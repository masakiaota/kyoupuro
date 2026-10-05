#!/usr/bin/env python3
"""四つの探索乱数の平均から選んだ計画の利益を独立な四つで測る。"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import fcntl
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import time
import traceback

import numpy as np
from check_v089_board import compile_binary
from check_v098_complete import action_line
from v089_data import ROOT, Geometry, remaining, save, sha, status, now
from v090_data import RUN as BC_RUN
from v091_env import load
from v092_stream import excluded_inputs, normalized, digest, describe

RUN = ROOT/'results/nn_rank/v112/20261004_lns_mean_studio/diagnostic'
PARENT = ROOT/'src/bin/v401_incremental_cnn.cpp'
SEEDS = tuple(range(112001,112009))
CONFIG = dict(cases=32, symmetries=8, seeds=list(SEEDS), checkpoints=[.05,.20,1.20],
              workers=20, seed_base=1120000000000, seed_stride=64, order_seed=112003,
              bootstrap_seed=112004, bootstrap_draws=10000, collection_limit_seconds=900,
              checkpoint_sha256='e9d615587a1a655b7b2818e2f19564619050e2ffd669050b015b7c46766b90d3')


def prepare(root):
    frozen = root/'frozen'; frozen.mkdir(parents=True, exist_ok=True)
    text = PARENT.read_text(); parent_sha = sha(PARENT)
    assert parent_sha == 'ceb38ea758212dbbd91f8faa63eb6684fd96aab4b3b60a82ea718150538d4618'
    original = text[:text.index('\nint main() {')]
    observation = (ROOT/'adhoc/scripts/v112_observe.cpp.txt').read_text()
    main = (ROOT/'adhoc/scripts/v112_main.cpp.txt').read_text()
    needle = '        while(time_keeper.exact_elapsed_sec()<min(slice_end,end)) {\n'
    call = '            v112_observe(best,attempts,accepted,improvements);\n'
    assert original.count(needle) == 1
    changed = original.replace('class TemporalLNS {', observation+'\nclass TemporalLNS {', 1)
    changed = changed.replace(needle, needle+call, 1)
    assert changed.replace(observation+'\n','',1).replace(call,'',1) == original
    content = '// v112_lns_labels.cpp\n'+changed.split('\n',1)[1]+'\n'+main
    source = ROOT/'adhoc/bin/v112_lns_labels.cpp'
    if source.exists(): assert source.read_text() == content
    else: source.write_text(content)
    inputs = [Path(__file__), ROOT/'adhoc/scripts/v112_observe.cpp.txt', ROOT/'adhoc/scripts/v112_main.cpp.txt', source,
              PARENT, ROOT/'scripts/build_solver.sh', ROOT/'adhoc/scripts/v089_data.py']
    identity = dict(config=CONFIG, parent_sha256=parent_sha, source_sha256=sha(source),
                    sources={str(p.relative_to(ROOT)): sha(p) for p in inputs})
    marker = root/'config.json'
    if marker.exists(): assert load(marker) == identity
    else:
        save(marker, identity)
        for path in inputs+[ROOT/'notes/experiments/v112.md']:
            target = frozen/path.relative_to(ROOT); target.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(path,target)
    binaries = []
    for local in (True, False):
        destination = root/('helper_local' if local else 'helper_judge'); built = destination.with_suffix('.json')
        if built.exists():
            record = load(built); assert record['source_sha256'] == sha(source) and record['binary_sha256'] == sha(destination)
        else:
            compile_binary(source.stem,destination,local)
            save(built,dict(source_sha256=sha(source),binary_sha256=sha(destination),local=local,built_at=now()))
        binaries.append(destination)
    packed = text.split('static constexpr char nn_packed[] =',1)[1].split(';',1)[0]
    assert packed == content.split('static constexpr char nn_packed[] =',1)[1].split(';',1)[0]
    save(root/'source_check.json',dict(passed=True, parent_sha256=parent_sha, source_sha256=sha(source),
                                     original_body_except_observer_identical=True,
                                     packed_sha256=hashlib.sha256(packed.encode()).hexdigest(), both_modes_built=True))
    return binaries


def invoke(binary, mode, payload):
    tick = time.monotonic()
    proc = subprocess.run([binary,mode],input=payload,text=True,capture_output=True,timeout=60)
    if proc.returncode: raise RuntimeError(f'{mode} failed: {proc.stderr}')
    return json.loads(proc.stdout), time.monotonic()-tick


def verify(geo, row):
    state = geo.initial.copy()
    for code in row['actions']: geo.apply(state,action_line(code))
    assert remaining(state) == row['E'] and len(row['actions']) == row['T']


def generate(root, case, binary):
    directory = root/'cases'/f"{case['index']:04d}"; directory.mkdir(parents=True,exist_ok=True)
    marker = directory/'plans.json'
    if marker.exists(): return load(marker)
    path = root/case['path']; assert sha(path) == case['sha256']
    rows, elapsed = invoke(binary,'generate',path.read_text()); assert [r['sym'] for r in rows] == list(range(8))
    geo = Geometry(path); seen = {}; plans = []
    for row in rows:
        verify(geo,row)
        fingerprint = digest(' '.join(map(str,row['actions']))+'\n')
        duplicate = seen.get(fingerprint)
        if duplicate is None and row['E'] == 0: seen[fingerprint] = row['sym']
        plans.append(dict(row,plan_sha256=fingerprint,duplicate_of=duplicate))
    result = dict(case=case['index'], input_sha256=case['sha256'], plans=plans, wall_seconds=elapsed, completed_at=now())
    save(marker,result); return result


def grow(root, case, plan, seed, binary):
    directory = root/'cases'/f"{case['index']:04d}"/'labels'; directory.mkdir(exist_ok=True)
    marker = directory/f"{plan['sym']}_{seed}.json"
    if marker.exists(): return load(marker)
    path = root/case['path']; assert sha(path) == case['sha256']
    payload = path.read_text()+f"{plan['T']} {seed}\n"+' '.join(map(str,plan['actions']))+'\n'
    result, elapsed = invoke(binary,'grow',payload)
    assert result['seed'] == seed and result['initial_T'] == plan['T'] and result['state_pool_free'] == 4
    assert result['invalid_candidates'] == 0 and result['attempts'] > 0
    assert [r['target'] for r in result['samples']] == CONFIG['checkpoints']
    geo = Geometry(path); previous = plan['T']
    for row in result['samples']:
        verify(geo,row); assert row['E'] == 0 and row['T'] <= previous and row['elapsed'] >= row['target']-1e-5
        previous = row['T']; row['plan_sha256'] = digest(' '.join(map(str,row['actions']))+'\n')
    result.update(case=case['index'],sym=plan['sym'],input_sha256=case['sha256'],initial_plan_sha256=plan['plan_sha256'],
                  process_seconds=elapsed,completed_at=now())
    save(marker,result); return result


def mechanism(root, binaries):
    directory = root/'mechanism'; directory.mkdir(exist_ok=True)
    marker = directory/'result.json'
    if marker.exists(): return load(marker)
    cases = [case for case in load(BC_RUN/'input_manifest.json') if case['role'] == 'train'][:2]
    local, judge = binaries; plans_checked = samples_checked = 0
    for i, original in enumerate(cases):
        path = directory/f'{i}.txt'; shutil.copy2(BC_RUN/original['path'],path)
        case = dict(index=i,path=str(path.relative_to(directory)),sha256=sha(path))
        generated = generate(directory,case,local)
        other, _ = invoke(judge,'generate',path.read_text())
        assert all(a['actions'] == b['actions'] and a['E'] == b['E'] for a,b in zip(generated['plans'],other))
        plans_checked += len(other)
        eligible = [p for p in generated['plans'] if p['E'] == 0 and p['duplicate_of'] is None]
        assert eligible, 'no complete mechanism plan'
        for seed in SEEDS:
            row = grow(directory,case,eligible[0],seed,local); samples_checked += len(row['samples'])
    result = dict(passed=True,plans_checked=plans_checked,lns_samples_checked=samples_checked,
                  local_judge_generation_identical=True,independent_replay=True,completed_at=now())
    save(marker,result); return result


def inputs(root):
    marker = root/'input_manifest.json'
    if marker.exists(): return load(marker)
    blocked = excluded_inputs()
    for path in (ROOT/'tools/validation1').glob('*.txt'):
        text = path.read_text(); blocked.update((digest(text),digest(normalized(text))))
    listing = subprocess.run(['rg','--files','--hidden','--no-ignore','results/nn_rank','-g','generated.jsonl'],
                             cwd=ROOT,capture_output=True,text=True,check=True).stdout.splitlines()
    records = []
    for name in listing:
        path = ROOT/name; count = 0
        with path.open() as stream:
            for line in stream:
                if not line.endswith('\n'): continue  # 他の学習が追記している最後の未完行は次回の記録に属する。
                row = json.loads(line)
                if 'sha256' in row: blocked.add(row['sha256']); count += 1
        records.append(dict(path=name,records=count))
    save(root/'excluded_sha256.json',sorted(blocked)); save(root/'excluded_sources.json',dict(records=records,snapshot_at=now()))
    generator = root/'official_gen'; shutil.copy2(ROOT/'tools/target/release/gen',generator)
    save(root/'generator.json',dict(sha256=sha(generator)))
    directory = root/'inputs'; directory.mkdir(exist_ok=True); manifest=[]
    temporary = root/'generation'; temporary.mkdir(exist_ok=True)
    for index in range(CONFIG['cases']):
        for offset in range(CONFIG['seed_stride']):
            seed=CONFIG['seed_base']+index*CONFIG['seed_stride']+offset
            (temporary/'seed.txt').write_text(str(seed)+'\n')
            subprocess.run([generator,temporary/'seed.txt','--dir',temporary/'output'],check=True,capture_output=True,timeout=30)
            text=normalized((temporary/'output/0000.txt').read_text()); fingerprint=digest(text)
            if fingerprint in blocked:continue
            blocked.add(fingerprint); path=directory/f'{index:04d}.txt';path.write_text(text)
            _, statistics=describe(text)
            manifest.append(dict(index=index,seed=seed,path=str(path.relative_to(root)),sha256=fingerprint,**statistics));break
        else: raise RuntimeError('seed range exhausted')
    save(marker,manifest);return manifest


def analyse(root, cases):
    per_case=[]; pair_hits=[]; pair_boards=set(); centered=[[],[]]; coverage=[]
    generation_seconds=0.; max_overshoot=0.; lns_seconds=0.; count_labels=0
    for case in cases:
        directory=root/'cases'/f"{case['index']:04d}"; generated=load(directory/'plans.json')
        generation_seconds+=generated['wall_seconds']
        plans=[p for p in generated['plans'] if p['E']==0 and p['duplicate_of'] is None]
        coverage.append(dict(case=case['index'],completed=sum(p['E']==0 for p in generated['plans']),unique=len(plans)))
        labels=[[load(directory/'labels'/f"{p['sym']}_{seed}.json") for p in plans] for seed in SEEDS]
        for group in labels:
            for label in group:
                count_labels+=1;lns_seconds+=label['samples'][-1]['elapsed']
                max_overshoot=max(max_overshoot,max(s['elapsed']-s['target'] for s in label['samples']))
        if len(plans)<4:continue
        raw=np.array([p['T'] for p in plans]);syms=np.array([p['sym'] for p in plans])
        costs=np.array([[[s['T'] for s in label['samples']] for label in group] for group in labels])
        individual_end=costs[:,:,2].copy()
        end=np.stack([individual_end[:4].mean(0),individual_end[4:].mean(0)])
        costs=np.stack([costs[:4].mean(0),costs[4:].mean(0)])
        def best(values, eligible=None):
            ids=list(range(len(plans))) if eligible is None else list(eligible)
            return min(ids,key=lambda i:(values[i],raw[i],syms[i]))
        initial=best(raw); oracle=[best(end[s]) for s in range(2)]
        early=[best(costs[s,:,1]) for s in range(2)]
        stages=[]
        for s in range(2):
            half=sorted(range(len(plans)),key=lambda i:(costs[s,i,0],raw[i],syms[i]))[:(len(plans)+1)//2]
            stages.append(best(costs[s,:,1],half))
        row=dict(case=case['index'],unique=len(plans),initial_best_sym=int(syms[initial]),
                 initial_shortest_final=float(end[:,initial].mean()),
                 oracle_final=float(np.mean([end[s,oracle[s]] for s in range(2)])),
                 transferred_final=float(np.mean([end[1-s,oracle[s]] for s in range(2)])),
                 short_pilot_final=float(np.mean([end[s,early[s]] for s in range(2)])),
                 two_stage_final=float(np.mean([end[s,stages[s]] for s in range(2)])),
                 oracle_syms=[int(syms[i]) for i in oracle],
                 final_by_group=end.tolist(),final_by_seed=individual_end.tolist(),symmetries=syms.tolist(),raw_lengths=raw.tolist())
        row['headroom']=row['initial_shortest_final']-row['oracle_final']
        row['transfer_gain']=row['initial_shortest_final']-row['transferred_final']
        row['pilot_gain']=row['initial_shortest_final']-row['short_pilot_final']
        row['two_stage_gain']=row['initial_shortest_final']-row['two_stage_final']
        per_case.append(row)
        for s in range(2):centered[s].extend((end[s]-end[s].mean()).tolist())
        for s in range(2):
            for a in range(len(plans)):
                for b in range(a+1,len(plans)):
                    delta=end[s,a]-end[s,b]
                    if abs(delta)<3:continue
                    other=end[1-s,a]-end[1-s,b]
                    pair_hits.append(.5 if other==0 else float(delta*other>0));pair_boards.add(case['index'])
    rng=np.random.default_rng(CONFIG['bootstrap_seed']); intervals={}
    for key in ('headroom','transfer_gain','pilot_gain','two_stage_gain'):
        values=np.array([r[key] for r in per_case])
        if len(values):
            draws=values[rng.integers(len(values),size=(CONFIG['bootstrap_draws'],len(values)))].mean(1)
            intervals[key]=dict(mean=float(values.mean()),ci95=np.quantile(draws,[.025,.975]).tolist())
    agreement=float(np.mean(pair_hits)) if pair_hits else None
    correlation=float(np.corrcoef(centered)[0,1]) if all(np.std(x)>0 for x in centered) else None
    passed=(len(per_case)>=24 and intervals['headroom']['mean']>=2 and intervals['transfer_gain']['mean']>=1
            and len(pair_hits)>=64 and len(pair_boards)>=16 and agreement>=.60)
    result=dict(promising_teacher=passed,cases=len(cases),eligible_cases=len(per_case),coverage=coverage,
                intervals=intervals,pair_agreement=agreement,pairs=len(pair_hits),pair_boards=len(pair_boards),
                centered_candidate_correlation=correlation,labels=count_labels,all_legal=True,
                total_generation_wall_seconds=generation_seconds,total_lns_elapsed_seconds=lns_seconds,
                maximum_checkpoint_overshoot_seconds=max_overshoot,per_case=per_case,
                training_performed=False,submission_evaluation_performed=False,completed_at=now())
    save(root/'result.json',result);return result


def execute(root):
    root.mkdir(parents=True,exist_ok=True)
    lock=(root/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (root/'result.json').exists():return
    started=time.monotonic()
    try:
        status(root,'building'); binaries=prepare(root)
        status(root,'mechanism');mechanism(root,binaries)
        status(root,'generating_inputs');cases=inputs(root)
        data_started=time.monotonic(); order_rng=np.random.default_rng(CONFIG['order_seed']);order=order_rng.permutation(len(cases))
        with ThreadPoolExecutor(max_workers=CONFIG['workers']) as pool:
            futures=[pool.submit(generate,root,cases[int(i)],binaries[0]) for i in order]
            for n,future in enumerate(as_completed(futures),1):
                future.result();status(root,'generating_plans',completed=n,total=len(cases),seconds=time.monotonic()-started)
        jobs=[]
        for case in cases:
            plans=load(root/'cases'/f"{case['index']:04d}"/'plans.json')['plans']
            jobs.extend((case,plan,seed) for plan in plans if plan['E']==0 and plan['duplicate_of'] is None for seed in SEEDS)
        order=order_rng.permutation(len(jobs))
        save(root/'job_order.json',[dict(case=jobs[int(i)][0]['index'],sym=jobs[int(i)][1]['sym'],seed=jobs[int(i)][2]) for i in order])
        with ThreadPoolExecutor(max_workers=CONFIG['workers']) as pool:
            pending={};next_job=done=0
            while done<len(jobs):
                if time.monotonic()-data_started>=CONFIG['collection_limit_seconds']:raise TimeoutError('registered collection budget')
                while len(pending)<CONFIG['workers'] and next_job<len(order):
                    index=int(order[next_job]);next_job+=1
                    pending[pool.submit(grow,root,*jobs[index],binaries[0])]=index
                finished=next(as_completed(pending));finished.result();del pending[finished];done+=1
                status(root,'growing_plans',completed=done,total=len(jobs),seconds=time.monotonic()-started)
        status(root,'analysing');result=analyse(root,cases)
        save(root/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,completed_at=now()))
        status(root,'completed',promising_teacher=result['promising_teacher'],seconds=time.monotonic()-started)
    except BaseException as error:
        save(root/'exit.json',dict(exit_code=1,error=repr(error),traceback=traceback.format_exc(),completed_at=now()))
        status(root,'failed',error=repr(error));raise


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);args=p.parse_args();execute(args.run.resolve())
