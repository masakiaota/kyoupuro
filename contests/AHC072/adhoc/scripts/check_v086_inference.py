#!/usr/bin/env python3
"""固定8局面の全選択段階で、提出用C++と学習側の点数を照合する。"""
import argparse
import json
from pathlib import Path
import subprocess

import numpy as np
import torch
from train_v086_policy import Policy
from v086_data import SOURCE, ROOT, examples, save, sha

TRAIN = ROOT/'results/nn_rank/v086/20261002T181701_studio'


def check(binary, output, model_path=None):
    torch.set_num_threads(1)
    model_path=model_path or TRAIN/'model.json'
    model_data=json.loads(model_path.read_text())
    model=Policy();model.load_state_dict({k:torch.tensor(v) for k,v in model_data['parameters'].items()})
    mean=np.array(model_data['mean'],np.float32);scale=np.array(model_data['scale'],np.float32)
    result={'passed':False,'binary_sha256':sha(binary),'model_sha256':sha(model_path),'groups':0,'max_error':0.}
    for item in json.loads((SOURCE/'input_manifest.json').read_text())[:4]:
        text,ids,rows,_=examples(item); positions=[0,len(rows)-1] if len(rows)>1 else [0]
        parts=[text.rstrip(),str(len(positions))]
        for pos in positions:
            moves=rows[pos]['before'];parts.append(str(len(moves)))
            parts.extend(' '.join(map(str,m)) for m in moves)
        child=subprocess.run([str(binary)],input=('\n'.join(parts)+'\n').encode(),capture_output=True,check=True,cwd=ROOT,timeout=60)
        M=len(ids);actual=np.frombuffer(child.stdout,np.float32).reshape(len(positions),13,M+1)
        folder=TRAIN/'cases'/f"{item['index']:06d}"
        nodes=np.load(folder/'nodes.npy',mmap_mode='r');edges=np.load(folder/'edges.npy',mmap_mode='r')
        for got,pos in zip(actual,positions):
            prefix=np.zeros((1,13,M),np.float32)
            for count in range(13):prefix[0,count,:min(count,M)]=1
            batch={'nodes':torch.tensor((nodes[pos:pos+1]-mean)/scale),'edges':torch.tensor(np.array(edges[pos:pos+1])),
                   'valid':torch.ones((1,M),dtype=torch.bool),'prefix':torch.tensor(prefix)}
            with torch.no_grad():expected=model(batch).numpy()[0]
            np.testing.assert_array_equal(got < -1e8,expected < -1e8)
            mask=expected>-1e8
            np.testing.assert_allclose(got[mask],expected[mask],rtol=2e-5,atol=2e-4)
            result['max_error']=max(result['max_error'],float(abs(got[mask]-expected[mask]).max()))
            result['groups']+=1
    assert result['groups']==8
    result['passed']=True;save(output,result);print(json.dumps(result),flush=True)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--model',type=Path);a=p.parse_args();check(a.binary.resolve(),a.output.resolve(),a.model)
