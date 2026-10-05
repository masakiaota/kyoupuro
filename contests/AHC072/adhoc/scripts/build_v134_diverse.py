from pathlib import Path
from v089_data import sha
ROOT=Path(__file__).resolve().parents[2]
RUN=ROOT/'results/nn_rank/v134/20261005_prefix_diversity_studio'
BASE=ROOT/'src/bin/v113_integrated_nn_lns.cpp'
SOURCE=ROOT/'src/bin/v134_prefix_diversity.cpp'
BASE_SHA='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'

def build():
    assert sha(BASE)==BASE_SHA
    s=BASE.read_text().replace('// v113_integrated_nn_lns.cpp','// v134_prefix_diversity.cpp',1)
    s=s.replace('// v113: v111 policy and incremental CNN; fixed integration HR.',
                '// v134: two greedy starts and two starts with a sampled early prefix.',1)
    old='    bool deadline=false, exhausted=false;\n};\n\ntemplate<class Timer>\nSearchResult search('
    new='    bool deadline=false, exhausted=false;\n    int prefix_steps=0, prefix_reorders=0, last_prefix_depth=0;\n};\n\ntemplate<class Timer>\nSearchResult search('
    assert s.count(old)==1;s=s.replace(old,new,1)
    old='const BoardState& initial,Timer& timer) {'
    assert s.count(old)==1;s=s.replace(old,'const BoardState& initial,Timer& timer,bool diversify_prefix=false) {',1)
    old='        for(int index:order){\n            next=state;next.apply(board,moves[index]);'
    new='''        // Branch only early and only among high-ranked moves. The later greedy
        // completion and visited-state filter remain unchanged. This RNG belongs
        // to the NN search, so it does not consume the LNS random stream.
        if(diversify_prefix && depth<32 && order.size()>1) {
            const int count=min<int>(4,order.size()), greedy=order.front();
            array<pair<double,int>,4> sampled;
            for(int i=0;i<count;++i) {
                const double u=max(uniform(),0x1p-53);
                sampled[i]={double(logits[order[i]])/0.35-log(-log(u)),order[i]};
            }
            stable_sort(sampled.begin(),sampled.begin()+count,
                        [](const auto& a,const auto& b){return a.first>b.first;});
            for(int i=0;i<count;++i)order[i]=sampled[i].second;
            ++result.prefix_steps;result.last_prefix_depth=depth+1;
            result.prefix_reorders+=(order.front()!=greedy);
        }
        for(int index:order){
            next=state;next.apply(board,moves[index]);'''
    assert s.count(old)==1;s=s.replace(old,new,1)
    old='        const auto result=nn::search(transformed,board,initial,timer);'
    new='''        const bool diverse=(sym==1 || sym==5);
        const auto result=nn::search(transformed,board,initial,timer,diverse);
#ifdef LOCAL
        if(diverse) {
            trace.count_by("diverse_attempted",1);
            trace.count_by("diverse_completed",result.E==0);
            trace.count_by("diverse_prefix_steps",result.prefix_steps);
            trace.count_by("diverse_prefix_reorders",result.prefix_reorders);
            if(result.last_prefix_depth>32)throw logic_error("prefix limit exceeded");
        }
#endif'''
    assert s.count(old)==1;s=s.replace(old,new,1)
    if SOURCE.exists():assert SOURCE.read_text()==s,'do not overwrite modified source'
    else:SOURCE.write_text(s)
    return SOURCE

if __name__=='__main__':print(build())
