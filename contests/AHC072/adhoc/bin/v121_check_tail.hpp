// Included by the diagnostic generator, not by the submitted solver.
void compare(const vector<ProblemWindow>& ps,bool fixed,bool zero) {
    finite_probe::width=12;finite_probe::node_limit=192;
    for(int i=0;i<int(ps.size());i++) {
        const auto& p=ps[i];array<int,2> cost={int(p.original.size()),int(p.original.size())};
        array<double,2> seconds{};array<FiniteStats,2> stats{};
        array<int64_t,2> calls{};array<vector<Move>,2> results;
        for(int pass=0;pass<2;pass++) {
            int mode=pass^(i&1);finite_value::enabled=mode;
            State initial=p.initial;FinitePlanner planner;
            const double t0=time_keeper.exact_elapsed_sec();const auto c0=finite_value::calls;
            const double deadline=fixed?numeric_limits<double>::infinity():t0+PROGRAM_TIME_LIMIT_SEC*0.003;
            if(planner.plan(initial,p.w,int(p.original.size())-1,deadline,results[mode],stats[mode])) {
                cost[mode]=int(results[mode].size());verify(p.initial,results[mode],p.goal);
            }
            seconds[mode]=time_keeper.exact_elapsed_sec()-t0;calls[mode]=finite_value::calls-c0;
        }
        if(zero) {
            require(cost[0]==cost[1]&&results[0].size()==results[1].size(),"zero residual result mismatch");
            for(size_t j=0;j<results[0].size();j++) {
                auto a=results[0][j],b=results[1][j];
                require(a.p==b.p&&a.k==b.k&&a.d==b.d&&a.l==b.l,"zero residual path mismatch");
            }
            require(stats[0].generated==stats[1].generated&&stats[0].expanded==stats[1].expanded,"zero residual search mismatch");
        }
        cout<<setprecision(12)<<"{\"problem\":"<<i<<",\"original\":"<<p.original.size()<<",\"baseline\":"<<cost[0]
            <<",\"learned\":"<<cost[1]<<",\"baseline_seconds\":"<<seconds[0]<<",\"learned_seconds\":"<<seconds[1]
            <<",\"baseline_calls\":"<<calls[0]<<",\"learned_calls\":"<<calls[1]
            <<",\"baseline_expanded\":"<<stats[0].expanded<<",\"learned_expanded\":"<<stats[1].expanded
            <<",\"baseline_generated\":"<<stats[0].generated<<",\"learned_generated\":"<<stats[1].generated
            <<",\"baseline_deadlines\":"<<stats[0].deadlines<<",\"learned_deadlines\":"<<stats[1].deadlines<<"}\n";
        require(state_pool.free_count==4,"compare state pool leak");
    }
}

void features(const vector<ProblemWindow>& ps,const fs::path& output) {
    ofstream raw(output,ios::binary);finite_value::enabled=false;
    for(const auto& p:ps) {
        FinitePlanner planner;FiniteStats stats;vector<Move> unused;
        // Initialization is common to plan(); cap zero prevents actual beam expansion.
        {State initial=p.initial;planner.plan(initial,p.w,0,numeric_limits<double>::infinity(),unused,stats);}
        auto state=unpack(p.initial);
        for(size_t t=0;t<=p.original.size();t++) {
            auto f=FiniteProbe::features(planner,FiniteProbe::encode(pack(state),p.background));
            raw.write(reinterpret_cast<const char*>(f.data()),sizeof(f));
            if(t<p.original.size())reference_apply(state,p.original[t]);
        }
    }
}

int main(int argc,char** argv) {
    try {
        if(argc==3&&string(argv[1])=="predict") {
            ifstream in(argv[2],ios::binary);array<float,48> x;cout<<setprecision(10);
            while(in.read(reinterpret_cast<char*>(x.data()),sizeof(x)))
                cout<<max(x[0],x[1]+finite_value::residual(x))<<'\n';
            return 0;
        }
        require(argc>=4,"usage: compare|fixed|zero|features input problems [output]");
        ifstream input(argv[2]);require(bool(input),"missing input");cin.rdbuf(input.rdbuf());
        board_info.read();state_pool.prepare(board_info.cell_count);
        time_keeper.start_=chrono::steady_clock::now();time_keeper.time_limit_sec=numeric_limits<double>::infinity();
        const string mode=argv[1];auto ps=read_problems(argv[3]);
        if(mode=="compare"||mode=="fixed"||mode=="zero")compare(ps,mode!="compare",mode=="zero");
        else if(mode=="features"){require(argc==5,"features output");features(ps,argv[4]);}
        else throw runtime_error("unknown mode");
        require(state_pool.free_count==4,"final state pool leak");return 0;
    }catch(const exception& e){cerr<<e.what()<<'\n';return 1;}
}
