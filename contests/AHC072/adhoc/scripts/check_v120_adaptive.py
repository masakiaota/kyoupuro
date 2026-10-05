#!/usr/bin/env python3
"""配分計算と元の128反復を照合し、学習用入力で合法性を確認する。"""
from concurrent.futures import ThreadPoolExecutor
import difflib
import json
import math
import subprocess
from build_v120_adaptive import ROOT, RUN, BASE
from check_v113_integrated import block, normalize
from check_v089_board import compile_binary, trace
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save, sha, now, status
from v090_data import RUN as BC_RUN

def load(path):return json.loads(path.read_text())

def equal_reward_cost_check(binary,where):
    # 他の報酬・受理・呼出回数を等しくし、費用だけを100倍変える。
    records=[]
    for _ in range(256):
        for kind in range(6):
            elapsed=.00001 if kind==0 else .001
            records.append(f'{kind} {elapsed:.17g} 1 1 63 0.5')
    proc=subprocess.run([binary],input='\n'.join(records)+'\n',text=True,capture_output=True,check=True,timeout=10)
    (where/'equal_reward_cost_output.txt').write_text(proc.stdout)
    weight=list(map(float,proc.stdout.splitlines()[-1].split()[:6]))
    # 通常/依存の基準比は2:1。観測する利得を同じにした上でそれを超えること。
    assert weight[0]/weight[1]>2.0,weight
    return dict(observations=len(records),equal_reward=True,cost_ratio=100,probability_ratio=weight[0]/weight[1])

FIXED=r'''
int main(int argc,char** argv) {
    if(argc!=2)return 2;
    ios::sync_with_stdio(false);cin.tie(nullptr);
    board_info.read();state_pool.prepare(board_info.cell_count);
    for(int p=0;p<board_info.cell_count;p++)
        rng.x=(rng.x^(board_info.initial[p]+17*board_info.nest_code[p]+p))*0x9e3779b97f4a7c15ULL;
    vector<Move> best;ifstream saved(argv[1]);int i,j,k,l;char d;
    while(saved>>i>>j>>k>>d>>l){
        int dir=int(string("UDLR").find(d));
        best.push_back({int16_t(board_info.cell_id[i][j]),uint8_t(k),uint8_t(dir),uint8_t(l)});
    }
    time_keeper.time_limit_sec=1e20;
    TemporalLNS lns;lns.random_state=rng.next();lns.optimize(best,1e9,0,1e9);
    if(state_pool.free_count!=4)throw logic_error("state pool not returned");
    cerr<<lns.attempts<<' '<<lns.accepted<<' '<<lns.improvements<<' '<<lns.random_state<<' '<<rng.x<<'\n';
    for(auto m:best)cout<<board_info.row[m.p]<<' '<<board_info.col[m.p]<<' '<<int(m.k)<<' '
        <<board_info.dirs[m.d]<<' '<<int(m.l)<<'\n';
}
'''

POLICY=r'''
int main() {
    RepairAllocation allocation;int kind,gain,changed,mask;double elapsed,draw;
    cout<<setprecision(17);
    while(cin>>kind>>elapsed>>gain>>changed>>mask>>draw) {
        allocation.observe(kind,elapsed,gain,changed!=0);
        array<bool,6> allowed{};for(int i=0;i<6;++i)allowed[i]=(mask>>i)&1;
        const auto p=allocation.probabilities(allowed);
        for(double value:allocation.weight)cout<<value<<' ';
        for(double value:p)cout<<value<<' ';
        cout<<allocation.select(allowed,draw)<<' '<<allocation.ready()<<'\n';
    }
}
'''

