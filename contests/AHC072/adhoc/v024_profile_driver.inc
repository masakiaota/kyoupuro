
struct DependencyProbe {
    static void run(const filesystem::path& root) {
        auto original_input=cin.rdbuf();
        vector<filesystem::path> files;
        for(auto e:filesystem::directory_iterator(root/"tools/in"))
            if(e.path().extension()==".txt")files.push_back(e.path());
        sort(files.begin(),files.end());
        cout<<"case,kind,candidate,size,stage,success,cap,total_ns,setup_ns,spatial_ns,event_ns,seed_ns,reconstruct_ns,layers,scanned,eligible\n";
        for(const auto& file:files) {
            geo.~Geometry();new(&geo)Geometry();rng=RNG{};
            ifstream input(file);cin.clear();cin.rdbuf(input.rdbuf());geo.read();
            ifstream output(root/"results/out/v022_dependency_lns"/file.filename());
            assert(output.is_open());vector<Move> answer;int i,j,k,l;char d;
            while(output>>i>>j>>k>>d>>l) {
                Move m;m.p=geo.id[i][j];m.k=k;m.l=l;
                auto direction=find(geo.dirs,geo.dirs+4,d);assert(direction!=geo.dirs+4);
                m.d=direction-geo.dirs;answer.push_back(m);
            }
            TemporalLNS lns;lns.initializeSpatialCuts();lns.makeCandidates(answer);
            for(int kind=0;kind<2;kind++)for(int c=0;c<int(lns.candidates.size());c++) {
                auto cand=lns.candidates[c];
                if(kind&&!lns.growDependency(cand))continue;
                // A deterministic diagnostic sample uses each seed once,
                // without accepting candidates or changing the saved answer.
                rng=RNG{};rng.x^=uint64_t(c+1)*0x9e3779b97f4a7c15ULL;
                int mode=c%12;
                lns.supportWeight=mode==0?0:mode==1?2:mode==2?8:5;
                lns.rideWeight=mode==3?4:1;
                vector<Move> base;Board initial;
                auto begin=ProfileClock::now();bool ok=lns.extract(answer,cand.ids,base,initial);
                double elapsed=profile_ns(begin);
                cout<<file.stem().string()<<','<<kind<<','<<c<<','<<cand.ids.size()
                    <<",0,"<<ok<<",0,"<<elapsed<<",0,0,0,0,0,0,0,0\n";
                if(!ok)continue;
                vector<pair<double,int>> order;
                for(int id:cand.ids) {
                    double weight=geo.dist[id][geo.home[geo.initial[id]]];
                    weight*=0.6+0.8*rng.unit();order.emplace_back(-weight,id);
                }
                sort(order.begin(),order.end());int stage=0;
                for(auto [weight,id]:order) {
                    (void)weight;++stage;vector<Move> next;
                    int cap=int(answer.size())+4-int(base.size());
                    insert_profile={};begin=ProfileClock::now();
                    ok=lns.insert(initial,base,id,cap,next);elapsed=profile_ns(begin);
                    const auto& z=insert_profile;
                    cout<<file.stem().string()<<','<<kind<<','<<c<<','<<cand.ids.size()<<','<<stage
                        <<','<<ok<<','<<cap<<','<<elapsed<<','<<z.setup<<','<<z.spatial<<','<<z.event
                        <<','<<z.seed<<','<<z.reconstruct<<','<<z.layers<<','<<z.scanned<<','<<z.eligible<<'\n';
                    if(!ok)break;
                    base=move(next);initial[id]=geo.initial[id];
                }
            }
        }
        cin.rdbuf(original_input);
    }
};

int main(int argc,char** argv) {
    assert(argc==2);clk.start=chrono::steady_clock::now();clk.limit=numeric_limits<double>::infinity();
    DependencyProbe::run(argv[1]);
}
