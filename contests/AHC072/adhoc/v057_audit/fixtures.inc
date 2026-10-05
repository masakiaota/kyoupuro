// check_v055_final_reductions.cpp
// Check the new transformation on fixed boundaries, never run the full solver.
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wreturn-type"
#define AHC072_COUPLED_AUDIT
#define AHC072_FINAL_AUDIT
#define main unused_solver_main
#include "../../src/bin/v057_search_reductions.cpp"
#undef main
#pragma GCC diagnostic pop

void initialize(istream& input) {
    assert(board_pool.free_count==0||board_pool.free_count==BoardPool::slots);
    auto previous=cin.rdbuf(input.rdbuf());cin.clear();
    geo.V=geo.M=0;geo.initial.fill(0);memset(geo.rayCount,0,sizeof(geo.rayCount));geo.read();
    cin.rdbuf(previous);cin.clear();board_pool.prepare(geo.V);
}
vector<Move> read_moves(const filesystem::path& path) {
    ifstream input(path);if(!input)throw runtime_error("missing answer: "+path.string());
    vector<Move> result;int i,j,k,l;char d;
    while(input>>i>>j>>k>>d>>l)result.push_back({int16_t(geo.id[i][j]),uint8_t(k),uint8_t(find(geo.dirs,geo.dirs+4,d)-geo.dirs),uint8_t(l)});
    return result;
}
void write_moves(const vector<Move>& moves,const filesystem::path& path) {
    ofstream output(path);
    for(auto m:moves)output<<geo.row[m.p]<<' '<<geo.col[m.p]<<' '<<int(m.k)<<' '<<geo.dirs[m.d]<<' '<<int(m.l)<<'\n';
}
using Reference=array<vector<int>,MV>;
Reference unpack(const Board& board) {
    Reference result;
    for(int p=0;p<geo.V;p++)for(Word w=board[p];w;w>>=4)result[p].push_back(w&15);
    return result;
}
bool apply_reference(Reference& towers,Move m) {
    if(m.p<0||m.p>=geo.V||m.d>=4||m.k>=towers[m.p].size()||m.l<1||m.l>m.k+1)return false;
    int q=-1;
    for(int step=1;step<=m.l;step++) {
        int i=geo.row[m.p]+geo.di[m.d]*step,j=geo.col[m.p]+geo.dj[m.d]*step;
        if(i<0||i>=geo.N||j<0||j>=geo.N||geo.id[i][j]<0)return false;
        q=geo.id[i][j];
    }
    if(towers[q].size()+towers[m.p].size()-m.k>8)return false;
    while(towers[m.p].size()>m.k){towers[q].push_back(towers[m.p].back());towers[m.p].pop_back();}
    for(int p:{int(m.p),q})while(!towers[p].empty()&&towers[p].back()==geo.nest[p])towers[p].pop_back();
    return true;
}
Word repeated(int color,int amount){return prefix(0x11111111u*color,amount);}

