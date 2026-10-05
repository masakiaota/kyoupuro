// v322: delete a command and repair its consequences in the colored state.
// The footprint is allowed to move between cells and to change tower heights.
// It is not a set of individual slime identities or a fixed geometric window.
struct SparseRewriteStats {
    uint64_t attempts=0,events=0,affected=0,independent=0,generated=0;
    uint64_t footprint_caps=0,edit_caps=0,width_caps=0,duplicates=0,deadlines=0;
    uint64_t completed=0,raw_saved=0,replaced=0,relocated=0,long_spans=0,max_span=0;
    uint64_t accepted=0,improvements=0,best_saved=0,height_differences=0;
    double seconds=0;
    void summary() const {
        cerr<<"v322_attempts="<<attempts<<" v322_events="<<events
            <<" v322_affected="<<affected<<" v322_independent="<<independent
            <<" v322_generated="<<generated<<" v322_footprint_caps="<<footprint_caps
            <<" v322_edit_caps="<<edit_caps<<" v322_width_caps="<<width_caps
            <<" v322_duplicates="<<duplicates<<" v322_deadlines="<<deadlines
            <<" v322_completed="<<completed<<" v322_raw_saved="<<raw_saved
            <<" v322_replaced="<<replaced<<" v322_relocated="<<relocated
            <<" v322_long_spans="<<long_spans<<" v322_max_span="<<max_span
            <<" v322_height_differences="<<height_differences
            <<" v322_accepted="<<accepted<<" v322_improvements="<<improvements
            <<" v322_best_saved="<<best_saved<<" v322_seconds="<<seconds<<'\n';
        LOCAL_ONLY(
            trace.count_by("sparse_rewrite_attempts",attempts);
            trace.count_by("sparse_rewrite_completed",completed);
            trace.count_by("sparse_rewrite_saved",raw_saved);
            trace.count_by("sparse_rewrite_accepted",accepted);
            trace.count_by("sparse_rewrite_best_saved",best_saved);
            trace.add_time_ms("sparse_rewrite",seconds*1000.0);
        );
    }
} sparse_rewrite_stats;

