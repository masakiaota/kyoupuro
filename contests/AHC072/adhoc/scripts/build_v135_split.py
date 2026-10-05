from pathlib import Path
from v089_data import sha
ROOT=Path(__file__).resolve().parents[2]
RUN=ROOT/'results/nn_rank/v135/20261005_split_rewrite_studio'
BASE=ROOT/'src/bin/v322_sparse_plan_rewriting.cpp'
CURRENT=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
SOURCE=ROOT/'src/bin/v135_split_rewrite.cpp'
BASE_SHA='abb46ec1d213e38840fd3780d9fdfc512f17383faa9b5234070d809bb9789fd9'
CURRENT_SHA='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
def build():
    assert sha(BASE)==BASE_SHA and sha(CURRENT)==CURRENT_SHA
    s=BASE.read_text()
    def replace(a,b):
        nonlocal s
        assert s.count(a)==1,(a[:100],s.count(a));s=s.replace(a,b,1)
    replace('// v322_sparse_plan_rewriting.cpp','// v135_split_rewrite.cpp\n// v135: also start sparse rewriting by changing the first split.')
    replace('struct SparseRewriteStats {','struct SparseRewriteStats {\n    uint64_t split_starts=0,split_completed=0,split_saved=0;')
    replace('        cerr<<"v322_attempts="','        cerr<<"v135_split_starts="<<split_starts<<" v135_split_completed="<<split_completed\n            <<" v135_split_saved="<<split_saved<<\'\\n\';\n        cerr<<"v322_attempts="')
    replace('            trace.count_by("sparse_rewrite_attempts",attempts);','            trace.count_by("split_rewrite_starts",split_starts);\n            trace.count_by("split_rewrite_completed",split_completed);\n            trace.count_by("split_rewrite_saved",split_saved);\n            trace.count_by("sparse_rewrite_attempts",attempts);')
    replace('class SparsePlanRewriter {','class SparsePlanRewriter {\n    friend struct SplitRewriteProbe;')
    replace('        ++sparse_rewrite_stats.completed;sparse_rewrite_stats.raw_saved+=node.skipped;',
    '''        // Every accepted reconnection must remove at least one command,
        // including paths that began by moving a different number of slimes.
        if(node.skipped<=0)throw logic_error("non-shortening sparse rewrite");
        if(node.edits[0].move.p>=0) {
            ++sparse_rewrite_stats.split_completed;
            sparse_rewrite_stats.split_saved+=node.skipped;
        }
        ++sparse_rewrite_stats.completed;sparse_rewrite_stats.raw_saved+=node.skipped;''')
    replace('                if(child.n==0){finish(original,child,first,t,reference,result);return true;}',
    '''                if(child.n==0) {
                    if(child.skipped>0){finish(original,child,first,t,reference,result);return true;}
                    // An equal-length path already rejoined the original state.
                    // It offers no active difference for later repair.
                    return false;
                }''')
    replace('                if(propose(active[0],Move{-1,0,0,0}))return true;',
    '''                if(propose(active[0],Move{-1,0,0,0}))return true;
                // Keep both endpoints, but allow a different payload from the
                // outset. Later commands must be deleted to earn a shorter plan.
                const int hs=height(old_p),hd=height(old_q);
                for(int k=max(int(op.l)-1,hs+hd-8);k<hs;k++)if(k!=op.k) {
                    ++sparse_rewrite_stats.split_starts;
                    Move alternative=op;alternative.k=uint8_t(k);
                    if(propose(active[0],alternative))return true;
                }''')
    if SOURCE.exists():assert SOURCE.read_text()==s,'source changed'
    else:SOURCE.write_text(s)
    return SOURCE
if __name__=='__main__':print(build())