int referenceStrictTransferWindow(const vector<Move>& answer,int begin,Board& board,
                         vector<Move>& output,int& saved,double deadline) {
    saved=0;
    const int stop=min(int(answer.size()),begin+64);
    Board original=board;int goal=-1;
    int end=consumeTransfer(answer,begin,stop,original,goal);
    if(end-begin<2)return begin;
    const Move first=answer[begin];
    // A palindrome has no unresolved orientation to carry through later events.
    const Word packet=board[first.p]>>(4*first.k);
    if(packet==reverseWord(packet))return begin;
    PortionRouter router;router.run(board,first.p,first.k,goal,end-begin-1);++coupled_routes.routes;
    vector<RouteVariant> active;
    for(int o=0;o<2;o++) {
        int length=router.cost(goal,o);if(length<0||length>=end-begin)continue;
        Board candidate=board;auto path=router.path(goal,o);bool legal=true;
        try{for(auto m:path)geo.apply(candidate,m);}catch(const exception&){legal=false;}
        if(legal&&routeDifference(candidate,original)>0)active.push_back(routeVariant(candidate,move(path)));
    }
    if(active.empty())return begin;
    ++coupled_routes.seeds;
    for(int depth=1;depth<8&&end<stop;depth++) {
        if(clk.elapsed()>=deadline){++coupled_routes.deadlines;return begin;}
        array<Word,MV> before{};for(int p=0;p<geo.V;p++)before[p]=original[p];
        const int start=end;const Move next=answer[start];
        end=consumeTransfer(answer,start,stop,original,goal);
        vector<RouteVariant> children;
        for(const auto& variant:active) {
            if(clk.elapsed()>=deadline){++coupled_routes.deadlines;return begin;}
            Board working=variant.words;
            bool touches=false;
            for(int i=start;i<end;i++) {
                const Move m=answer[i];const int q=geo.ray[m.p][m.d][m.l];
                touches|=before[m.p]!=variant.words[m.p]||before[q]!=variant.words[q];
            }
            // Unrelated operations need no new path search and keep their order.
            if(!touches) {
                auto path=variant.moves;
                for(int i=start;i<end;i++){geo.apply(working,answer[i]);path.push_back(answer[i]);}
                children.push_back(routeVariant(working,move(path)));++coupled_routes.independent;
                continue;
            }
            router.run(working,next.p,next.k,goal,end-start);++coupled_routes.routes;
            for(int o=0;o<2;o++) {
                const int length=router.cost(goal,o);if(length<0||length>end-start)continue;
                auto path=router.path(goal,o);Board candidate=working;bool legal=true;
                try{for(auto m:path)geo.apply(candidate,m);}catch(const exception&){legal=false;}
                if(!legal)continue;
                const int difference=routeDifference(candidate,original);
                if(difference<0)continue;
                vector<Move> complete=variant.moves;
                complete.insert(complete.end(),path.begin(),path.end());++coupled_routes.branches;
                if(difference==0&&int(complete.size())<end-begin) {
                    saved=end-begin-int(complete.size());board=original;
                    output.insert(output.end(),complete.begin(),complete.end());
                    ++coupled_routes.replacements;coupled_routes.saved+=saved;return end;
                }
                children.push_back(routeVariant(candidate,move(complete)));
            }
        }
        if(children.empty())return begin;
        stable_sort(children.begin(),children.end(),[](const auto& a,const auto& b){return a.moves.size()<b.moves.size();});
        active.clear();
        for(auto& child:children) {
            bool duplicate=false;for(const auto& kept:active)duplicate|=child.words==kept.words;
            if(duplicate)continue;
            if(active.size()==4){++coupled_routes.beam_caps;break;}
            active.push_back(move(child));
        }
    }
    ++coupled_routes.depth_caps;return begin;
}

vector<Move> referenceStrictPolishCoupledRoutes(const vector<Move>& answer,double deadline,int& saved) {
    CoupledRoutesTimer timer;++coupled_routes.calls;saved=0;
    Board board=geo.initial;vector<Move> output;output.reserve(answer.size());
    for(int i=0;i<int(answer.size());) {
        // Only proven equal-boundary replacements enter output. At the budget
        // limit the untouched suffix can be appended without another replay.
        if(clk.elapsed()>=deadline) {
            ++coupled_routes.deadlines;
            output.insert(output.end(),answer.begin()+i,answer.end());return output;
        }
        int shorter=0;const int end=referenceStrictTransferWindow(answer,i,board,output,shorter,deadline);
        if(end>i){saved+=shorter;i=end;}
        else{geo.apply(board,answer[i]);output.push_back(answer[i++]);}
    }
    if(geo.count(board))throw logic_error("coupled routes incomplete");
    return output;
}

