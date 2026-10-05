#!/usr/bin/env python3
"""数値・状態特徴の照合後、事前登録した開発用探索を一度ずつ比べる。"""
from concurrent.futures import ThreadPoolExecutor,as_completed
import json
from pathlib import Path
import re
import shutil
import subprocess
import time

import numpy as np
import torch

from build_v116_guidance import PARENT,block
from build_v121_guidance import build,checker
from check_v089_board import compile_binary
from run_v105_hybrid import preprocess
from train_v121_value import Model
from v116_data import ROOT,BC,save,load,sha,now
from v121_data import RUN


def without_guidance(text):
    for marker in ['namespace finite_value {','class FinitePlanner {']:
        if marker in text:text=text.replace(block(text,marker),'',1)
    text=re.sub(r'\s*trace.count_by\("finite_value_calls",\s*finite_value::calls\);','',text)
    text=re.sub(r'"[^"\n]*\.cpp"','"source.cpp"',text)
    text=re.sub(r'("source.cpp",\s*)\d+',r'\g<1>0',text)
    return re.sub(r'\s+',' ',text).strip()


def invoke(binary,mode,item,output=None):
    case=RUN/'data/cases'/f"{item['index']:06d}"
    args=[binary,mode,BC/item['path'],case/'problems.txt']
    if output is not None:args.append(output)
    proc=subprocess.run(args,cwd=ROOT,capture_output=True,text=True,timeout=120)
    assert proc.returncode==0,(item['index'],mode,proc.returncode,proc.stderr)
    return proc.stdout


def mechanism():
    where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True)
    marker=where/'cpp.json';source=ROOT/'adhoc/bin/v121_finite_value.cpp'
    if marker.exists():
        result=load(marker);assert result['source_sha256']==sha(source) and result['passed'];return result
    config=build(source,RUN/'training/model.json')
    zero_source=ROOT/'adhoc/bin/v121_zero_value.cpp';build(zero_source)
    checker(ROOT/'adhoc/bin/check_v121_finite.cpp',source)
    checker(ROOT/'adhoc/bin/check_v121_zero.cpp',zero_source)
    # The first LOCAL checkers were already built. Copy before a judge build replaces the target.
    for name,dest in [('check_v121_finite','checker_local'),('check_v121_zero','zero_local')]:
        shutil.copy2(ROOT/'target/release'/name,where/dest)
    compile_binary('check_v121_finite',where/'checker_judge',False)
    preprocessing={}
    for local in (True,False):
        mode='local' if local else 'judge'
        compile_binary(source.stem,where/('solver_'+mode),local)
        before=preprocess(PARENT,where/('parent_'+mode+'.ii'),local)
        after=preprocess(source,where/('learned_'+mode+'.ii'),local)
        assert without_guidance(before)==without_guidance(after),(mode,'unintended expanded change')
        # The proof and state codec are separately retained inside the excluded class.
        for token in ['int assess(','static FiniteState encode(','static void decode(']:
            assert block(before,token)==block(after,token),(mode,token)
        preprocessing[mode]=dict(outside_guidance_identical=True,proof_and_state_codec_identical=True,
                                  parent_sha256=sha(where/('parent_'+mode+'.ii')),learned_sha256=sha(where/('learned_'+mode+'.ii')))
    items=[x for x in load(RUN/'input_manifest.json') if x['guidance_role']=='train'][:4]
    zero_rows=[];feature_rows=0;feature_error=0.
    for item in items:
        raw=invoke(where/'zero_local','zero',item)
        (where/f"zero_{item['index']:06d}.jsonl").write_text(raw)
        zero_rows.extend(json.loads(x) for x in raw.splitlines())
        path=where/f"features_{item['index']:06d}.raw"
        invoke(where/'checker_local','features',item,path)
        actual=np.fromfile(path,dtype='<f4').reshape(-1,48)
        observed=np.fromfile(RUN/'data/cases'/f"{item['index']:06d}"/'samples.raw',dtype='<f4').reshape(-1,52)
        expected=observed[observed[:,51]==0,:48]
        assert actual.shape==expected.shape
        error=float(np.max(np.abs(actual-expected))) if len(actual) else 0.
        assert error==0,(item['index'],error)
        feature_error=max(feature_error,error);feature_rows+=len(actual)
    assert zero_rows and sum(r['learned_calls'] for r in zero_rows)>0
    arrays=[np.load(RUN/'data'/f'{role}.npy') for role in ['train','development']]
    batch=np.concatenate([a[np.linspace(0,len(a)-1,min(2048,len(a)),dtype=int),:48] for a in arrays])
    batch.astype('<f4').tofile(where/'features.raw')
    spec=load(RUN/'training/model.json');parameters={k:torch.tensor(v) for k,v in spec['parameters'].items()}
    cpu=Model(parameters['mean'],parameters['scale']);cpu.load_state_dict(parameters);cpu.eval()
    torch.set_num_threads(2);gpu=Model(parameters['mean'],parameters['scale']);gpu.load_state_dict(parameters);gpu.to('mps');gpu.eval()
    with torch.no_grad():
        expected=cpu(torch.from_numpy(batch)).numpy()
        accelerated=gpu(torch.from_numpy(batch).to('mps')).cpu().numpy()
    errors={'cpu_mps':float(np.max(np.abs(expected-accelerated)))}
    for mode in ['local','judge']:
        output=subprocess.check_output([where/('checker_'+mode),'predict',where/'features.raw'],text=True)
        (where/('predictions_'+mode+'.txt')).write_text(output)
        values=np.fromstring(output,sep='\n');assert len(values)==len(expected)
        errors['cpu_cpp_'+mode]=float(np.max(np.abs(expected-values)))
        errors['mps_cpp_'+mode]=float(np.max(np.abs(accelerated-values)))
    assert max(errors.values())<=1e-4,errors
    result=dict(passed=True,source_sha256=sha(source),model_sha256=sha(RUN/'training/model.json'),
                build=config,preprocessing=preprocessing,zero_windows=len(zero_rows),
                zero_calls=sum(r['learned_calls'] for r in zero_rows),feature_rows=feature_rows,
                feature_error=feature_error,numeric_rows=len(batch),numeric_max_errors=errors,completed_at=now())
    save(marker,result);return result


