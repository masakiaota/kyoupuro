#!/usr/bin/env python3
"""Build the frozen v323 experiment from the exact evaluated v322 source."""
from pathlib import Path
import hashlib
ROOT = Path(__file__).resolve().parents[2]
p = ROOT / 'src/bin/v322_sparse_plan_rewriting.cpp'
s = p.read_text()
assert hashlib.sha256(s.encode()).hexdigest() == 'abb46ec1d213e38840fd3780d9fdfc512f17383faa9b5234070d809bb9789fd9'
def replace(old, new):
    global s
    assert s.count(old) == 1, (old[:100], s.count(old))
    s = s.replace(old, new, 1)
replace('// v322_sparse_plan_rewriting.cpp', '''// v323_retargeted_plan_rewriting.cpp
// v323: repair both endpoints of affected commands, parent v322.
// Candidate moves stay inside the original endpoints plus changed cells.
// Same learned weights, beam and edit budgets, LNS, racing and clock.
// See notes/experiments/v323.md; no validation1 use.''')
replace('    double seconds=0;\n    void summary() const {\n        cerr<<"v322_attempts="', '''    uint64_t retarget_generated=0,retargeted=0,retarget_completed=0;
    double seconds=0;
    void summary() const {
        cerr<<"v323_retarget_generated="<<retarget_generated
            <<" v323_retargeted="<<retargeted
            <<" v323_retarget_completed="<<retarget_completed<<'\\n';
        cerr<<"v322_attempts="''')
replace('            trace.count_by("sparse_rewrite_attempts",attempts);', '''            trace.count_by("sparse_rewrite_retarget_generated",retarget_generated);
            trace.count_by("sparse_rewrite_retargeted",retargeted);
            trace.count_by("sparse_rewrite_retarget_completed",retarget_completed);
            trace.count_by("sparse_rewrite_attempts",attempts);''')
replace('        int at=0;\n        for(int t=0;t<int(original.size());t++) {', '''        int at=0;bool used_retarget=false;
        for(int t=0;t<int(original.size());t++) {''')
replace('                    sparse_rewrite_stats.relocated+=m.p!=original[t].p;}', '''                    sparse_rewrite_stats.relocated+=m.p!=original[t].p;
                    const Move old=original[t];
                    const bool retarget=board_info.ray[m.p][m.d][m.l]!=board_info.ray[old.p][old.d][old.l];
                    sparse_rewrite_stats.retargeted+=retarget;used_retarget|=retarget;}''')
replace('        ++sparse_rewrite_stats.completed;sparse_rewrite_stats.raw_saved+=node.skipped;', '''        sparse_rewrite_stats.retarget_completed+=used_retarget;
        ++sparse_rewrite_stats.completed;sparse_rewrite_stats.raw_saved+=node.skipped;''')
replace('                if(propose(from,Move{-1,0,0,0}))return true;', '''                if(propose(from,Move{-1,0,0,0}))return true;
                // Keep every old fixed-destination proposal above. The new
                // proposals also relocate the meeting point. Only the two
                // original endpoints and changed cells can participate, so
                // the temporary union contains at most six cells. Returning
                // to a nest is handled by transition, including source return.
                array<int,6> endpoints{};int endpoint_count=0;
                for(int j=0;j<count;j++)endpoints[endpoint_count++]=sources[j];
                bool has_q=false;for(int j=0;j<endpoint_count;j++)has_q|=endpoints[j]==q;
                if(!has_q)endpoints[endpoint_count++]=q;
                array<int,6> heights{};
                for(int j=0;j<endpoint_count;j++)
                    heights[j]=height(actual(from,reference,endpoints[j],op.p,q,old_p,old_q));
                for(int b=0;b<endpoint_count;b++) {
                    const int dest=endpoints[b];if(dest==q)continue;
                    // Check once per destination, independently of candidate
                    // count; invalid pairs must not postpone the time check.
                    if(time_keeper.exact_elapsed_sec()>=deadline){++sparse_rewrite_stats.deadlines;return false;}
                    for(int a=0;a<endpoint_count;a++) {
                        const int p=endpoints[a],hs=heights[a],hd=heights[b];
                        if(a==b||hs==0)continue;
                        int d=-1,l=0;
                        if(board_info.row[p]==board_info.row[dest]){d=board_info.col[dest]>board_info.col[p]?3:2;l=abs(board_info.col[p]-board_info.col[dest]);}
                        else if(board_info.col[p]==board_info.col[dest]){d=board_info.row[dest]>board_info.row[p]?1:0;l=abs(board_info.row[p]-board_info.row[dest]);}
                        if(d<0||l<1||l>8||board_info.ray[p][d][l]!=dest)continue;
                        for(int k=max(l-1,hs+hd-8);k<hs;k++) {
                            ++sparse_rewrite_stats.retarget_generated;
                            if(propose(from,Move{int16_t(p),uint8_t(k),uint8_t(d),uint8_t(l)}))return true;
                        }
                    }
                }''')
# Rename only this experiment's diagnostic prefix, not historical comments.
s=s.replace('"v322_', '"v323_').replace('" v322_', '" v323_')
out=ROOT/'src/bin/v323_retargeted_plan_rewriting.cpp'
out.write_text(s)
print(out, len(s.encode()), hashlib.sha256(s.encode()).hexdigest())