int strictTransferWindow(const vector<Move>& a,int begin,Board& b,vector<Move>& out,int& saved,double deadline) {return coupleTransferWindow(a,begin,b,out,saved,deadline,false);}
int route_fixtures() {
    string blank="12 4\n";for(int i=0;i<12;i++)blank+=string(12,'.')+'\n';
    istringstream input(blank);initialize(input);
    const int source=geo.id[8][6],mid=geo.id[6][6],last=geo.id[5][7],goal=geo.id[3][7];
    int tests=0;
    for(int amount=2;amount<=7;amount++)for(int bits=0;bits<(1<<amount);bits++) {
        Word packet=0;for(int i=0;i<amount;i++)packet|=Word(2+((bits>>i)&1))<<(4*i);
        if(packet==reverseWord(packet))continue;
        for(int support=1;support+amount<=8;support++)for(int target=0;target+amount<=8;target++)for(int home=0;home<2;home++) {
            array<Word,MV> initial{};
            initial[source]=joinWord(1,packet);initial[mid]=repeated(1,support);initial[goal]=repeated(4,target);
            geo.nest[source]=home?1:0;geo.nest[last]=home?1:0;geo.nest[goal]=home?2:0;
            vector<Move> answer={{int16_t(source),1,0,1},{int16_t(geo.id[7][6]),0,0,1},
                                {int16_t(mid),0,0,1},{int16_t(geo.id[5][6]),0,3,1},
                                {int16_t(last),uint8_t(support),0,1},{int16_t(geo.id[4][7]),0,0,1}};
            Board board=initial;auto expected=unpack(board),before=expected;
            for(auto m:answer)assert(apply_reference(expected,m));
            vector<Move> replacement;int saved=0;
            int end=coupleTransferWindow(answer,0,board,replacement,saved,numeric_limits<double>::infinity());
            assert(end==6&&saved>=2&&replacement.size()<=4);
            for(auto m:replacement)assert(apply_reference(before,m));
            assert(before==expected&&unpack(board)==expected);++tests;
        }
    }
    geo.nest[source]=geo.nest[last]=geo.nest[goal]=0;
    array<Word,MV> initial{};initial[source]=joinWord(1,0x2333322);initial[mid]=1;initial[goal]=2;
    const vector<Move> answer={{int16_t(source),1,0,1},{int16_t(geo.id[7][6]),0,0,1},
                              {int16_t(mid),0,0,1},{int16_t(geo.id[5][6]),0,3,1},
                              {int16_t(last),1,0,1},{int16_t(geo.id[4][7]),0,0,1}};
    for(int mode=0;mode<3;mode++) {
        Board board=initial;auto before=unpack(board);auto input_moves=answer;
        if(mode==0)input_moves.resize(5); // unresolved orientation at the endpoint
        vector<Move> replacement;int saved=0;
        if(mode==2) { // no actual orientation difference to propagate
            board[source]=joinWord(1,repeated(2,7));before=unpack(board);
        }
        assert(coupleTransferWindow(input_moves,0,board,replacement,saved,mode==1?-1:numeric_limits<double>::infinity())==0);
        assert(saved==0&&replacement.empty()&&unpack(board)==before);++tests;
    }
    geo.initial=initial;
    int saved=0;auto unchanged=polishCoupledRoutes(answer,-1,saved);
    assert(unchanged.size()==answer.size()&&saved==0);
    for(size_t i=0;i<answer.size();i++)assert(unchanged[i].p==answer[i].p&&unchanged[i].k==answer[i].k&&unchanged[i].d==answer[i].d&&unchanged[i].l==answer[i].l);
    return tests+1;
}


struct Patch {
    array<Word,MV> initial{};
    int begin,end,peak;
    bool relaxed;
    vector<Move> moves;
    vector<pair<int,int>> boundaries;
};
vector<Patch> patches;
bool collect_patches=false;
int boundary_checks=0,patch_checks=0;
void auditCoupledRoute(const Board& initial,const vector<Move>& answer,int begin,int end,const RouteVariant& child) {
    Reference old=unpack(initial),now=old;
    int a=begin,b=0,peak=0;bool relaxed=false;
    assert(child.boundaries.size()>=2&&child.boundaries.size()<=8&&end-begin<=64);
    for(auto [next_a,next_b]:child.boundaries) {
        assert(a<next_a&&b<=next_b&&next_b<=int(child.moves.size()));
        if(b<next_b)assert(child.moves[b].p==answer[a].p&&child.moves[b].k==answer[a].k);
        const int original_length=next_a-a,new_length=next_b-b;
        relaxed|=a==begin?new_length>=original_length:new_length>original_length;
        int expected_goal=-1,actual_goal=answer[a].p;
        for(;a<next_a;a++) {expected_goal=geo.ray[answer[a].p][answer[a].d][answer[a].l];assert(apply_reference(old,answer[a]));}
        for(;b<next_b;b++) {actual_goal=geo.ray[child.moves[b].p][child.moves[b].d][child.moves[b].l];assert(apply_reference(now,child.moves[b]));}
        assert(expected_goal==actual_goal);
        int different=0;
        for(int p=0;p<geo.V;p++) {assert(old[p].size()==now[p].size());different+=old[p]!=now[p];}
        assert(different<=2&&b-(a-begin)<=1);peak=max(peak,b-(a-begin));++boundary_checks;
    }
    assert(a==end&&b==int(child.moves.size())&&b<end-begin&&old==now);
    assert(peak==child.peak_excess&&relaxed==child.relaxed);++patch_checks;
    if(collect_patches) {
        Patch patch;for(int p=0;p<geo.V;p++)patch.initial[p]=initial[p];
        patch.begin=begin;patch.end=end;patch.peak=peak;patch.relaxed=relaxed;
        patch.moves=child.moves;patch.boundaries=child.boundaries;patches.push_back(move(patch));
    }
}

