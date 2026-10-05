#!/usr/bin/env python3
"""配分モデルだけが変わったことと、C++の推論を確認する。"""
import json
import subprocess
import numpy as np
import torch
from build_v124_allocation import ROOT, RUN, BASE
from export_v124_allocation import build_candidate
from check_v113_integrated import block, normalize
from check_v089_board import compile_binary, trace
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from train_v124_allocation import model_new
from v089_data import save, sha, now

def load(path):return json.loads(path.read_text())


def check(name):
    where=RUN/'checks'/name;where.mkdir(parents=True,exist_ok=True);marker=where/'result.json'
    model_path=RUN/'training'/name/'model.json'
    if marker.exists():
        r=load(marker);assert r['model_sha256']==sha(model_path) and sha(ROOT/r['source'])==r['source_sha256'];return r
    source=build_candidate(model_path,'v124_'+name+'_allocation')
    for local in (True,False):
        mode='local' if local else 'judge';compile_binary(source.stem,where/('solver_'+mode),local)
        actual=normalize(preprocess(source,where/(mode+'.ii'),local));before=normalize(preprocess(BASE,where/('parent_'+mode+'.ii'),local))
        for part in ['namespace nn {','class TemporalLNS {','struct TimeKeeper','struct StatePool','class State {',
                     'struct SearchReductions','struct InitialSolutions {','static void neural_extra_starts(']:
            assert block(actual,part)==block(before,part),(name,mode,part)
        # mainの呼出順・終了処理を保持する。
        assert block(actual,'int main() {')==block(before,'int main() {')
    helper=ROOT/'adhoc/bin'/('check_v124_'+name+'.cpp')
    helper.write_text('// '+helper.name+'\n#define main unused_solver_main\n#include "'+source.name+'"\n#undef main\n'+
                     'int main(){array<float,18> f;cout<<setprecision(17);while(cin>>f[0]){for(int i=1;i<18;++i)cin>>f[i];cout<<allocation_policy::predict(f)<<"\\n";}return 0;}\n')
    compile_binary(helper.stem,where/'numerical',False)
    matrix=np.load(RUN/'training/predictions.npz')['features'][::7]
    # 欠けない程度に固定間隔で抽出し、結果による難しい例への差し替えはしない。
    payload='\n'.join(' '.join(str(float(v)) for v in row) for row in matrix)+'\n'
    proc=subprocess.run([where/'numerical'],input=payload,text=True,capture_output=True,check=True,timeout=30)
    actual=np.array([float(v) for v in proc.stdout.split()]);assert len(actual)==len(matrix)
    torch.set_num_threads(4);model=model_new();model.load_state_dict({k:torch.tensor(v) for k,v in load(model_path)['parameters'].items()})
    with torch.no_grad():expected=model(torch.tensor(matrix)).squeeze(1).numpy()
    error=float(np.max(np.abs(expected-actual)));assert error<=1e-4,error
    runtime=[]
    if name=='full':
        cases=[c for c in load(RUN/'input_manifest.json') if c['role']=='train'][:4]
        for c in cases:
            path=RUN/c['path'];proc=subprocess.run([where/'solver_judge'],input=path.read_text(),text=True,capture_output=True,check=True,timeout=10)
            result=validated_output(path,proc.stdout);assert result['E']==0
            save(where/f"judge_{c['index']}.json",result);runtime.append(result)
    result=dict(passed=True,source=str(source.relative_to(ROOT)),source_sha256=sha(source),model_sha256=sha(model_path),
                numerical_samples=len(matrix),max_abs_error=error,judge_runtime=runtime,completed_at=now())
    save(marker,result);return result
