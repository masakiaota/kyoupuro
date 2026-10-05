#!/usr/bin/env python3
"""凍結した3条件を、新規200入力・Studio20並列で各1回比較する。"""
import argparse
from datetime import datetime
import json
from pathlib import Path
import shutil
import subprocess
import sys

import numpy as np
from run_v079_experiment import command, local_log, replay
from run_v077_overnight import input_properties, prior_inputs
from v086_data import ROOT, save, sha, status

BINS=('v076_relative_tuned','v079_nn_immediate','v086_nn_proposal')


def prepare(run):
    run.mkdir(parents=True,exist_ok=False);(run/'inputs').mkdir();(run/'frozen').mkdir()
    seeds,hashes=prior_inputs(run)
    # 初期の大規模生成はJSONL正本も持つため、JSONの入力一覧に加えて除外する。
    for path in (ROOT/'results/nn_rank/v077/20261002T021222_studio').glob('*.jsonl'):
        if path.name not in ('inputs.jsonl','generated.jsonl'):continue
        for line in path.read_text().splitlines():
            row=json.loads(line);seeds.add(row['seed']);hashes.add(row['sha256'])
    save(run/'exclusions.json',{'seeds':sorted(seeds),'hashes':sorted(hashes)})
    index=970000000;manifest=[]
    while len(manifest)<200:
        batch=[]
        while len(batch)<200-len(manifest):
            if index not in seeds:batch.append(index)
            index+=1
        seedfile=run/f'seeds_{batch[0]}.txt';seedfile.write_text(''.join(f'{s}\n' for s in batch))
        folder=run/f'generated_{batch[0]}';folder.mkdir()
        command([ROOT/'tools/target/release/gen',seedfile,'--dir',folder],run/f'gen_{batch[0]}.log')
        for i,seed in enumerate(batch):
            source=folder/f'{i:04d}.txt';digest=sha(source);seeds.add(seed)
            if digest in hashes:continue
            hashes.add(digest);name=f'{len(manifest):04d}.txt';shutil.copy2(source,run/'inputs'/name)
            manifest.append({'case':name,'seed':seed,'sha256':digest,**input_properties(source.read_bytes())})
    save(run/'input_manifest.json',manifest)
    files=[ROOT/f'src/bin/{n}.cpp' for n in BINS]+[Path(__file__).resolve(),ROOT/'scripts/eval.py',ROOT/'scripts/build_solver.sh']
    frozen={str(p.relative_to(ROOT)):sha(p) for p in files}
    for path in files:shutil.copy2(path,run/'frozen'/path.name)
    save(run/'config.json',{'sources':frozen,'bins':BINS,'jobs':20,'cases':200,'created_at':datetime.now().isoformat()})
    # 提出ビルドは直前に--no-localで作った物を保存する。LOCALはeval.pyが再構築する。
    shutil.copy2(ROOT/'target/release/v086_nn_proposal',run/'frozen/v086_nn_proposal_nonlocal')
    status(run,'prepared')