int slack_fixtures() {
    vector<string> rows(12,string(12,'.'));rows[5][9]='#';
    string text="12 4\n";for(const auto& row:rows)text+=row+'\n';
    istringstream input(text);initialize(input);
    const int source=geo.id[8][6],mid=geo.id[6][6],last=geo.id[5][7],goal=geo.id[5][8];
    int tests=0;
    for(int amount=2;amount<=7;amount++)for(int bits=0;bits<(1<<amount);bits++) {
        Word packet=0;for(int i=0;i<amount;i++)packet|=Word(2+((bits>>i)&1))<<(4*i);
        if(packet==reverseWord(packet))continue;
        for(int support=1;support+amount<=8;support++)for(int target=0;target+amount<=8;target++)for(int home=0;home<2;home++) {
            array<Word,MV> initial{};initial[source]=joinWord(1,packet);
            initial[mid]=repeated(1,support);initial[goal]=repeated(4,target);
            geo.nest[source]=home?1:0;geo.nest[goal]=home?2:0;
            const vector<Move> answer={{int16_t(source),1,0,2},{int16_t(mid),0,0,1},
                {int16_t(geo.id[5][6]),0,3,1},{int16_t(last),uint8_t(support),0,2},
                {int16_t(geo.id[3][7]),0,3,1},{int16_t(geo.id[3][8]),0,1,1},
                {int16_t(geo.id[4][8]),0,1,1}};
            Reference expected;
            {Board board=initial;expected=unpack(board);for(auto m:answer)assert(apply_reference(expected,m));}
            {
                Board board=initial;vector<Move> replacement;int saved=0;
                assert(strictTransferWindow(answer,0,board,replacement,saved,numeric_limits<double>::infinity())==0);
                assert(saved==0&&replacement.empty());++tests;
            }
            {
                Board board=initial;vector<Move> replacement;int saved=0;
                auto before=coupled_routes;
                assert(coupleTransferWindow(answer,0,board,replacement,saved,numeric_limits<double>::infinity())==7);
                assert(saved>=2&&unpack(board)==expected);
                assert(coupled_routes.overrun_saved-before.overrun_saved==saved);++tests;
            }
        }
    }
    // Reaching capacity before home returns is illegal, even if the packet
    // would immediately disappear on landing.
    array<Word,MV> initial{};initial[source]=0x221;initial[mid]=repeated(4,7);geo.nest[mid]=2;
    {Board board=initial;auto ref=unpack(board);assert(!apply_reference(ref,{int16_t(source),1,0,2}));++tests;}
    return tests;
}

void json_moves(ostream& out,const vector<Move>& moves) {
    out<<'[';bool first=true;
    for(auto m:moves) {if(!first)out<<',';first=false;
        out<<'['<<geo.row[m.p]<<','<<geo.col[m.p]<<','<<int(m.k)<<",\""<<geo.dirs[m.d]<<"\","<<int(m.l)<<']';}
    out<<']';
}

