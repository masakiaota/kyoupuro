#!/usr/bin/env python3
"""評価済みの経路制限を、全体のLNS予算の後半だけ有効にする。"""
from v089_data import ROOT, sha, save

RUN=ROOT/'results/nn_rank/v125/20261004_late_corridor_studio'
BASE=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
CORRIDOR=ROOT/'src/bin/v115_corridor_reinsertion.cpp'


def build():
    assert sha(BASE)=='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
    assert sha(CORRIDOR)=='00c649abb06fa8f46927ba50e5f3b7f000c806c0029bbbec683e79712d04387e'
    text=CORRIDOR.read_text()
    helper='''// 全候補共通のLNS予算を基準にし、候補ごとの中断で切替時刻をずらさない。
static bool late_corridor(double now,double begin,double end) {
    return now>=begin+0.5*(end-begin);
}
'''
    text=text.replace('struct CorridorStats {',helper+'\nstruct CorridorStats {',1)
    needle='                corridor_enabled=iteration%4!=0;';assert text.count(needle)==1
    text=text.replace(needle,'''                const bool corridor_late=late_corridor(time_keeper.exact_elapsed_sec(),lns_start,end);
                corridor_enabled=corridor_late&&iteration%4!=0;
                LOCAL_NOTE(trace.count_by(corridor_late?(corridor_enabled?"corridor_late_limited":"corridor_late_full"):"corridor_early_full",1);)
''',1)
    target=ROOT/'adhoc/bin/v125_late_corridor.cpp';target.write_text('// '+target.name+'\n'+text.split('\n',1)[1])
    RUN.mkdir(parents=True,exist_ok=True)
    save(RUN/'source.json',dict(source=str(target.relative_to(ROOT)),source_sha256=sha(target),base_sha256=sha(BASE),corridor_sha256=sha(CORRIDOR),switch_fraction=.5))
    return target


if __name__=='__main__':print(build())
