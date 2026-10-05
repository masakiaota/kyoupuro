#!/usr/bin/env python3
"""実際に提出するC++の特徴、点数、キャッシュ失効と合法性を確認する。"""
import argparse
import gzip
import json
from pathlib import Path
import subprocess

import numpy as np
import torch
from v089_data import ROOT, save, sha, now
from v090_data import RUN as BC_RUN
from v085_data import Problem, packed_text
from v106_data import Dataset, RUN
from train_v087_value import Value,tensors
from check_v089_board import compile_binary,trace
from run_v089_evaluation import validated_output
from run_v105_hybrid import move_hash


def check(root):
    directory=root/'integration';directory.mkdir(parents=True,exist_ok=True)
    source=ROOT/'src/bin/v106_nn_lns.cpp';model_path=root/'main/model.json'
    marker=directory/'mechanism.json'
    if marker.exists():
        record=json.loads(marker.read_text());assert record['source_sha256']==sha(source) and record['model_sha256']==sha(model_path)
        assert record['passed'];return record
    assert json.loads((root/'main/result.json').read_text())['gate_passed']
    checker=ROOT/'adhoc/bin/check_v106_selector.cpp'
    checker.write_text('''// check_v106_selector.cpp
#define AHC072_NN_PROBE
#define main unused_v106_main
#include "../../src/bin/v106_nn_lns.cpp"
#undef main
struct NeuralRankProbe {
    static void run() {
        board_info.read();state_pool.prepare(board_info.cell_count);
        time_keeper.start_=chrono::steady_clock::now();time_keeper.time_limit_sec=1e9;TemporalLNS lns;
        int Q;cin>>Q;
        while(Q--) {
            int T;cin>>T;vector<Move> moves;
            for(int i=0;i<T;i++){int p,k,d,l;cin>>p>>k>>d>>l;moves.push_back({int16_t(p),uint8_t(k),uint8_t(d),uint8_t(l)});}
            lns.trajectory_changed();lns.make_candidates(moves);
            const auto random_before=rng.x;
            lns.prepare_nn106(moves);auto encoded=lns.nn106.h;
            lns.prepare_nn106(moves);
            if(encoded!=lns.nn106.h||rng.x!=random_before)throw logic_error("selector encoding mutated cached values or random state");
            int n;cin>>n;
            while(n--){int count;cin>>count;vector<int> ids(count);for(int& id:ids)cin>>id;
                const auto values=lns.nn106.score(ids);cout.write(reinterpret_cast<const char*>(values.data()),sizeof(float)*2);}
            if(rng.x!=random_before||state_pool.free_count!=4)throw logic_error("selector mutated random state or leaked a board");
        }
        if(!cin)throw logic_error("incomplete selector check input");
    }
};
int main(){ios::sync_with_stdio(false);cin.tie(nullptr);NeuralRankProbe::run();}
''')
    for local in (True,False):
        mode='local' if local else 'judge'
        compile_binary(checker.stem,directory/('checker_'+mode),local)
        compile_binary(source.stem,directory/('solver_'+mode),local)
    data=Dataset(root/'main');stored=json.loads(model_path.read_text());model=Value()
    model.load_state_dict({k:torch.tensor(v) for k,v in stored['parameters'].items()});model.eval()
    train=[c['input'] for c in data.description['cases'] if c['input']['role']=='train']
    selected=[train[i] for i in np.linspace(0,len(train)-1,4,dtype=int)]
    maximum=0.;checks=0;executions=[]
    for ci,case in enumerate(selected):
        inp=BC_RUN/case['path'];text=inp.read_text();folder=root/'main/cases'/f"{case['index']:06d}"
        with gzip.open(folder/'raw.jsonl.gz','rt') as f:rows=[json.loads(line) for line in f]
        request=[text.rstrip(),str(len(rows))]
        for row in rows:
            request.append(str(len(row['current'])));request.extend(' '.join(map(str,m)) for m in row['current'])
            request.append(str(len(row['candidates'])))
            for c in row['candidates']:request.append(' '.join(map(str,[len(c['ids'])]+c['ids'])))
        indices=[i for i,r in enumerate(data.rows) if r[0]==case['index']]
        raw=data.batch(indices)
        with torch.no_grad():expected=model(tensors(raw,'cpu')).numpy()[raw['candidate_valid']]
        for mode in ('local','judge'):
            child=subprocess.run([str(directory/('checker_'+mode))],input=('\n'.join(request)+'\n').encode(),capture_output=True,timeout=60)
            (directory/f"numeric_{case['index']}_{mode}.log").write_bytes(child.stderr)
            child.check_returncode()
            actual=np.frombuffer(child.stdout,np.float32).reshape(-1,2)
            np.testing.assert_allclose(actual,expected,atol=3e-4,rtol=2e-5)
            maximum=max(maximum,float(np.max(abs(actual-expected))));checks+=len(expected)
            if ci<2:
                child=subprocess.run([str(directory/('solver_'+mode))],input=text,text=True,capture_output=True,check=True,timeout=8)
                tag=f"rollout_{case['index']}_{mode}";(directory/(tag+'.txt')).write_text(child.stdout);(directory/(tag+'.err')).write_text(child.stderr)
                result=validated_output(inp,child.stdout);assert result['E']==0
                counts=trace(child.stderr)
                if mode=='local':
                    expected_hash=move_hash(packed_text(Problem.read(inp),rows[0]['current']))
                    assert counts['nn_initial_hash']==expected_hash
                    assert counts['nn_initial_ops']==rows[0]['current_T'] and counts['nn_complete']==1
                    assert counts['nn_calls']>0 and counts['nn106_encoded_states']>0
                    assert counts['state_pool_free_at_end']==4
                    assert all(counts.get(k,0)==0 for k in ('lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery'))
                executions.append(dict(case=case['index'],mode=mode,trace=counts,**result))
    parent=(ROOT/'src/bin/v105_nn_lns.cpp').read_text();current=source.read_text()
    start='namespace nn {';stop='} // namespace nn'
    assert parent[parent.index(start):parent.index(stop)]==current[current.index(start):current.index(stop)]
    result=dict(passed=True,checks=checks,max_error=maximum,initial_nn_identical=True,executions=executions,
                source_sha256=sha(source),model_sha256=sha(model_path),source_bytes=source.stat().st_size,completed_at=now())
    save(marker,result);print(json.dumps({k:v for k,v in result.items() if k!='executions'}),flush=True)
    return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2);check(a.run)