int mono_fixtures(int& positive,int& negative) {
    string blank="12 4\n";for(int i=0;i<12;i++)blank+=string(12,'.')+'\n';
    istringstream input(blank);initialize(input);
    const int p=geo.id[5][4],r=geo.id[5][5],q=geo.id[5][8];
    int checked=0;
    // Enumerate payload sizes, support heights, destination capacities and
    // returns. The independent dynamic-tower model validates both paths.
    for(int amount=2;amount<=8;amount++)for(int keep=0;keep+amount<=8;keep++)
    for(int sent=1;sent<amount;sent++)for(int relay=2;relay<=8;relay++)
    for(int target=0;target<=8;target++)for(int home=0;home<2;home++) {
        if(keep+amount-sent<3||relay+amount-sent>8)continue;
        if(target+max(sent,amount-sent)>8||(!home&&target+amount>8))continue;
        geo.nest[q]=home?3:0;
        array<Word,MV> initial{};
        initial[p]=joinWord(repeated(1,keep),repeated(3,amount));
        initial[r]=repeated(2,relay);initial[q]=repeated(4,target);
        vector<Move> original={{int16_t(p),uint8_t(keep+amount-sent),3,4},
                               {int16_t(p),uint8_t(keep),3,1},
                               {int16_t(r),uint8_t(relay),3,3}};
        Board board=initial;auto expected=unpack(board),before=expected;
        for(auto m:original)assert(apply_reference(expected,m));
        vector<Move> replacement;int saved=0;
        int next=consolidateMonoDispatch(original,0,board,replacement,saved);
        if(next) {
            ++positive;assert(next==3&&saved==3-int(replacement.size())&&saved>0);
            for(auto m:replacement)assert(apply_reference(before,m));
            assert(before==expected&&unpack(board)==expected);
        }else {++negative;assert(saved==0&&replacement.empty()&&unpack(board)==before);}
        ++checked;
    }
    geo.nest[q]=0;
    const vector<Move> base={{int16_t(p),3,3,4},{int16_t(p),0,3,1},{int16_t(r),4,3,3}};
    for(int mode=0;mode<7;mode++) {
        array<Word,MV> initial{};initial[p]=0x3333;initial[r]=0x2222;initial[q]=3;
        vector<Move> answer;answer.reserve(5);answer.assign(base.begin(),base.end());
        const int a=geo.id[1][1],b=geo.id[1][2];initial[a]=1;
        if(mode==1)answer.insert(answer.begin()+1,Move{int16_t(a),0,3,1}); // unrelated
        if(mode==2) { // destination is touched, so this window must close
            answer.insert(answer.begin()+1,Move{int16_t(q),0,1,1});
        }
        if(mode==3) { // a non-monochrome source has the same height
            initial[p]=0x3313;
        }
        if(mode==4) { // old relay can take three pieces, but cannot take four
            initial[r]=0x22222;answer[2].k=5;
        }
        if(mode==5) { // a nest returns the common payload after each arrival
            geo.nest[q]=3;initial[q]=4;
        }
        if(mode==6) { // extra unrelated work between second and third departure
            answer.insert(answer.begin()+2,Move{int16_t(a),0,3,1});
            answer.insert(answer.begin()+3,Move{int16_t(b),0,3,1});
        }
        Board board=initial;auto expected=unpack(board),before=expected;
        for(auto m:answer)assert(apply_reference(expected,m));
        vector<Move> replacement;int saved=0;
        const int next=consolidateMonoDispatch(answer,0,board,replacement,saved);
        const bool wanted=mode==0||mode==1||mode==5||mode==6;
        assert(bool(next)==wanted);
        if(next){for(auto m:replacement)assert(apply_reference(before,m));assert(before==expected&&unpack(board)==expected);}
        else assert(unpack(board)==before&&replacement.empty());
        geo.nest[q]=0;++checked;
    }
    assert(positive>0&&negative>0);
    return checked;
}


