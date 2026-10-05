#!/usr/bin/env python3
"""固定した4学習入力で移植の同一性と完成候補の合法性を確認する。"""
from concurrent.futures import ThreadPoolExecutor
import difflib
import json
import os
import subprocess
from build_v118_heterogeneous import ROOT, RUN, BASE, CLASSIC
from check_v113_integrated import block, normalize
from check_v089_board import compile_binary, trace
from run_v105_hybrid import preprocess, move_hash
from run_v089_evaluation import validated_output
from v089_data import save, sha, now, status
from v090_data import RUN as BC_RUN

def load(path):return json.loads(path.read_text())

PROBE=r'''
int main(int argc,char** argv) {
    if(argc!=2)return 2;
    ios::sync_with_stdio(false);cin.tie(nullptr);
    board_info.read();state_pool.prepare(board_info.cell_count);
    for(int p=0;p<board_info.cell_count;p++)
        rng.x=(rng.x^(board_info.initial[p]+17*board_info.nest_code[p]+p))*0x9e3779b97f4a7c15ULL;
    time_keeper.time_limit_sec=1e20;
    const uint64_t initial_random=rng.x;
    vector<Move> best;
    auto output=[&](const ConstructionState& result,int kind) {
        if(result.E)throw logic_error("probe incomplete");
        ofstream out(string(argv[1])+"/seed_"+to_string(kind)+".txt");
        for(auto m:result.moves)out<<board_info.row[m.p]<<' '<<board_info.col[m.p]<<' '<<int(m.k)<<' '
            <<board_info.dirs[m.d]<<' '<<int(m.l)<<'\n';
        if(best.empty()||result.moves.size()<best.size())best=result.moves;
        cout<<kind<<' '<<result.E<<' '<<result.moves.size()<<' '<<rng.x<<'\n';
    };
    for(int variant=0;variant<3;++variant){auto result=tree_baseline(variant);output(result,variant);}
    try {
        EmptyDP potential;Constructor solver(potential,0,1e20,best.size()+1);
        auto result=solver.run();output(result,3);
    } catch(const Deadline&) {cout<<"capped "<<rng.x<<'\n';}
    if(state_pool.free_count!=4)throw logic_error("probe state ownership");
#ifdef V118_SCOPE_CHECK
    rng.x=initial_random;
    InitialSolutions pool;pool.offer(best,-105);
    classical_start(pool,best);
    if(rng.x!=initial_random||time_keeper.time_limit_sec!=1e20||state_pool.free_count!=4)
        throw logic_error("classical scope not restored");
#endif
}
'''