def execute(run):
    with (run/'started.json').open('x') as out:json.dump({'started_at':datetime.now().isoformat()},out)
    config=json.loads((run/'config.json').read_text())
    for path,digest in config['sources'].items():assert sha(ROOT/path)==digest
    checks=ROOT/'results/analysis/v086/integration_checks'
    for mode in ('local','nonlocal'):
        assert json.loads((checks/f'{mode}.json').read_text())['passed']
        shutil.copy2(checks/f'{mode}.json',run/f'inference_{mode}.json')
    manifest=json.loads((run/'input_manifest.json').read_text());allrecords={}
    for name in BINS:
        status(run,'evaluating',solver=name,cases=200,jobs=20)
        out=ROOT/'results/out'/name
        if out.exists():
            previous=run/'previous_out'/name;previous.parent.mkdir(exist_ok=True);shutil.move(out,previous)
        label=f'{run.name}_v086_{name}'
        command([sys.executable,ROOT/'scripts/eval.py',name,run/'inputs','-j','20','--label',label],run/f'eval_{name}.log')
        archive=run/'outputs'/name;archive.parent.mkdir(exist_ok=True);shutil.copytree(out,archive)
        records=[]
        with (ROOT/'results/eval_records.jsonl').open() as stream:
            for line in stream:
                row=json.loads(line)
                if row['label']==label:records.append(row)
        assert len(records)==len({r['case_name'] for r in records})==200
        assert len({r['run_id'] for r in records})==1
        save(run/f'records_{name}.json',records);allrecords[name]={r['case_name']:r for r in records}
        shutil.copy2(ROOT/'target/release'/name,run/'frozen'/f'{name}_local')
    cases=[]
    for item in manifest:
        versions={}
        for name in BINS:
            record=allrecords[name][item['case']]
            assert record['status']=='ok' and record['local'] and record['bin']==name
            output=run/'outputs'/name/item['case'];T=replay(run/'inputs'/item['case'],output)
            assert T==record['score']
            versions[name]={'T':T,'elapsed_ms':record['elapsed'],**local_log(output.with_suffix('.txt.err'),T)}
        cases.append({**item,'versions':versions})
    summaries={}
    for name in BINS:
        rows=[c['versions'][name] for c in cases];countkeys={k for r in rows for k in r['counts']}
        timekeys={k for r in rows for k in r['times_ms']}
        summaries[name]={'total_T':sum(r['T'] for r in rows),'mean_T':np.mean([r['T'] for r in rows]).item(),
                         'mean_elapsed_ms':np.mean([r['elapsed_ms'] for r in rows]).item(),
                         'max_elapsed_ms':max(r['elapsed_ms'] for r in rows),
                         'counts':{k:sum(r['counts'].get(k,0) for r in rows) for k in countkeys},
                         'mean_times_ms':{k:sum(r['times_ms'].get(k,0) for r in rows)/200 for k in timekeys}}
    differences={}
    for base,new in ((BINS[0],BINS[1]),(BINS[0],BINS[2]),(BINS[1],BINS[2])):
        saved=np.array([c['versions'][base]['T']-c['versions'][new]['T'] for c in cases])
        draws=np.random.default_rng(86002).integers(0,200,(6000,200));interval=np.percentile(saved[draws].mean(1),[2.5,97.5])
        differences[f'{new}_vs_{base}']={'total_saved':int(saved.sum()),'mean_saved':float(saved.mean()),'bootstrap95':interval.tolist(),
                                       'wins':int((saved>0).sum()),'draws':int((saved==0).sum()),'losses':int((saved<0).sum())}
    counts=summaries[BINS[2]]['counts'];mechanism=counts['nn086_proposals']>0 and counts['nn086_rebuilt']>0 and counts['nn086_outside_old_pool']>0
    effect=differences[f'{BINS[2]}_vs_{BINS[1]}'];time_ratio=summaries[BINS[2]]['mean_elapsed_ms']/summaries[BINS[1]]['mean_elapsed_ms']
    result={'cases':200,'versions':summaries,'differences':differences,'all_legal_E0':True,'mechanism_passed':mechanism,
            'mean_time_ratio_vs_v079':time_ratio,'adopt':bool(mechanism and effect['mean_saved']>0 and effect['bootstrap95'][0]>0 and time_ratio<=1.1),
            'five_move_target':effect['mean_saved']>=5,'finished_at':datetime.now().isoformat()}
    save(run/'case_results.json',cases);save(run/'comparison.json',result)
    status(run,'completed',differences=differences,adopt=result['adopt'])
    print(json.dumps({k:v for k,v in result.items() if k!='versions'}),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('mode',choices=('prepare','execute'));p.add_argument('--run',type=Path,required=True)
    a=p.parse_args();{'prepare':prepare,'execute':execute}[a.mode](a.run.resolve())
