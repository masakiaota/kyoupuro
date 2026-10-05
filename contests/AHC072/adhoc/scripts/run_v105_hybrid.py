#!/usr/bin/env python3
"""固定した3条件をCPUで検査し、同じ検証入力を各1回だけ評価する。"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

import numpy as np
import torch

from assess_v102_ablation import numerical, compare
from check_v089_board import compile_binary, trace
from run_v089_evaluation import validated_output, statistics
from v089_data import ROOT, save, sha, now, status
from v090_data import Dataset
from v099_complete import BC_RUN


def load(path):
    return json.loads(path.read_text())


def move_hash(output):
    value=1469598103934665603
    for line in output.splitlines():
        if not line.strip():continue
        i,j,k,d,l=line.split();i,j,k,l=map(int,(i,j,k,l))
        code=(((i*20+j)*8+k)*4+'UDLR'.index(d))*8+l-1
        value=((value^code)*1099511628211)&((1<<64)-1)
    return value&((1<<63)-1)


def frozen(root):
    config=load(root/'config.json')
    assert sha(root/'training/model.json')==config['model_sha256']
    assert sha(root/'input_manifest.json')==config['input_sha256']
    for item in config['sources'].values():assert sha(ROOT/item['path'])==item['sha256']
    return config


def preprocess(source,destination,local):
    env=os.environ.copy()
    if sys.platform=='darwin':
        env.setdefault('MACOSX_DEPLOYMENT_TARGET','15.0')
        env.setdefault('SDKROOT',subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip())
    args=[env.get('CXX','g++-15'),'-std=gnu++23','-O2','-march=native','-pthread','-fopenmp','-E','-P']
    args+=['-DLOCAL'] if local else ['-DATCODER','-DONLINE_JUDGE','-DNOMINMAX']
    with destination.open('w') as out:subprocess.run(args+[str(source)],env=env,stdout=out,check=True)
    return destination.read_text()


def check(root):
    marker=root/'mechanism/result.json'
    if marker.exists():assert load(marker)['passed'];return load(marker)
    config=frozen(root);where=marker.parent;where.mkdir(parents=True,exist_ok=True)
    sources={key:ROOT/item['path'] for key,item in config['sources'].items()}
    status(root,'checking_numerics',device='cpu')
    numeric=numerical(root,sources['nn'])
    assert sha(sources['nn'])==config['sources']['nn']['sha256']
    checker=ROOT/'adhoc/bin/check_v105_hybrid.cpp'
    checker.write_text('''// check_v105_hybrid.cpp
#define V090_DIAGNOSTIC
#define main v105_submission_main
#include "../../src/bin/v105_nn_lns.cpp"
#undef main
int main(int argc,char** argv) {
    if(argc!=2)return 2;
    nn::load_nn();nn::Input input;input.read();const nn::Board board(input);
    nn::BoardState state(input,board);std::ifstream snapshot(argv[1]);
    nn::read_snapshot(snapshot,board,state);nn::dump_inference(input,board,state);
}
''')
    for local in (True,False):
        mode='local' if local else 'judge'
        compile_binary(checker.stem,where/f'checker_{mode}',local)
        for key,source in sources.items():compile_binary(source.stem,where/f'{key}_{mode}',local)
    # 対照の完全なマクロ展開を照合する。診断用のファイル名だけを正規化する。
    preprocessing={}
    for local in (True,False):
        mode='local' if local else 'judge'
        before=preprocess(root/'frozen/v214_original.cpp',where/f'parent_{mode}.ii',local)
        after=preprocess(sources['classical'],where/f'classical_{mode}.ii',local)
        normalize=lambda text:re.sub(r'"[^"\n]*(?:v214_original|v105_classical)\.cpp"','"solver.cpp"',text)
        assert normalize(before)==normalize(after),'traditional preprocessing mismatch'
        preprocess(sources['hybrid'],where/f'hybrid_{mode}.ii',local)
        preprocessing[mode]=dict(classical_identical=True,hybrid_compiled=True)
    original=(root/'frozen/v214_original.cpp').read_text();hybrid=sources['hybrid'].read_text()
    algorithm=original[original.index('class PortionRouter'):original.index('struct InitialSolutions')]
    assert algorithm in hybrid,'LNS or shortening implementation changed'
    data=Dataset(BC_RUN/'data');cases=[c for c in data.cases if c['role']=='train'][:4]
    numeric_errors={key:0. for key in ('x','features','logits','value')};executions=[]
    for case in cases:
        fid=case['frame_start']+case['frames']//2
        snapshot=where/f"{case['index']:06d}.state"
        snapshot.write_text(' '.join(map(str,data.states[fid]))+'\n')
        input_path=BC_RUN/case['path'];inp=input_path.read_text()
        for mode in ('local','judge'):
            expected=json.loads(subprocess.check_output([root/'numerical'/f'checker_{mode}',snapshot,'dump'],input=inp,text=True))
            actual=json.loads(subprocess.check_output([where/f'checker_{mode}',snapshot],input=inp,text=True))
            assert actual['codes']==expected['codes']
            for key in numeric_errors:
                delta=float(np.max(np.abs(np.asarray(actual[key])-np.asarray(expected[key]))))
                numeric_errors[key]=max(numeric_errors[key],delta)
            reference=None
            for key in ('nn','hybrid'):
                started=time.monotonic()
                proc=subprocess.run([where/f'{key}_{mode}'],input=inp,text=True,capture_output=True,check=True,timeout=5)
                tag=f"{case['index']:06d}_{key}_{mode}"
                (where/f'{tag}.txt').write_text(proc.stdout);(where/f'{tag}.err').write_text(proc.stderr)
                result=validated_output(input_path,proc.stdout);assert result['E']==0
                counts=trace(proc.stderr)
                if key=='nn':reference=dict(result,hash=move_hash(proc.stdout))
                else:
                    assert result['T']<=reference['T']
                    if mode=='local':
                        assert counts['nn_initial_ops']==reference['T']
                        assert counts['nn_initial_hash']==reference['hash']
                        assert counts['lns_attempts']>0
                        assert counts['state_pool_free_at_end']==counts['state_slots']==4
                        assert all(counts.get(k,0)==0 for k in ('lns_errors','construction_errors','baseline_recovery','final_recovery','lns_invalid_candidates'))
                executions.append(dict(case=case['index'],condition=key,mode=mode,seconds=time.monotonic()-started,**result))
    assert numeric_errors['x']<2e-6 and numeric_errors['features']<2e-6
    assert numeric_errors['logits']<.003 and numeric_errors['value']<.03,numeric_errors
    result=dict(passed=True,numerical=numeric,hybrid_numeric_errors=numeric_errors,preprocessing=preprocessing,
                lns_source_identical=True,executions=executions,completed_at=now())
    frozen(root);save(marker,result);return result


def evaluate(root,condition):
    config=frozen(root);directory=root/'evaluation'/condition;directory.mkdir(parents=True,exist_ok=True)
    marker=directory/'result.json'
    if marker.exists():return load(marker)
    source=ROOT/config['sources'][condition]['path'];cases=load(root/'input_manifest.json')
    label=f'v105_165_{condition}_studio_j12';scratch=ROOT/'results/out'/source.stem
    started=directory/'started.json';exit_path=directory/'exit.json'
    if not started.exists():
        assert not scratch.exists(),f'refuse to overwrite {scratch}'
        save(started,dict(label=label,source_sha256=sha(source),jobs=12,started_at=now()))
        command=[sys.executable,ROOT/'scripts/eval.py',source.stem,root/'inputs','-j','12','--wait-lock','--label',label]
        status(root,'evaluating',condition=condition,cases=256,jobs=12)
        with (directory/'eval.log').open('w') as out:
            proc=subprocess.run(command,cwd=ROOT,stdout=out,stderr=subprocess.STDOUT)
        save(exit_path,dict(exit_code=proc.returncode,finished_at=now()))
    assert exit_path.exists(),'evaluation interrupted: inspect saved official records before resuming'
    assert load(exit_path)['exit_code']==0,f'evaluation failed: {directory}'
    records=[]
    with (ROOT/'results/eval_records.jsonl').open() as stream:
        for line in stream:
            row=json.loads(line)
            if row['label']==label:records.append(row)
    assert len(records)==len({r['case_name'] for r in records})==256
    assert len({r['run_id'] for r in records})==1
    save(directory/'official_records.json',records)
    outputs=directory/'outputs'
    if not outputs.exists():shutil.copytree(scratch,outputs)
    indexed={r['case_name']:r for r in records};rows=[]
    for case in cases:
        filename=case['filename'];record=indexed[filename];assert record['status']=='ok' and record['local']
        output=(outputs/filename).read_text();result=validated_output(root/'inputs'/filename,output)
        assert result['S']==record['score']
        counts=trace((outputs/(filename+'.err')).read_text())
        for key in ('inferences','depth','deadline'):counts.setdefault(key,0)
        rows.append(dict(case=case['index'],filename=filename,elapsed_ms=record['elapsed'],trace=counts,
                         path_hash=move_hash(output),**result))
    result=dict(condition=condition,metrics=statistics(rows),rows=rows,all_legal=True,
                solver_sha256=sha(source),completed_at=now())
    save(marker,result);return result


def run(root):
    config=frozen(root);save(root/'pipeline/status.json',dict(stage='checking',pid=os.getpid(),updated_at=now()))
    check(root)
    results={key:evaluate(root,key) for key in ('nn','classical','hybrid')}
    nn={r['case']:r for r in results['nn']['rows']};mechanism=[]
    for row in results['hybrid']['rows']:
        c=row['trace'];base=nn[row['case']]
        mechanism.append(dict(case=row['case'],same_initial=c.get('nn_initial_hash')==base['path_hash'],
            nn_complete=c.get('nn_complete')==1,lns_attempts=c.get('lns_attempts',0),
            saved_from_nn=c['nn_initial_ops']-row['T'],state_returned=c.get('state_pool_free_at_end')==4,
            errors=sum(c.get(k,0) for k in ('construction_errors','lns_errors','baseline_recovery','final_recovery','lns_invalid_candidates'))))
    mechanism_pass=all(r['same_initial'] and r['nn_complete'] and r['lns_attempts']>0 and r['state_returned']
                       and r['errors']==0 and r['saved_from_nn']>=0 for r in mechanism)
    differences={key:compare(results['hybrid'],results[key]) for key in ('classical','nn')}
    legal=all(r['all_legal'] for r in results.values())
    # 対照側も不正候補や作り直しを使っていないことを採否条件へ含める。
    structural_success=all(row['trace'].get('state_pool_free_at_end')==4 and
        all(row['trace'].get(k,0)==0 for k in ('construction_errors','lns_errors','baseline_recovery','final_recovery','lns_invalid_candidates'))
        for condition in ('classical','hybrid') for row in results[condition]['rows'])
    quality=legal and mechanism_pass and structural_success and all(r['metrics']['complete']==256 and r['metrics']['max_elapsed_ms']<=2000 for r in results.values())
    adopted=quality and all(d['mean_S_difference_all']<0 and d['mean_T_difference_on_common']<0 for d in differences.values())
    result=dict(adopted=adopted,quality_gate=quality,mechanism_pass=mechanism_pass,
        metrics={k:r['metrics'] for k,r in results.items()},comparisons=differences,mechanism=mechanism,
        conditions=config,completed_at=now())
    save(root/'result.json',result);status(root,'completed',adopted=adopted)
    save(root/'pipeline/status.json',dict(stage='completed',updated_at=now()))
    print(json.dumps({k:result[k] for k in ('adopted','quality_gate','metrics','comparisons')},ensure_ascii=False),flush=True)


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--run',type=Path,required=True);args=parser.parse_args()
    root=args.run.resolve();torch.set_num_threads(2);torch.set_num_interop_threads(2)
    try:run(root)
    except BaseException as error:
        save(root/'pipeline/exit.json',dict(exit_code=1,error=repr(error),finished_at=now()));raise
    else:save(root/'pipeline/exit.json',dict(exit_code=0,finished_at=now()))
