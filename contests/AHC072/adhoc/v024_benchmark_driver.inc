namespace BENCH_NAMESPACE {
struct DependencyProbe {
    static BenchResult run(const std::filesystem::path& root,const std::filesystem::path& file) {
        BenchResult result;
        geo.~Geometry();new(&geo)Geometry();rng=RNG{};
        clk.start=chrono::steady_clock::now();clk.limit=numeric_limits<double>::infinity();
        auto original_input=cin.rdbuf();
        ifstream input(file);cin.clear();cin.rdbuf(input.rdbuf());geo.read();cin.rdbuf(original_input);
        ifstream output(root/"results/out/v022_dependency_lns"/file.filename());
        assert(output.is_open());vector<Move> answer;int i,j,k,l;char d;
        while(output>>i>>j>>k>>d>>l) {
            Move m;m.p=geo.id[i][j];m.k=k;m.l=l;
            auto direction=find(geo.dirs,geo.dirs+4,d);assert(direction!=geo.dirs+4);
            m.d=direction-geo.dirs;answer.push_back(m);
        }
        TemporalLNS lns;lns.initializeSpatialCuts();lns.makeCandidates(answer);
        auto record=[&](int kind,int stage,bool ok,const vector<Move>& moves) {
            auto& values=result.results;
            values.push_back(kind);values.push_back(stage);values.push_back(ok);
            values.push_back(ok?moves.size():0);
            if(ok)for(auto m:moves) {
                values.push_back(m.p);values.push_back(m.k);values.push_back(m.d);values.push_back(m.l);
            }
            result.rng_states.push_back(rng.x);
        };
        for(int kind=0;kind<2;kind++)for(int c=0;c<int(lns.candidates.size());c++) {
            auto cand=lns.candidates[c];if(kind&&!lns.growDependency(cand))continue;
            rng=RNG{};rng.x^=uint64_t(c+1)*0x9e3779b97f4a7c15ULL;
            int mode=c%12;
            lns.supportWeight=mode==0?0:mode==1?2:mode==2?8:5;lns.rideWeight=mode==3?4:1;
            vector<Move> base;Board initial;
            auto begin=chrono::steady_clock::now();bool ok=lns.extract(answer,cand.ids,base,initial);
            result.extract_ns[kind]+=chrono::duration<double,nano>(chrono::steady_clock::now()-begin).count();
            ++result.extract_calls[kind];record(kind,0,ok,base);
            for(Word word:initial)result.results.push_back(word);
            if(!ok)continue;
            vector<pair<double,int>> order;
            for(int id:cand.ids) {
                double weight=geo.dist[id][geo.home[geo.initial[id]]];
                weight*=0.6+0.8*rng.unit();order.emplace_back(-weight,id);
            }
            sort(order.begin(),order.end());int stage=0;
            for(auto [weight,id]:order) {
                (void)weight;++stage;vector<Move> next;
                int cap=int(answer.size())+4-int(base.size());begin=chrono::steady_clock::now();
                ok=lns.insert(initial,base,id,cap,next);
                result.insert_ns[kind]+=chrono::duration<double,nano>(chrono::steady_clock::now()-begin).count();
                ++result.insert_calls[kind];result.insert_success[kind]+=ok;record(kind,stage,ok,next);
                if(!ok)break;
                base=move(next);initial[id]=geo.initial[id];
            }
        }
        // The same routine also handles nonzero initial gaps in prebuilt towers.
        // Exercise each saved cut and a fixed set of paired reconstructions.
        for(int c=0;c<int(lns.packetCuts.size());c++) {
            rng=RNG{};rng.x^=uint64_t(c+1)*0xd1b54a32d192ed03ULL;
            vector<Move> next;bool ok=lns.flexibleNeighbor(answer,lns.packetCuts[c],3,next);
            record(2,c,ok,next);++result.flexible_calls;result.flexible_success+=ok;
        }
        lns.lastTried.clear();
        for(int c=0;c<64;c++) {
            rng=RNG{};rng.x^=uint64_t(c+1)*0x94d049bb133111ebULL;lns.iteration+=25;
            vector<Move> next;bool ok=lns.pairedNeighbor(answer,3,next);
            record(3,c,ok,next);++result.paired_calls;result.paired_success+=ok;
        }
#ifdef CHECK_INCOMING
        auto verify=[&](const array<int,MV>& h,const TemporalLNS::IncomingLaunches& incoming) {
            for(int p=0;p<geo.V;p++) {
                uint32_t expected=0;
                for(int direction=0;direction<4;direction++)for(int distance=1;distance<=geo.rayCount[p][direction];distance++) {
                    int q=geo.ray[p][direction][distance];
                    if(h[q]<8&&distance<=h[q]+1)expected|=uint32_t(1)<<(direction*8+distance-1);
                }
                assert(incoming.mask[p]==expected);++result.mask_checks;
            }
        };
        Board b=geo.initial;array<int,MV> h{};
        for(int p=0;p<geo.V;p++)h[p]=height(b[p]);
        TemporalLNS::IncomingLaunches incoming(h);verify(h,incoming);
        for(auto m:answer) {
            int p=m.p,q=geo.ray[p][m.d][m.l],hp=h[p],hq=h[q];
            geo.apply(b,m);h[p]=height(b[p]);h[q]=height(b[q]);
            incoming.update(p,hp,h[p]);incoming.update(q,hq,h[q]);verify(h,incoming);
        }
        // Empty/full towers and all height changes, independently of solver paths.
        for(int p=0;p<geo.V;p++)for(int after=0;after<=8;after++) {
            int before=h[p];incoming.update(p,before,after);h[p]=after;
            for(int direction=0;direction<4;direction++)for(int distance=1;distance<=geo.rayCount[p][direction];distance++) {
                int q=geo.ray[p][direction][distance],bit=(direction^1)*8+distance-1;
                assert(bool(incoming.mask[q]&(uint32_t(1)<<bit))==(after<8&&distance<=after+1));
                ++result.mask_checks;
            }
        }
        verify(h,incoming);
#endif
        return result;
    }
};
}