def development():
    where=RUN/'development';where.mkdir(exist_ok=True)
    source=ROOT/'adhoc/bin/v121_finite_value.cpp';binary=RUN/'mechanism/checker_local'
    identity=dict(source_sha256=sha(source),binary_sha256=sha(binary),dataset_sha256=sha(RUN/'data/dataset.json'),
                  jobs=12,width=12,node_limit=192,time_fraction=.003)
    marker=where/'result.json'
    if marker.exists():
        result=load(marker);assert result['identity']==identity;return result
    inputs=[x for x in load(RUN/'input_manifest.json') if x['guidance_role']=='development']
    assert len(inputs)==128
    def compare_case(item,mode):
        path=where/mode/f"{item['index']:06d}.jsonl";done=path.with_suffix('.json')
        path.parent.mkdir(parents=True,exist_ok=True)
        if done.exists():
            assert load(done)==dict(identity=identity,output_sha256=sha(path))
        else:
            assert not path.exists(),'inspect incomplete case before resuming'
            raw=invoke(binary,mode,item);path.write_text(raw)
            save(done,dict(identity=identity,output_sha256=sha(path)))
        return [dict(case=item['index'],**json.loads(x)) for x in path.read_text().splitlines()]
    summaries={};start=time.monotonic()
    for mode in ['fixed','compare']:
        rows=[]
        with ThreadPoolExecutor(max_workers=12) as pool:
            for f in as_completed([pool.submit(compare_case,item,mode) for item in inputs]):rows.extend(f.result())
        rows.sort(key=lambda x:(x['case'],x['problem']))
        assert len(rows)==load(RUN/'data/dataset.json')['counts']['development']['groups']
        save(where/mode/'rows.json',rows)
        summary=dict(windows=len(rows),cases=len(inputs),all_replays_passed=True,
                     baseline_saved=sum(r['original']-r['baseline'] for r in rows),
                     learned_saved=sum(r['original']-r['learned'] for r in rows),
                     wins=sum(r['learned']<r['baseline'] for r in rows),
                     ties=sum(r['learned']==r['baseline'] for r in rows),
                     losses=sum(r['learned']>r['baseline'] for r in rows))
        for field in ['seconds','calls','expanded','generated','deadlines']:
            for label in ['baseline','learned']:summary[label+'_'+field]=sum(r[label+'_'+field] for r in rows)
        summary['more_shortening']=summary['learned_saved']>summary['baseline_saved'];summaries[mode]=summary
    training=load(RUN/'training/result.json')
    result=dict(identity=identity,metrics=summaries,mae_improved=training['mae_improved'],
                gate_passed=training['mae_improved'] and summaries['compare']['more_shortening'],
                seconds=time.monotonic()-start,completed_at=now())
    save(marker,result);return result


if __name__=='__main__':
    print(json.dumps(mechanism(),indent=2),flush=True)
    print(json.dumps(development(),indent=2),flush=True)