void reference_apply(Reference& towers,Move m){assert(apply_reference(towers,m));}
void verify(const Board& initial,const vector<Move>& answer,const Board* final=nullptr) {
    auto towers=unpack(initial);for(Move m:answer)reference_apply(towers,m);
    if(final)assert(towers==unpack(*final));
    else for(const auto& tower:towers)assert(tower.empty());
}
vector<Move> read_path(const filesystem::path& path){return read_moves(path);}
void save(const filesystem::path& path,const vector<Move>& moves){write_moves(moves,path);}
using Small=array<Word,4>;
FiniteState encode_small(const Small& s) {
    FiniteState result{};int n=0;
    for(int p=0;p<4;p++)if(s[p])result[n++]=(uint64_t(p)<<32)|s[p];
    return result;
}
vector<pair<Small,Move>> reference_edges(const Small& s) {
    array<vector<int>,4> towers;
    for(int p=0;p<4;p++)for(Word w=s[p];w;w>>=4)towers[p].push_back(w&15);
    vector<pair<Small,Move>> result;
    for(int p=0;p<4;p++)for(int q=0;q<4;q++)if(p!=q)for(int k=0;k<int(towers[p].size());k++) {
        int l=abs(p-q);if(l>k+1||towers[q].size()+towers[p].size()-k>8)continue;
        auto next=towers;
        while(next[p].size()>size_t(k)){next[q].push_back(next[p].back());next[p].pop_back();}
        for(int v:{p,q})while(!next[v].empty()&&next[v].back()==geo.nest[v])next[v].pop_back();
        Small packed{};
        for(int v=0;v<4;v++)for(int i=0;i<int(next[v].size());i++)packed[v]|=Word(next[v][i])<<(4*i);
        result.push_back({packed,{int16_t(p),uint8_t(k),uint8_t(q<p?2:3),uint8_t(l)}});
    }
    return result;
}
struct FiniteProbe {
    static int lower(const Small& initial,const Small& goal) {
        FinitePlanner planner;planner.target=encode_small(goal);
        for(int p=0;p<4;p++) {
            planner.goal_word[p]=goal[p];
            for(Word w=goal[p];w;w>>=4)++planner.required[w&15];
        }
        double priority=0;return planner.assess(encode_small(initial),priority);
    }
};
int finite_fixtures() {
    string corridor="4 2\nA..B\n####\n####\n####\n";
    istringstream input(corridor);initialize(input);int checked=0;
    set<Small> all;
    for(int a=0;a<=2;a++)for(int b=0;b<=1;b++) {
        vector<int> colors(a,1);colors.insert(colors.end(),b,2);int n=a+b;
        do {
            for(int h0=0;h0<=n;h0++)for(int h1=0;h1<=n-h0;h1++)for(int h2=0;h2<=n-h0-h1;h2++) {
                array<int,4> hs={h0,h1,h2,n-h0-h1-h2};Small s{};int at=0;
                for(int p=0;p<4;p++)for(int k=0;k<hs[p];k++)s[p]|=Word(colors[at++])<<(4*k);
                bool normalized=true;for(int p=0;p<4;p++)normalized&=geo.normalize(s[p],p)==s[p];
                if(normalized)all.insert(s);
            }
        }while(next_permutation(colors.begin(),colors.end()));
    }
    map<Small,vector<pair<Small,Move>>> edges;
    for(const auto& s:all)edges[s]=reference_edges(s);
    for(const auto& initial:all) {
        map<Small,int> distance;queue<Small> todo;distance[initial]=0;todo.push(initial);
        while(!todo.empty()) {
            auto from=todo.front();todo.pop();
            for(const auto& [next,m]:edges[from])if(!distance.contains(next)){distance[next]=distance[from]+1;todo.push(next);}
        }
        for(const auto& goal:all) {
            const int lower=FiniteProbe::lower(initial,goal);
            if(distance.contains(goal))assert(lower>=0&&lower<=distance[goal]);
            if(lower<0)assert(!distance.contains(goal));
            ++checked;
        }
    }
    // Every primitive reference edge is also expressible by a one-step event.
    for(const auto& s:all) {
        Board board=geo.initial;for(int p=0;p<4;p++)board[p]=s[p];
        for(const auto& [next,m]:edges[s]) {
            FiniteRouter router;router.run(board,m.p,m.k,1);int q=geo.ray[m.p][m.d][m.l];
            assert(router.cost(q,1)==1);auto path=router.path(q,1);auto actual=unpack(board);
            for(Move step:path)reference_apply(actual,step);
            for(int p=0;p<4;p++) {
                vector<int> expected;for(Word w=next[p];w;w>>=4)expected.push_back(w&15);
                assert(actual[p]==expected);
            }
            ++checked;
        }
    }
    // A finite endpoint retains the slime even though its home is reachable.
    {
        Board initial=geo.initial;initial[1]=1;
        FiniteWindow w;w.count=4;w.pieces=1;w.towers=1;
        for(int p=0;p<4;p++){w.cells[p]=p;w.begin[p]=initial[p];}w.end[2]=1;
        FinitePlanner planner;FiniteStats stats;vector<Move> answer;
        assert(planner.plan(initial,w,1,numeric_limits<double>::infinity(),answer,stats)&&answer.size()==1);
        Board goal=geo.initial;goal[2]=1;verify(initial,answer,&goal);++checked;
        FiniteStats expired;
        assert(!planner.plan(initial,w,1,-1,answer,expired)&&expired.deadlines==1&&answer.empty());++checked;
    }
    // Capacity is checked before nest returns, including eight-slime parcels.
    {
        Board initial=geo.initial;initial[1]=0x11221122u;initial[2]=1;
        for(int k=0;k<8;k++) {
            FiniteRouter router;router.run(initial,1,k,6);
            for(int q=0;q<4;q++)for(int o=0;o<2;o++)if(router.cost(q,o)>0) {
                auto path=router.path(q,o);auto actual=unpack(initial);
                for(Move m:path)reference_apply(actual,m);
                assert(int(path.size())==router.cost(q,o));++checked;
            }
        }
    }
    assert(board_pool.free_count==BoardPool::slots);return checked;
}
void admit(const Board& initial,const FiniteWindow& w,const vector<Move>& reference) {
    Board current=initial;array<Word,MV> background{};
    for(int p=0;p<geo.V;p++)background[p]=initial[p];
    for(int i=0;i<w.count;i++)background[w.cells[i]]=0;
    for(Move m:reference) {
        int q=geo.ray[m.p][m.d][m.l];
        assert(find(w.cells.begin(),w.cells.begin()+w.count,q)!=w.cells.begin()+w.count);
        FiniteRouter router;router.run(current,m.p,m.k,1);
        assert(router.cost(q,1)==1);auto path=router.path(q,1);
        Board actual=current;for(Move step:path)geo.apply(actual,step);
        auto independent=unpack(current);reference_apply(independent,m);geo.apply(current,m);
        assert(actual==current&&unpack(current)==independent);
        for(int p=0;p<geo.V;p++)assert(prefix(current[p],height(background[p]))==background[p]);
    }
    for(int p=0;p<geo.V;p++) {
        int at=int(find(w.cells.begin(),w.cells.begin()+w.count,p)-w.cells.begin());
        assert(current[p]==(at<w.count?w.end[at]:initial[p]));
    }
}
int witnesses(const filesystem::path& root,const filesystem::path& out) {
    ifstream manifest(out/"witness_manifest.tsv");assert(manifest.is_open());
    ofstream csv(out/"witnesses.csv");csv<<"label,case,before,reference,found,after,expanded,generated,beam_pruned,nonempty,ms\n";
    string label,case_name,before_path,after_path;int first,last,checked=0;bool mandatory=true;
    while(manifest>>label>>case_name>>first>>last>>before_path>>after_path) {
        ifstream input(root/"tools/in"/(case_name+".txt"));initialize(input);
        auto original=read_path(before_path),reference=read_path(after_path);FiniteStats extraction;
        auto windows=collect_finite_windows(original,numeric_limits<double>::infinity(),extraction);
        auto found=find_if(windows.begin(),windows.end(),[&](const auto& w){return w.first==first&&w.last+1==last;});
        assert(found!=windows.end());const auto& w=*found;
        Board initial=geo.initial;for(int t=0;t<first;t++)geo.apply(initial,original[t]);
        vector<Move> related;for(int t:w.operations)related.push_back(original[t]);
        admit(initial,w,related);admit(initial,w,reference);checked+=2;
        FinitePlanner planner;FiniteStats stats;vector<Move> answer;
        auto start=chrono::steady_clock::now();
        bool ok=planner.plan(initial,w,int(reference.size()),numeric_limits<double>::infinity(),answer,stats);
        const double ms=chrono::duration<double,milli>(chrono::steady_clock::now()-start).count();
        if(ok) {
            assert(answer.size()<=reference.size());
            Board goal=initial;for(Move m:related)geo.apply(goal,m);verify(initial,answer,&goal);
            vector<Move> full(original.begin(),original.begin()+first);full.insert(full.end(),answer.begin(),answer.end());
            for(int t=first;t<last;t++)if(find(w.operations.begin(),w.operations.end(),t)==w.operations.end())full.push_back(original[t]);
            full.insert(full.end(),original.begin()+last,original.end());save(out/"witnesses"/(label+"_auto.txt"),full);
        }
        if(label=="paired_orientation"||label=="nest_pickup")mandatory&=ok;
        bool nonempty=any_of(w.end.begin(),w.end.begin()+w.count,[](Word x){return x!=0;});
        csv<<label<<','<<case_name<<','<<related.size()<<','<<reference.size()<<','<<ok<<','<<(ok?int(answer.size()):-1)
           <<','<<stats.expanded<<','<<stats.generated<<','<<stats.beam_pruned<<','<<nonempty<<','<<ms<<'\n';
        csv.flush();cout<<label<<" admitted=both found="<<ok<<" after="<<(ok?int(answer.size()):-1)<<" expanded="<<stats.expanded<<endl;
    }
    assert(checked==8&&mandatory);return checked;
}