def policy_reference(binary,where):
    limit=1.9*.8;reorder=1/113;paired=(1-reorder)/17;packet=(1-reorder-paired)/7
    regular=1-reorder-paired-packet
    prior=[regular*2/3,regular/3,packet/3,packet*2/3,paired,reorder]
    weight=prior.copy();reward=[0.]*6;seconds=[0.]*6;records=[];expected=[]
    for index in range(1536):
        kind=index%6
        elapsed=[.00001,.001,.00004,.00008,.00015,.0001][kind]
        gain=(index//6)%11;changed=(index%3==0)
        if index%67==0:elapsed=0
        if index%61==0:gain=30
        mask=[63,35,59,51][(index//96)%4];draw=((index*2971)%10000)/10000
        records.append(f'{kind} {elapsed:.17g} {gain} {int(changed)} {mask} {draw:.17g}')
        reward[kind]+=min(8,max(0,gain))+(0.05 if changed else 0)
        seconds[kind]+=max(elapsed,limit*1e-7)
        if (index+1)%64==0:
            reward=[v*.75 for v in reward];seconds=[v*.75 for v in seconds]
            pooled=(sum(reward)+1)/(sum(seconds)+.01*limit);smoothing=.002*limit
            rates=[prior[i]*(reward[i]+smoothing*pooled)/(seconds[i]+smoothing) for i in range(6)]
            weight=[.25*prior[i]+.75*rates[i]/sum(rates) for i in range(6)]
        available=[i for i in range(6) if (mask>>i)&1];denominator=sum(weight[i] for i in available)
        p=[weight[i]/denominator if i in available else 0 for i in range(6)]
        chosen=available[-1];cumulative=0
        for i in available:
            cumulative+=p[i]
            if draw<cumulative:chosen=i;break
        assert abs(sum(p)-1)<1e-12 and all(p[i]>=.25*prior[i]-1e-12 for i in available)
        expected.append(weight+p+[chosen,int(index+1>=128)])
    proc=subprocess.run([binary],input='\n'.join(records)+'\n',text=True,capture_output=True,check=True,timeout=10)
    (where/'policy_input.txt').write_text('\n'.join(records)+'\n');(where/'policy_output.txt').write_text(proc.stdout)
    actual=[[float(v) for v in line.split()] for line in proc.stdout.splitlines()]
    assert len(actual)==len(expected)
    error=0.
    for a,b in zip(actual,expected):
        error=max(error,max(abs(x-y) for x,y in zip(a[:12],b[:12])))
        assert a[12:]==b[12:]
    assert error<1e-12,error
    assert weight[0]/prior[0]>weight[1]/prior[1], 'equal reward, lower cost must be preferred'
    return dict(observations=len(actual),max_error=error,cheap_method_preferred=True)

def check():
    where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True)
    source=ROOT/load(RUN/'sources.json')['sources']['adaptive']['path']
    identity={str(p.relative_to(ROOT)):sha(p) for p in (source,BASE)}
    marker=where/'result.json'
    if marker.exists():
        result=load(marker);assert result['sources']==identity
        if 'equal_reward_cost' not in result['synthetic']:
            result['synthetic']['equal_reward_cost']=equal_reward_cost_check(where/'policy',where)
            save(marker,result)
        return result
    status(RUN/'pipeline','mechanism_builds')
    def build_submission():
        dest=where/'adaptive';dest.mkdir(exist_ok=True)
        for local in (True,False):
            mode='local' if local else 'judge'
            compile_binary(source.stem,dest/('solver_'+mode),local)
            actual=normalize(preprocess(source,dest/(mode+'.ii'),local))
            before=normalize(preprocess(BASE,dest/('parent_'+mode+'.ii'),local))
            names=['namespace nn {','struct TimeKeeper','struct Board {','struct StatePool','class State {',
                   'void grow_initial_solutions(','static void neural_extra_starts(','struct InitialSolutions {',
                   'class PortionRouter','class FinitePlanner','struct SearchReductions',
                   'bool insert_at_impl(','bool insert_two_orders(','bool insert_packet(',
                   'bool packet_neighbor(','bool flexible_neighbor(','bool paired_neighbor(',
                   'vector<Move> causal_reorder(','void make_candidates(']
            for name in names:assert block(actual,name)==block(before,name),(mode,name)
            (dest/(mode+'.patch')).write_text(''.join(difflib.unified_diff(before.splitlines(True),actual.splitlines(True))))
    def build_fixed():
        for label,parent in [('ref',BASE),('adaptive',source)]:
            text=parent.read_text();clock=block(text,'double exact_elapsed_sec() const')
            text=text.replace(clock,'double exact_elapsed_sec() const { return 0.0; }',1)
            loop='while(time_keeper.exact_elapsed_sec()<min(slice_end,end))'
            assert text.count(loop)==1
            text=text.replace(loop,'while(iteration<128)',1)
            text=text.replace(block(text,'int main()'),FIXED,1)
            target=ROOT/'adhoc/bin'/f'check_v120_fixed_{label}.cpp'
            target.write_text('// '+target.name+'\n'+text.split('\n',1)[1])
            compile_binary(target.stem,where/('fixed_'+label),True)
    def build_policy():
        target=ROOT/'adhoc/bin/check_v120_policy.cpp'
        target.write_text('// check_v120_policy.cpp\n#define main v120_submission_main\n#include "v120_adaptive_repair.cpp"\n#undef main\n'+POLICY)
        compile_binary(target.stem,where/'policy',True)
    with ThreadPoolExecutor(max_workers=3) as pool:
        futures=[pool.submit(f) for f in (build_submission,build_fixed,build_policy)]
        for future in futures:future.result()
    synthetic=policy_reference(where/'policy',where)
    synthetic['equal_reward_cost']=equal_reward_cost_check(where/'policy',where)
    cases=load(ROOT/'results/nn_rank/v118/20261004_heterogeneous_studio/mechanism/inputs.json')
    save(where/'inputs.json',cases);reports=[];outputs=0;activation=[0]*6
    for case in cases:
        inp=BC_RUN/case['path'];assert sha(inp)==case['sha256'];name=f"{case['index']:06d}"
        seed=ROOT/'results/nn_rank/v118/20261004_heterogeneous_studio/mechanism/runtime'/name/'local/seed_-105.txt'
        pair=[]
        for label in ('ref','adaptive'):
            dest=where/'fixed'/name/label;dest.mkdir(parents=True,exist_ok=True)
            proc=subprocess.run([where/('fixed_'+label),seed],input=inp.read_text(),text=True,capture_output=True,timeout=30)
            (dest/'output.txt').write_text(proc.stdout);(dest/'stderr.log').write_text(proc.stderr)
            assert proc.returncode==0,(name,label,proc.stderr[-2000:])
            assert validated_output(inp,proc.stdout)['E']==0;outputs+=1
            pair.append((proc.stdout,proc.stderr))
        assert pair[0]==pair[1],(name,'warm-up differs from parent')
        runtime=[]
        for mode in ('local','judge'):
            dest=where/'runtime'/name/mode;dest.mkdir(parents=True,exist_ok=True)
            proc=subprocess.run([where/'adaptive'/('solver_'+mode)],input=inp.read_text(),text=True,capture_output=True,timeout=10)
            (dest/'output.txt').write_text(proc.stdout);(dest/'stderr.log').write_text(proc.stderr)
            assert proc.returncode==0,(name,mode,proc.stderr[-1500:])
            checked=validated_output(inp,proc.stdout);assert checked['E']==0;outputs+=1
            counts=trace(proc.stderr)
            if mode=='local':
                assert counts['allocation_draws']>0 and counts['allocation_updates']>0
                assert counts['state_pool_free_at_end']==4
                assert all(counts.get(k,0)==0 for k in ['lns_errors','construction_errors','lns_invalid_candidates'])
                for i in range(6):activation[i]+=counts[f'allocation_{i}_trials']
            runtime.append(dict(mode=mode,trace=counts,**checked))
        reports.append(dict(case=case['index'],fixed_identical=True,runtime=runtime))
    assert min(activation)>0,activation
    result=dict(sources=identity,synthetic=synthetic,fixed_reference_pairs=4,independent_outputs=outputs,
                reports=reports,activation=activation,preprocessing_repair_algorithms_identical=True,
                gates={'adaptive':dict(passed=True)},completed_at=now())
    save(marker,result);return result

if __name__=='__main__':
    result=check();print(json.dumps({k:v for k,v in result.items() if k!='reports'},ensure_ascii=False))
