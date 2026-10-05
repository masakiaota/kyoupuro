#!/usr/bin/env python3
"""Assemble v322 from the exact v113 parent and the new local rewriter block."""
from pathlib import Path
import hashlib
ROOT=Path(__file__).resolve().parents[2]
s=(ROOT/'src/bin/v113_integrated_nn_lns.cpp').read_text()
assert hashlib.sha256(s.encode()).hexdigest()=='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
def replace(a,b):
    global s
    assert s.count(a)==1,(a[:100],s.count(a))
    s=s.replace(a,b,1)
replace('// v113_integrated_nn_lns.cpp','// v322_sparse_plan_rewriting.cpp\n// v322: sparse colored-state plan rewriting; direct parent v113 at 7e5c9ba.\n// No weight changes. See notes/experiments/v322.md for the frozen design.')
replace('class TemporalLNS {',(ROOT/'adhoc/v322/sparse_rewriter.inc').read_text()+'class TemporalLNS {')
replace('    double paired_seconds=0;','    double rewrite_seconds=0;\n    double paired_seconds=0;')
replace('        paired_seconds+=other.paired_seconds;','        rewrite_seconds+=other.rewrite_seconds;\n        paired_seconds+=other.paired_seconds;')
replace('            bool paired=!reorder&&iteration%17==0&&','''            const bool rewriting=!reorder&&iteration%29==0&&
                rewrite_seconds<0.06*max(PROGRAM_TIME_LIMIT_SEC*(0.03/1.90),active_seconds+time_keeper.exact_elapsed_sec()-slice_begin);
            bool paired=!reorder&&!rewriting&&iteration%17==0&&''')
replace('const bool packet_slot=!reorder&&!paired&&iteration%7==0&&','const bool packet_slot=!reorder&&!rewriting&&!paired&&iteration%7==0&&')
replace('            else if(!reorder)ensure_history(current);','            else if(!reorder&&!rewriting)ensure_history(current);')
replace('bool dependency=!reorder&&!paired&&!packet&&++regular_selections%dependency_interval==0;','bool dependency=!reorder&&!rewriting&&!paired&&!packet&&++regular_selections%dependency_interval==0;')
replace('''            }else if(paired) {
                ++attempts;++paired_attempts;''','''            }else if(rewriting) {
                ++attempts;
                const double begin=time_keeper.exact_elapsed_sec();
                const double deadline=min({slice_end,end,time_keeper.time_limit_sec,begin+0.00075*PROGRAM_TIME_LIMIT_SEC});
                SparsePlanRewriter rewriter;
                ok=rewriter.run(current,rng(int(current.size())),deadline,trial);
                const double spent=time_keeper.exact_elapsed_sec()-begin;
                rewrite_seconds+=spent;sparse_rewrite_stats.seconds+=spent;
            }else if(paired) {
                ++attempts;++paired_attempts;''')
replace('''                ++accepted;uphill+=delta>0;''','''                sparse_rewrite_stats.accepted+=rewriting;
                ++accepted;uphill+=delta>0;''')
replace('''                    int saved=int(best.size()-polished.size());''','''                    int saved=int(best.size()-polished.size());
                    if(rewriting){++sparse_rewrite_stats.improvements;sparse_rewrite_stats.best_saved+=saved;}''')
replace('!packet && !reorder && !paired ? saved : 0','!packet && !reorder && !rewriting && !paired ? saved : 0')
replace('    cerr<<"baseline="<<baseline','    sparse_rewrite_stats.summary();\n    cerr<<"baseline="<<baseline')
p=ROOT/'src/bin/v322_sparse_plan_rewriting.cpp';p.write_text(s)
print(p,len(s.encode()),hashlib.sha256(s.encode()).hexdigest())
