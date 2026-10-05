#!/usr/bin/env python3
"""条件付き確率、合法候補の分割、量子化C++との数値を照合する。"""
import argparse
import json
from pathlib import Path
import numpy as np
import torch
import subprocess
from v097_policy import ROOT,RUN,BC_RUN,ChoiceData,ChoiceEngine,FactorModel,build_library,tensors
from v091_env import load
from v090_data import Geometry,save,sha,now
from build_v097_policy import build,quantize
from check_v089_board import compile_binary,transformed_input


def numerical(root,model_path,mechanism=False):
    d=root/('mechanism/numerical' if mechanism else 'numerical/factor');d.mkdir(parents=True,exist_ok=True)
    marker=d/'result.json'
    if marker.exists():assert load(marker)['model_sha256']==sha(model_path);return load(marker)
    source=ROOT/('adhoc/bin/v097_mechanism.cpp' if mechanism else 'src/bin/v097_nn_factor.cpp')
    storage=build(model_path,source)
    checker=ROOT/'adhoc/bin'/('check_v097_mechanism.cpp' if mechanism else 'check_v097_factor.cpp')
    checker.write_text(f'// {checker.name}\n#define V097_DIAGNOSTIC\n#include "../../{source.relative_to(ROOT)}"\n')
    binaries=[d/'checker_local',d/'checker_judge']
    for path,local in zip(binaries,(True,False)):compile_binary(checker.stem,path,local)
    saved=load(model_path);restored,_,_=quantize(saved['parameters']);model=FactorModel(saved['spec'])
    model.load_state_dict({k:torch.from_numpy(v) for k,v in restored.items()});model.eval()
    data=ChoiceData(BC_RUN/'data');engine=ChoiceEngine(build_library(root/'environment'),data);data.engine=engine
    errors=dict(tower=0.,split=0.,move=0.,joint_log_probability=0.,summary=0.);checks=0;prefixes=0
    try:
        for case in [c for c in data.cases if c['role']=='train'][:4]:
            fid=case['frame_start']+max(0,case['frames']-4);geo=Geometry(BC_RUN/case['path'])
            for group in range(8):
                raw,codes,counts=data.raw([fid],[group]);A=int(counts[0]);mapping=data.maps[geo.N][group]
                state=np.zeros(400,np.uint32);state[mapping]=data.states[fid];snapshot=d/'snapshot.txt';snapshot.write_text(' '.join(map(str,state))+'\n')
                transformed=mapping[codes[0,:A]&511] | (codes[0,:A] & (7<<9)) | (data.dirs[geo.N][group][(codes[0,:A]>>12)&3]<<12) | (codes[0,:A] & (7<<14))
                inp=transformed_input(geo,mapping)
                for binary in binaries if group==0 else binaries[:1]:
                    proc=subprocess.run([binary,snapshot,'dump'],input=inp,text=True,capture_output=True,check=True,timeout=10.)
                    out=json.loads(proc.stdout);p_cpp=dict(out['p']);k_cpp={(p,k):v for p,k,v in out['k']};actions={c:(v,joint) for c,v,joint in out['actions']}
                    assert set(actions)==set(map(int,transformed));assert abs(sum(np.exp(v[1]) for v in actions.values())-1)<1e-6
                    keys=raw['src'][0,:A]*8+((codes[0,:A]>>9)&7)
                    for prefix in np.unique(keys):
                        indices=np.flatnonzero(keys==prefix);teacher=int(indices[0]);altered=dict(raw,target=np.array([teacher],np.int64));f=data.factor(altered,codes,counts)
                        # 要約をNumPyでも別に計算して確認する。
                        for p in np.flatnonzero(f['p_valid'][0]):
                            features=raw['features'][0,:A][raw['src'][0,:A]==p];expected=np.r_[features.mean(0),features.max(0),np.log1p(len(features))/10]
                            errors['summary']=max(errors['summary'],float(np.max(np.abs(f['p_summary'][0,p]-expected))))
                        with torch.no_grad():pl,kl,dl=model(tensors(f,'cpu'))
                        p=int(f['p_target'][0]);k=int(f['k_target'][0]);pids=np.flatnonzero(f['p_valid'][0]);kids=np.flatnonzero(f['k_valid'][0]);n=int(f['dl_valid'][0].sum())
                        errors['tower']=max(errors['tower'],max(abs(p_cpp[int(q)]-float(pl[0,q])) for q in pids))
                        errors['split']=max(errors['split'],max(abs(k_cpp[p,int(q)]-float(kl[0,q])) for q in kids))
                        joint=torch.log_softmax(pl,-1)[0,p]+torch.log_softmax(kl,-1)[0,k]+torch.log_softmax(dl,-1)[0,:n]
                        assert n==len(indices)
                        for j,index in enumerate(indices):
                            value,logp=actions[int(transformed[index])]
                            errors['move']=max(errors['move'],abs(value-float(dl[0,j])))
                            errors['joint_log_probability']=max(errors['joint_log_probability'],abs(logp-float(joint[j])))
                        prefixes+=1
                    checks+=1
        assert errors['summary']<2e-6 and max(errors[k] for k in ('tower','split','move'))<.003 and errors['joint_log_probability']<.01,errors
    finally:engine.close()
    result=dict(passed=True,checks=checks,prefixes=prefixes,errors=errors,storage=storage,model_sha256=sha(model_path),solver_sha256=sha(source),completed_at=now())
    save(marker,result);print(json.dumps(result),flush=True);return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);p.add_argument('--mechanism',action='store_true');a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2)
    numerical(a.run,a.run/('mechanism/model.json' if a.mechanism else 'factor/model.json'),a.mechanism)