ofstream pass_log;
string active_case,active_mode;
int pass_number=0,checked_passes=0;
bool same_move(Move a,Move b){return tie(a.p,a.k,a.d,a.l)==tie(b.p,b.k,b.d,b.l);}
bool same_moves(const vector<Move>& a,const vector<Move>& b){return a.size()==b.size()&&equal(a.begin(),a.end(),b.begin(),same_move);}
void auditFinalPass(int kind,const vector<Move>& original,const vector<Move>& reduced) {
    assert(reduced.size()<=original.size());
    Reference before=unpack(geo.initial),after=before;
    size_t first=0;
    while(first<reduced.size()&&same_move(original[first],reduced[first])) {
        assert(apply_reference(before,original[first]));++first;
    }
    after=before;
    size_t old_end=original.size(),new_end=reduced.size();
    while(old_end>first&&new_end>first&&same_move(original[old_end-1],reduced[new_end-1])) {--old_end;--new_end;}
    for(size_t i=first;i<old_end;i++)assert(apply_reference(before,original[i]));
    for(size_t i=first;i<new_end;i++)assert(apply_reference(after,reduced[i]));
    // Identical commands can move different amounts if their source heights
    // differ. Extend the common suffix until the board boundary really closes.
    while(before!=after&&old_end<original.size()&&new_end<reduced.size()) {
        assert(same_move(original[old_end],reduced[new_end]));
        assert(apply_reference(before,original[old_end++]));
        assert(apply_reference(after,reduced[new_end++]));
    }
    assert(before==after);
    for(size_t i=new_end;i<reduced.size();i++)assert(apply_reference(after,reduced[i]));
    for(const auto& tower:after)assert(tower.empty());
    ++checked_passes;++pass_number;
    if(pass_log.is_open()&&original.size()>reduced.size()) {
        pass_log<<"{\"case\":\""<<active_case<<"\",\"mode\":\""<<active_mode<<"\",\"pass\":"<<pass_number
                <<",\"kind\":\""<<final_reduction_names[kind]<<"\",\"first\":"<<first
                <<",\"old_end\":"<<old_end<<",\"new_end\":"<<new_end<<",\"before\":";
        json_moves(pass_log,original);pass_log<<",\"after\":";json_moves(pass_log,reduced);pass_log<<"}\n";
    }
}