def check():
    where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True)
    source=ROOT/load(RUN/'sources.json')['sources']['mixed']['path']
    identity={str(p.relative_to(ROOT)):sha(p) for p in (source,BASE,CLASSIC)}
    marker=where/'result.json'
    if marker.exists():
        result=load(marker);assert result['sources']==identity;return result
    status(RUN/'pipeline','mechanism_builds')
    # 同じbasenameのLOCALと非LOCALは逐次。独立した診断用プログラムだけ並列ビルドする。
    def build_submission():
        for local in (True,False):
            mode='local' if local else 'judge';dest=where/'mixed';dest.mkdir(exist_ok=True)
            compile_binary(source.stem,dest/('solver_'+mode),local)
            expanded=normalize(preprocess(source,dest/(mode+'.ii'),local))
            reference=normalize(preprocess(BASE,dest/('parent_'+mode+'.ii'),local))
            for name in ('namespace nn {','struct TimeKeeper','struct Board {','struct StatePool',
                         'class State {','class TemporalLNS','void grow_initial_solutions(',
                         'class PortionRouter','vector<Move> compress_joint_windows(',
                         'void polish_final_reductions('):
                assert block(expanded,name)==block(reference,name),(mode,name)
            before=block(reference,'static void neural_extra_starts(').replace('for(int sym:{4,1,5})','for(int sym:{4,1})')
            assert block(expanded,'static void neural_extra_starts(')==before
            (dest/(mode+'.patch')).write_text(''.join(difflib.unified_diff(reference.splitlines(True),expanded.splitlines(True))))
    def build_runtime():
        wrapper=ROOT/'adhoc/bin/check_v118_seeds.cpp'
        wrapper.write_text('// check_v118_seeds.cpp\n#define V118_SEED_DUMP\n#include "v118_heterogeneous_initial.cpp"\n')
        for local in (True,False):
            compile_binary(wrapper.stem,where/('seed_local' if local else 'seed_judge'),local)
    def build_fixed():
        for label,parent in [('ref',CLASSIC),('port',source)]:
            text=parent.read_text()
            clock=block(text,'double exact_elapsed_sec() const')
            text=text.replace(clock,'double exact_elapsed_sec() const { return 0.0; }',1)
            text=text.replace(block(text,'int main()'),PROBE,1)
            target=ROOT/'adhoc/bin'/f'check_v118_fixed_{label}.cpp'
            target.write_text('// '+target.name+'\n'+('#define V118_SCOPE_CHECK\n' if label=='port' else '')+text.split('\n',1)[1])
            compile_binary(target.stem,where/('fixed_'+label),True)
    with ThreadPoolExecutor(max_workers=3) as pool:
        futures=[pool.submit(f) for f in (build_submission,build_runtime,build_fixed)]
        for future in futures:future.result()
    manifest=[c for c in load(BC_RUN/'input_manifest.json') if c['role']=='train']
    cases=[manifest[i] for i in (0,1000,2000,3000)]
    save(where/'inputs.json',cases)
    checked=0;reports=[];reference_pairs=0
    for case in cases:
        inp=BC_RUN/case['path'];assert sha(inp)==case['sha256'];name=f"{case['index']:06d}"
        fixed=[];real=[]
        for label in ('ref','port'):
            dest=where/'fixed'/name/label;dest.mkdir(parents=True,exist_ok=True)
            proc=subprocess.run([where/('fixed_'+label),dest],input=inp.read_text(),text=True,capture_output=True,timeout=120)
            (dest/'stderr.log').write_text(proc.stderr);(dest/'stdout.log').write_text(proc.stdout)
            assert proc.returncode==0,(label,name,proc.stderr[-1000:])
            fixed.append((proc.stdout,{p.name:sha(p) for p in dest.glob('*.txt')}))
            for path in dest.glob('*.txt'):
                assert validated_output(inp,path.read_text())['E']==0;checked+=1
        assert fixed[0]==fixed[1],(name,'classical moves or random state differ');reference_pairs+=1
        for mode in ('local','judge'):
            dest=where/'runtime'/name/mode;dest.mkdir(parents=True,exist_ok=True)
            env=os.environ.copy();env['V118_SEED_DIRECTORY']=str(dest)
            proc=subprocess.run([where/('seed_'+mode)],input=inp.read_text(),text=True,capture_output=True,env=env,timeout=10)
            (dest/'stderr.log').write_text(proc.stderr);(dest/'final.txt').write_text(proc.stdout)
            assert proc.returncode==0,(name,mode,proc.stderr[-1000:])
            counts=trace(proc.stderr)
            for path in dest.glob('*.txt'):
                assert validated_output(inp,path.read_text())['E']==0;checked+=1
            first=dest/'seed_-105.txt';assert first.exists()
            first_hash=move_hash(first.read_text())
            if mode=='local':
                assert counts['nn_initial_hash']==first_hash
                assert counts['state_pool_free_at_end']==4 and counts['race_seed_count']<=4
                assert counts['heterogeneous_retained']==1
                assert all(counts.get(k,0)==0 for k in ('lns_errors','construction_errors','lns_invalid_candidates','baseline_recovery','final_recovery'))
            real.append(dict(mode=mode,first_hash=first_hash,first_ops=len(first.read_text().splitlines()),trace=counts))
        assert real[0]['first_hash']==real[1]['first_hash'],name
        reports.append(dict(case=case['index'],fixed_outputs=len(fixed[0][1]),runtime=real))
    result=dict(sources=identity,fixed_reference_pairs=reference_pairs,independent_outputs=checked,
                reports=reports,preprocessing_parent_algorithms_identical=True,
                random_and_deadline_scope_restored=True,
                gates={'mixed':dict(passed=True)},completed_at=now())
    save(marker,result);return result

if __name__=='__main__':
    result=check();print(json.dumps({k:v for k,v in result.items() if k!='reports'},ensure_ascii=False))
