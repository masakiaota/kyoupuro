#!/usr/bin/env python3
"""固定された部分問題でC++の順序選択、数値、費用を照合する。"""
from concurrent.futures import ThreadPoolExecutor
import gzip
import json
import re
import subprocess
import numpy as np
import torch

from build_v123_order import ROOT, RUN, BASE
from export_v123_order import build
from check_v113_integrated import block, normalize
from check_v089_board import compile_binary, trace
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from collect_v122_orders import codes_to_text
from train_v123_order import model_new
from v089_data import save, sha, now, status
from v090_data import RUN as BC_RUN


def load(path):return json.loads(path.read_text())


TIMED=r'''
            auto process_seconds=[](){timespec stamp;clock_gettime(CLOCK_PROCESS_CPUTIME_ID,&stamp);return stamp.tv_sec+stamp.tv_nsec*1e-9;};
            vector<Move> reference,learned,forced;bool old_ok=false,learned_ok=false;
            double baseline_seconds=0,learned_seconds=0;
            auto old_trial=[&](){const double start=process_seconds();old_ok=lns.insert_two_orders(initial,base,order,allowance,reference);baseline_seconds=process_seconds()-start;};
            auto new_trial=[&](){const double start=process_seconds();learned_ok=lns.learned_two_orders(initial,base,order,allowance,learned);learned_seconds=process_seconds()-start;};
            if(index%2){new_trial();old_trial();}else{old_trial();new_trial();}
            const bool forced_ok=lns.forced_two_orders(initial,base,order,allowance,forced);
            if(forced_ok!=old_ok||(old_ok&&!lns.same_moves(forced,reference))||rng.x!=before)throw logic_error("forced old order mismatch");
            if(learned_ok)verify(learned);
            cout<<"{\"kind\":\"check\",\"phase\":"<<phase<<",\"group\":"<<index
                <<",\"baseline_seconds\":"<<setprecision(17)<<baseline_seconds<<",\"learned_seconds\":"<<learned_seconds
                <<",\"learned_ok\":"<<(learned_ok?"true":"false")<<",\"learned_cost\":"<<(learned_ok?int(learned.size()):allowance+8)
                <<",\"scores\":[";
            for(int k=0;k<int(context.values.size());++k){if(k)cout<<',';cout<<order_policy::predict(context.values[k]);}
            cout<<"],\"learned_path\":";path(cout,learned);cout<<"}\n";
'''


def generate_checker(source):
    text=source.read_text()
    learned=block(text,'bool insert_two_orders(')
    parent=block(BASE.read_text(),'bool insert_two_orders(')
    forced=learned.replace('bool insert_two_orders(','bool forced_two_orders(',1).replace(
        'if(n>=3&&int(base.size())<=allowance)', 'if(false)')
    text=text.replace(learned,learned.replace('bool insert_two_orders(','bool learned_two_orders(',1)+'\n'+forced+'\n'+parent,1)
    text=text.replace(block(text,'double exact_elapsed_sec() const'), 'double exact_elapsed_sec() const { return 0.0; }',1)
    text=text.replace('while(time_keeper.exact_elapsed_sec()<min(slice_end,end))','while(iteration<512)',1)
    core=ROOT/'adhoc/bin/v123_order_check_core.cpp';core.write_text('// '+core.name+'\n'+text.split('\n',1)[1])
    checker=(ROOT/'results/nn_rank/v122/20261004_order_studio/frozen/collect_v122_orders.cpp').read_text()
    checker=checker.replace('v122_order_core.cpp','v123_order_check_core.cpp')
    anchor='            auto context=lns.order_features(base,order,allowance);'
    assert checker.count(anchor)==1;checker=checker.replace(anchor,anchor+'\n'+TIMED,1)
    old='            vector<Move> reference;const bool old_ok=lns.insert_two_orders(initial,base,order,allowance,reference);'
    assert checker.count(old)==1;checker=checker.replace(old,'',1)
    target=ROOT/'adhoc/bin/check_v123_order.cpp';target.write_text('// '+target.name+'\n'+checker.split('\n',1)[1])
    return core,target