class SparsePlanRewriter {
    static constexpr int footprint=4,width=8,max_edits=12,max_skips=3,max_span=256;
    struct Edit { int time=0; Move move{}; }; // p=-1 denotes an omitted command.
    struct Node {
        array<uint64_t,footprint> difference{}; // sorted (cell << 32) | color word
        array<Edit,max_edits> edits{};
        int n=0,edit_count=0,skipped=0,priority=0;
    };
    array<Node,width> active{},next{};
    int active_count=0,next_count=0;
    static bool same_move(Move a,Move b) {
        return a.p==b.p&&a.k==b.k&&a.d==b.d&&a.l==b.l;
    }
    static int cell(uint64_t v){return int(v>>32);}
    static bool touches(const Node& node,int p) {
        for(int i=0;i<node.n;i++)if(cell(node.difference[i])==p)return true;
        return false;
    }
    static bool preferred(const Node& a,const Node& b) {
        if(a.priority!=b.priority)return a.priority<b.priority;
        if(a.skipped!=b.skipped)return a.skipped>b.skipped;
        if(a.edit_count!=b.edit_count)return a.edit_count<b.edit_count;
        return a.difference<b.difference;
    }
    static TowerBits actual(const Node& node,const State& after,int p,
                            int op_p,int op_q,TowerBits old_p,TowerBits old_q) {
        for(int i=0;i<node.n;i++)if(cell(node.difference[i])==p)return TowerBits(node.difference[i]);
        return p==op_p?old_p:p==op_q?old_q:after[p];
    }
    // Compare against the original plan AFTER its current command. Check the
    // candidate command before normalization, so a nest cannot hide overflow.
    static bool transition(const Node& from,Node& out,const State& after,
                           Move original,int q,TowerBits old_p,TowerBits old_q,
                           Move replacement,int t) {
        ++sparse_rewrite_stats.generated;
        const bool skip=replacement.p<0;
        const bool edited=skip||!same_move(original,replacement);
        if((skip&&from.skipped>=max_skips)||(edited&&from.edit_count>=max_edits)) {
            ++sparse_rewrite_stats.edit_caps;return false;
        }
        int source=-1,destination=-1;TowerBits source_after=0,destination_after=0;
        if(!skip) {
            source=replacement.p;
            if(source<0||source>=board_info.cell_count||replacement.d>=4||
               replacement.l<1||replacement.l>8||replacement.l>replacement.k+1)return false;
            destination=board_info.ray[source][replacement.d][replacement.l];
            if(destination<0)return false;
            const TowerBits s=actual(from,after,source,original.p,q,old_p,old_q);
            const TowerBits d=actual(from,after,destination,original.p,q,old_p,old_q);
            const int hs=height(s),hd=height(d),k=replacement.k;
            if(k>=hs||hd+hs-k>8)return false;
            source_after=board_info.normalize(prefix(s,k),source);
            destination_after=board_info.normalize(append_tower(d,reverse_tower(s>>(4*k))),destination);
        }
        out=from;out.n=0;out.difference.fill(0);out.priority=0;
        array<int,8> cells{};int count=0;
        auto add=[&](int p) {
            if(p<0)return;
            for(int i=0;i<count;i++)if(cells[i]==p)return;
            cells[count++]=p;
        };
        for(int i=0;i<from.n;i++)add(cell(from.difference[i]));
        add(original.p);add(q);add(source);add(destination);
        // At most eight integers; insertion sort avoids library sort overhead.
        for(int i=1;i<count;i++){int x=cells[i],j=i;while(j&&x<cells[j-1]){cells[j]=cells[j-1];--j;}cells[j]=x;}
        bool different_height=false;
        for(int i=0;i<count;i++) {
            const int p=cells[i];
            TowerBits w=actual(from,after,p,original.p,q,old_p,old_q);
            if(p==source)w=source_after;
            if(p==destination)w=destination_after;
            if(w==after[p])continue;
            if(out.n==footprint){++sparse_rewrite_stats.footprint_caps;return false;}
            out.difference[out.n++]=(uint64_t(p)<<32)|w;
            const int common=common_tower_prefix(w,after[p]);
            out.priority+=8*(height(w)+height(after[p])-2*common)+2;
            different_height|=height(w)!=height(after[p]);
        }
        if(edited)out.edits[out.edit_count++]={t,replacement};
        out.skipped+=skip;
        // A beam ranking only: unmatched colors are NOT an admissible bound.
        out.priority+=out.edit_count-2*out.skipped;
        sparse_rewrite_stats.height_differences+=different_height;
        return true;
    }
    void offer(const Node& candidate) {
        for(int i=0;i<next_count;i++) {
            // Same event, board AND remaining edit/skip budgets are required.
            // Equal boards with different resources are not silently merged.
            if(next[i].n==candidate.n&&next[i].difference==candidate.difference&&
               next[i].skipped==candidate.skipped&&next[i].edit_count==candidate.edit_count) {
                ++sparse_rewrite_stats.duplicates;return;
            }
        }
        int at=next_count;
        while(at>0&&preferred(candidate,next[at-1]))--at;
        if(at>=width){++sparse_rewrite_stats.width_caps;return;}
        if(next_count==width)++sparse_rewrite_stats.width_caps;
        else ++next_count;
        for(int i=next_count-1;i>at;i--)next[i]=next[i-1];
        next[at]=candidate;
    }
    static void finish(const vector<Move>& original,const Node& node,int first,int last,
                       const State& boundary,vector<Move>& result) {
        result.clear();result.reserve(original.size()-node.skipped);
        int at=0;
        for(int t=0;t<int(original.size());t++) {
            if(at<node.edit_count&&node.edits[at].time==t) {
                const Move m=node.edits[at++].move;
                if(m.p>=0){result.push_back(m);++sparse_rewrite_stats.replaced;
                    sparse_rewrite_stats.relocated+=m.p!=original[t].p;}
            } else result.push_back(original[t]);
        }
        if(at!=node.edit_count||int(result.size())!=int(original.size())-node.skipped)
            throw logic_error("sparse rewrite edit chronology");
        State checked=board_info.initial;int index=0;
        // All modifications end at last. Exact full-board equality is checked
        // there before relying on the unchanged suffix of the original plan.
        const int prefix_length=last+1-node.skipped;
        for(;index<prefix_length;index++)board_info.apply(checked,result[index]);
        if(!(checked==boundary))throw logic_error("sparse rewrite boundary mismatch");
        for(;index<int(result.size());index++)board_info.apply(checked,result[index]);
        if(board_info.count(checked))throw logic_error("sparse rewrite incomplete");
        ++sparse_rewrite_stats.completed;sparse_rewrite_stats.raw_saved+=node.skipped;
        sparse_rewrite_stats.long_spans+=last-first+1>64;
        sparse_rewrite_stats.max_span=max(sparse_rewrite_stats.max_span,uint64_t(last-first+1));
    }
public:
    bool run(const vector<Move>& original,int first,double deadline,vector<Move>& result) {
        ++sparse_rewrite_stats.attempts;
        if(first<0||first>=int(original.size()))return false;
        State reference=board_info.initial;
        for(int t=0;t<first;t++) {
            if((t&31)==0&&time_keeper.exact_elapsed_sec()>=deadline){++sparse_rewrite_stats.deadlines;return false;}
            board_info.apply(reference,original[t]);
        }
        active_count=1;active[0]=Node{};
        const int stop=min(int(original.size()),first+max_span);
        for(int t=first;t<stop&&active_count;t++) {
            if(time_keeper.exact_elapsed_sec()>=deadline){++sparse_rewrite_stats.deadlines;return false;}
            const Move op=original[t];const int q=board_info.ray[op.p][op.d][op.l];
            const TowerBits old_p=reference[op.p],old_q=reference[q];
            board_info.apply(reference,op);++sparse_rewrite_stats.events;next_count=0;
            auto propose=[&](const Node& from,Move m) {
                Node child;
                if(!transition(from,child,reference,op,q,old_p,old_q,m,t))return false;
                if(child.n==0){finish(original,child,first,t,reference,result);return true;}
                offer(child);return false;
            };
            if(t==first) {
                if(propose(active[0],Move{-1,0,0,0}))return true;
            } else for(int i=0;i<active_count;i++) {
                const Node& from=active[i];
                if(!touches(from,op.p)&&!touches(from,q)) {
                    ++sparse_rewrite_stats.independent;offer(from);continue;
                }
                ++sparse_rewrite_stats.affected;
                if(propose(from,op))return true;
                array<int,5> sources{};int count=0;sources[count++]=op.p;
                for(int j=0;j<from.n;j++)if(cell(from.difference[j])!=op.p)sources[count++]=cell(from.difference[j]);
                for(int j=0;j<count;j++) {
                    if((sparse_rewrite_stats.generated&63)==0&&time_keeper.exact_elapsed_sec()>=deadline){++sparse_rewrite_stats.deadlines;return false;}
                    const int p=sources[j];if(p==q)continue;
                    int d=-1,l=0;
                    if(board_info.row[p]==board_info.row[q]){d=board_info.col[q]>board_info.col[p]?3:2;l=abs(board_info.col[p]-board_info.col[q]);}
                    else if(board_info.col[p]==board_info.col[q]){d=board_info.row[q]>board_info.row[p]?1:0;l=abs(board_info.row[p]-board_info.row[q]);}
                    if(d<0||l<1||l>8||board_info.ray[p][d][l]!=q)continue;
                    const int hs=height(actual(from,reference,p,op.p,q,old_p,old_q));
                    const int hd=height(actual(from,reference,q,op.p,q,old_p,old_q));
                    for(int k=max(l-1,hs+hd-8);k<hs;k++) {
                        Move m{int16_t(p),uint8_t(k),uint8_t(d),uint8_t(l)};
                        if(!same_move(m,op)&&propose(from,m))return true;
                    }
                }
                if(propose(from,Move{-1,0,0,0}))return true;
            }
            active_count=next_count;
            for(int i=0;i<active_count;i++)active[i]=next[i];
        }
        return false;
    }
};