void reset_stats() {trace={};coupled_routes={};local_mono_dispatch={};}
void unbounded(){clk.start=chrono::steady_clock::now();clk.limit=numeric_limits<double>::infinity();}
void reduce_control(vector<Move>& result,const string& mode,FinalReductionStats& stats) {
    while(true) {
        const size_t before=result.size();
        if(mode=="repeat") {
            apply_final_reduction(result,FINAL_ROUTE,stats,[&]{return smoothRoutes(result);});
            apply_final_reduction(result,FINAL_PAIR,stats,[&]{return compress_pair_transfers(result);});
            apply_final_reduction(result,FINAL_JOINT,stats,[&]{return compressJointWindows(result,clk.limit,stats.joint);});
        } else if(mode=="mono") {
            apply_final_reduction(result,FINAL_MONO,stats,[&]{return compress_mono_dispatch(result,clk.limit);});
        } else if(mode=="orientation") {
            apply_final_reduction(result,FINAL_STRICT,stats,[&]{int saved;return polishCoupledRoutes(result,clk.limit,saved,false);});
            apply_final_reduction(result,FINAL_SLACK,stats,[&]{int saved;return polishCoupledRoutes(result,clk.limit,saved,true);});
        } else {
            assert(mode=="finite");
            apply_final_reduction(result,FINAL_FINITE,stats,[&]{return compress_finite_windows(result,clk.limit,stats.finite);});
        }
        ++stats.cheap_rounds;
        if(result.size()==before){stats.stable=1;break;}
    }
}

int combination_fixtures() {
    string blank="12 4\n";for(int i=0;i<12;i++)blank+=string(12,'.')+'\n';
    istringstream input(blank);initialize(input);
    auto op=[](int i,int j,int k,int d,int l){return Move{int16_t(geo.id[i][j]),uint8_t(k),uint8_t(d),uint8_t(l)};};
    geo.initial.fill(0);fill(geo.nest,geo.nest+MV,0);
    geo.initial[geo.id[4][4]]=0x221;geo.initial[geo.id[4][7]]=3;
    geo.nest[geo.id[4][3]]=1;geo.nest[geo.id[3][5]]=2;geo.nest[geo.id[4][8]]=3;
    geo.home[1]=geo.id[4][3];geo.home[2]=geo.id[3][5];geo.home[3]=geo.id[4][8];
    const vector<Move> loop={op(4,4,2,3,1),op(4,5,0,3,1),op(4,6,0,3,1),op(4,7,1,2,2),
                            op(4,4,1,3,1),op(4,5,0,0,1),op(4,4,0,2,1),op(4,7,0,3,1)};
    unbounded();reset_stats();auto reduced=loop;FinalReductionStats stats;
    polish_final_reductions(reduced,stats);assert(reduced.size()<=4&&stats.saved[FINAL_ROUTE]>0&&stats.saved[FINAL_PAIR]>0);
    assert(stats.total_saved()==int(loop.size()-reduced.size()));
    auto fixed=reduced;FinalReductionStats repeated;polish_final_reductions(fixed,repeated);assert(same_moves(reduced,fixed));
    clk.limit=-1;auto expired=loop;FinalReductionStats stopped;polish_final_reductions(expired,stopped);
    assert(same_moves(loop,expired)&&stopped.deadlines==1&&stopped.total_saved()==0);
    unbounded();geo.initial.fill(0);fill(geo.nest,geo.nest+MV,0);
    vector<Move> empty;FinalReductionStats empty_stats;polish_final_reductions(empty,empty_stats);assert(empty.empty());
    geo.initial[geo.id[4][4]]=1;geo.nest[geo.id[4][5]]=1;geo.home[1]=geo.id[4][5];
    vector<Move> single={op(4,4,0,3,1)};FinalReductionStats one;polish_final_reductions(single,one);assert(single.size()==1);
    assert(board_pool.free_count==BoardPool::slots);return 5;
}