def check():
    where=RUN/'mechanism';where.mkdir(exist_ok=True)
    if (where/'result.json').exists():return load(where/'result.json')
    source=build();generate_checker(source)
    status(RUN/'pipeline','building_mechanism')
    def checker():compile_binary('check_v123_order',where/'checker',True)
    def submission():
        for local in (True,False):
            mode='local' if local else 'judge'
            compile_binary(source.stem,where/('solver_'+mode),local)
            actual=normalize(preprocess(source,where/(mode+'.ii'),local))
            before=normalize(preprocess(BASE,where/('parent_'+mode+'.ii'),local))
            for name in ['namespace nn {','struct TimeKeeper','struct Board {','struct StatePool','class State {',
                         'void grow_initial_solutions(','static void neural_extra_starts(','struct InitialSolutions {',
                         'class PortionRouter','class FinitePlanner','struct SearchReductions','bool insert_at_impl(',
                         'bool insert_packet(','bool packet_neighbor(','bool flexible_neighbor(','bool paired_neighbor(',
                         'vector<Move> causal_reorder(','void make_candidates(']:
                assert block(actual,name)==block(before,name),(mode,name)
    with ThreadPoolExecutor(max_workers=2) as pool:
        tasks=[pool.submit(checker),pool.submit(submission)]
        for task in tasks:task.result()
    cases=[c for c in load(RUN/'inputs.json') if c['order_role']=='development']
    def probe(case):
        dest=where/'cases'/f"{case['index']:06d}";dest.mkdir(parents=True,exist_ok=True)
        with (BC_RUN/case['path']).open() as inp,(dest/'rows.jsonl').open('w') as out,(dest/'stderr.log').open('w') as err:
            subprocess.run([where/'checker'],stdin=inp,stdout=out,stderr=err,check=True,timeout=300)
        return case,dest
    with ThreadPoolExecutor(max_workers=12) as pool:completed=list(pool.map(probe,cases))
    torch.set_num_threads(4);model=model_new();parameters=load(RUN/'training/model.json')['parameters']
    model.load_state_dict({k:torch.tensor(v) for k,v in parameters.items()});model.eval()
    timing=[0.,0.];feature_error=0.;score_error=0.;groups=0;baseline_complete=0;learned_complete=0;cost_difference=0
    replayed=0;near_ties=0
    for case,dest in completed:
        with gzip.open(RUN/'data'/f"{case['index']:06d}"/'rows.jsonl.gz','rt') as stream:
            saved={(r['phase'],r['group']):r for line in stream if (r:=json.loads(line))['extracted']}
        lines=[json.loads(line) for line in (dest/'rows.jsonl').read_text().splitlines()]
        checks={(r['phase'],r['group']):r for r in lines if r.get('kind')=='check'}
        actual={(r['phase'],r['group']):r for r in lines if r.get('extracted')}
        assert set(actual)==set(saved)==set(checks)
        for key,row in actual.items():
            ref=saved[key];current=checks[key]
            a=np.array(row['features'],dtype=np.float32);b=np.array(ref['features'],dtype=np.float32)
            feature_error=max(feature_error,float(np.max(np.abs(a-b))))
            assert row['costs']==ref['costs'] and row['complete']==ref['complete']
            with torch.no_grad():pred=model(torch.from_numpy(a)).squeeze(-1).numpy()
            cpp=np.array(current['scores']);score_error=max(score_error,float(np.max(np.abs(pred-cpp))))
            python_choice=1+int(np.argmin(pred[1:]));cpp_choice=1+int(np.argmin(cpp[1:]))
            if python_choice!=cpp_choice:
                assert abs(float(pred[python_choice]-pred[cpp_choice]))<=2e-4;near_ties+=1
            expected=min(row['costs'][0],row['costs'][cpp_choice])
            assert expected==current['learned_cost']
            base=min(row['costs'][0],row['costs'][row['alternate']])
            baseline_complete+=base<=row['allowance'];learned_complete+=current['learned_ok']
            cost_difference+=expected-base;groups+=1
            timing[0]+=current['baseline_seconds'];timing[1]+=current['learned_seconds']
        for phase in (0,1):
            example=next((r for key,r in checks.items() if key[0]==phase and r['learned_ok']),None)
            if example:
                result=validated_output(BC_RUN/case['path'],codes_to_text(example['learned_path']))
                assert result['E']==0 and result['T']==example['learned_cost'];replayed+=1
    assert feature_error<=1e-7 and score_error<=1e-4,(feature_error,score_error)
    partial_pass=learned_complete>=baseline_complete and cost_difference/groups<=-.10
    time_pass=timing[1]/timing[0]<=1.10
    runtime=[]
    if partial_pass and time_pass:
        train=[c for c in load(RUN/'inputs.json') if c['order_role']=='train'][:4]
        for case in train:
            for local in (True,False):
                mode='local' if local else 'judge';dest=where/'runtime'/f"{case['index']:06d}"/mode;dest.mkdir(parents=True,exist_ok=True)
                proc=subprocess.run([where/('solver_'+mode)],input=(BC_RUN/case['path']).read_text(),text=True,capture_output=True,timeout=10,check=True)
                (dest/'output.txt').write_text(proc.stdout);(dest/'stderr.log').write_text(proc.stderr)
                checked=validated_output(BC_RUN/case['path'],proc.stdout);assert checked['E']==0
                counts=trace(proc.stderr)
                if local:
                    assert counts['state_pool_free_at_end']==4 and counts.get('order_nn_calls',0)>0
                    assert all(counts.get(k,0)==0 for k in ['lns_errors','construction_errors','lns_invalid_candidates'])
                runtime.append(dict(case=case['index'],mode=mode,trace=counts,**checked))
    result=dict(source_sha256=sha(source),groups=groups,feature_error=feature_error,score_error=score_error,
                near_tie_choices=near_ties,baseline_complete=baseline_complete,learned_complete=learned_complete,
                mean_cost_difference=cost_difference/groups,baseline_cpu_seconds=timing[0],learned_cpu_seconds=timing[1],
                cpu_ratio=timing[1]/timing[0],independent_python_replays=replayed,runtime=runtime,
                gates={'learned':dict(passed=partial_pass and time_pass,partial_pass=partial_pass,time_pass=time_pass)},completed_at=now())
    save(where/'result.json',result)
    return result


if __name__=='__main__':
    result=check();print(json.dumps({k:v for k,v in result.items() if k!='runtime'},ensure_ascii=False))
