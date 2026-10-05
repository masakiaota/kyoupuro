#!/usr/bin/env python3
"""事前登録した最終600周の重みだけを、v079と新規200入力で比較する。"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys

import numpy as np
from build_v086_proposal import build
from run_v077_overnight import input_properties, prior_inputs
from run_v079_experiment import command, local_log, replay
from v086_data import ROOT, save, sha, status, now

BINS=('v079_nn_immediate','v086_nn_proposal','v088_nn_proposal')


def evaluate(training):
    model=training/'model_epoch600.json';result=json.loads((training/'result_epoch600.json').read_text())
    assert result['final']['epoch']==600 and json.loads(model.read_text())['epochs']==600
    run=training/'evaluation';run.mkdir(exist_ok=False);(run/'frozen').mkdir();(run/'inputs').mkdir()
    status(training,'cpp_evaluation_preparing')
    source=ROOT/f'src/bin/{BINS[2]}.cpp'
    if source.exists():raise FileExistsError(source)
    build(model,source)
    probe=ROOT/'adhoc/bin/check_v088_inference.cpp'
    if probe.exists():raise FileExistsError(probe)
    text=(ROOT/'adhoc/bin/check_v086_inference.cpp').read_text().replace('check_v086_inference','check_v088_inference').replace('v086_nn_proposal','v088_nn_proposal')
    probe.write_text(text)
    for mode in ('local','nonlocal'):
        args=['bash',ROOT/'scripts/build_solver.sh']+(['--no-local'] if mode=='nonlocal' else [])
        command(args+[BINS[2]],run/f'build_solver_{mode}.log')
        shutil.copy2(ROOT/'target/release'/BINS[2],run/'frozen'/f'{BINS[2]}_{mode}')
        command(args+['check_v088_inference'],run/f'build_probe_{mode}.log')
        command([sys.executable,ROOT/'adhoc/scripts/check_v086_inference.py','--binary',ROOT/'target/release/check_v088_inference',
                 '--model',model,'--output',run/f'check_{mode}.json'],run/f'check_{mode}.log')
        assert json.loads((run/f'check_{mode}.json').read_text())['passed']
    seeds,hashes=prior_inputs(run)
    for path in (ROOT/'results/nn_rank/v077/20261002T021222_studio').glob('*.jsonl'):
        if path.name not in ('inputs.jsonl','generated.jsonl'):continue
        for line in path.read_text().splitlines():
            row=json.loads(line);seeds.add(row['seed']);hashes.add(row['sha256'])
    manifest=[];seed=980000000
    while len(manifest)<200:
        chosen=[]
        while len(chosen)<200-len(manifest):
            if seed not in seeds:chosen.append(seed)
            seed+=1
        seedfile=run/f'seeds_{chosen[0]}.txt';seedfile.write_text(''.join(f'{s}\n' for s in chosen))
        folder=run/f'generated_{chosen[0]}';folder.mkdir()
        command([ROOT/'tools/target/release/gen',seedfile,'--dir',folder],run/f'gen_{chosen[0]}.log')
        for i,s in enumerate(chosen):
            path=folder/f'{i:04d}.txt';digest=sha(path);seeds.add(s)
            if digest in hashes:continue
            hashes.add(digest);name=f'{len(manifest):04d}.txt';shutil.copy2(path,run/'inputs'/name)
            manifest.append({'case':name,'seed':s,'sha256':digest,**input_properties(path.read_bytes())})
    save(run/'input_manifest.json',manifest)
    frozen={f'src/bin/{n}.cpp':sha(ROOT/f'src/bin/{n}.cpp') for n in BINS}
    save(run/'config.json',{'source_sha256':frozen,'model_sha256':sha(model),'cases':200,'jobs':20,'created_at':now()})
    records={}
    for name in BINS:
        status(training,'cpp_evaluating',solver=name)
        for path,digest in frozen.items():assert sha(ROOT/path)==digest
        out=ROOT/'results/out'/name
        if out.exists():
            dest=run/'previous_out'/name;dest.parent.mkdir(exist_ok=True);shutil.move(out,dest)
        label=f'{training.name}_v088_{name}'
        command([sys.executable,ROOT/'scripts/eval.py',name,run/'inputs','-j','20','--label',label],run/f'eval_{name}.log')
        archive=run/'outputs'/name;archive.parent.mkdir(exist_ok=True);shutil.copytree(out,archive)
        rows=[]
        with (ROOT/'results/eval_records.jsonl').open() as f:
            for line in f:
                record=json.loads(line)
                if record['label']==label:rows.append(record)
        assert len(rows)==len({r['case_name'] for r in rows})==200 and len({r['run_id'] for r in rows})==1
        save(run/f'records_{name}.json',rows);records[name]={r['case_name']:r for r in rows}
        shutil.copy2(ROOT/f'src/bin/{name}.cpp',run/'frozen'/f'{name}.cpp')
        shutil.copy2(ROOT/'target/release'/name,run/'frozen'/f'{name}_evaluated')
    cases=[]
    for item in manifest:
        versions={}
        for name in BINS:
            record=records[name][item['case']];assert record['status']=='ok' and record['local']
            output=run/'outputs'/name/item['case'];T=replay(run/'inputs'/item['case'],output);assert T==record['score']
            versions[name]={'T':T,'elapsed_ms':record['elapsed'],**local_log(output.with_suffix('.txt.err'),T)}
        cases.append({**item,'versions':versions,'saved':versions[BINS[0]]['T']-versions[BINS[2]]['T'],
                      'saved_vs_60':versions[BINS[1]]['T']-versions[BINS[2]]['T']})
    saved=np.array([c['saved'] for c in cases]);draws=np.random.default_rng(88002).integers(0,200,(6000,200))
    interval=np.percentile(saved[draws].mean(1),[2.5,97.5]);mean_times={n:float(np.mean([c['versions'][n]['elapsed_ms'] for c in cases])) for n in BINS}
    ratio=mean_times[BINS[2]]/mean_times[BINS[0]]
    keys={k for c in cases for k in c['versions'][BINS[2]]['counts'] if k.startswith('nn086')}
    counts={k:sum(c['versions'][BINS[2]]['counts'].get(k,0) for c in cases) for k in keys}
    mechanism=counts.get('nn086_proposals',0)>0 and counts.get('nn086_rebuilt',0)>0
    result={'cases':200,'mean_saved':float(saved.mean()),'total_saved':int(saved.sum()),'bootstrap95':interval.tolist(),
            'wins':int((saved>0).sum()),'draws':int((saved==0).sum()),'losses':int((saved<0).sum()),
            'mean_elapsed_ms':mean_times,'time_ratio':ratio,'mechanism_passed':mechanism,'mechanism_counts':counts,
            'all_legal_E0':True,'adopt':bool(saved.mean()>0 and interval[0]>0 and ratio<=1.1 and mechanism),
            'five_move_target':bool(saved.mean()>=5),'completed_at':now()}
    extended=np.array([c['saved_vs_60'] for c in cases])
    result['vs_same_architecture_60']={'mean_saved':float(extended.mean()),'bootstrap95':np.percentile(extended[draws].mean(1),[2.5,97.5]).tolist()}
    save(run/'case_results.json',cases);save(run/'comparison.json',result)
    status(training,'completed_with_evaluation',mean_saved=result['mean_saved'],adopt=result['adopt'])
    print(json.dumps(result),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);a=p.parse_args();evaluate(a.run.resolve())
