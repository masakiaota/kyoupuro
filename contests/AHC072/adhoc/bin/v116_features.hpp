    array<uint8_t,max_cells> value_background_height{};
    // Goal-relative aggregates avoid dependence on color IDs and orientation.
    // Routing stays exact; these features are used only for beam ordering.
    array<float,48> learned_features(const FiniteState& s,int lower,double base) const {
        array<TowerBits,max_cells> current{};
        array<int,13> actual{};
        array<int,16> positions{},masks{};
        array<float,48> f{};
        f[0]=lower;f[1]=float(base);f[2]=jump;
        int total=0,needed=0,matched_total=0,n=0,goals=0;
        int sources=0,destinations=0,unsettled=0;
        double distance=0,weighted_distance=0,top_distance=0,scaled_distance=0;
        int max_distance=0,max_height=0,max_goal_height=0;
        for(uint64_t x:s)if(x) {
            const int p=cell(x);const TowerBits w=word(x);const int h=height(w);
            current[p]=w;positions[n]=p;masks[n++]=color_mask(w);
            total+=h;max_height=max(max_height,h);f[13]+=h*h;
            const int matched=common_tower_prefix(w,goal_word[p]);
            matched_total+=matched;sources+=h>matched;unsettled+=h-matched;
            f[16]+=w==goal_word[p];f[17]+=matched>0&&w!=goal_word[p];
            f[18]+=goal_word[p]==0;f[19]+=mono_color_code(w)>0;
            f[20]+=runs(w);f[21]+=h==1;f[22]+=mono_color_code(w)==0;
            f[26]+=goal_word[p]!=0&&top_color_code(w)==top_color_code(goal_word[p]);
            f[27]+=goal_word[p]!=0&&(w&15)==(goal_word[p]&15);
            f[28]+=board_info.nest_code[p]!=0;
            f[29]+=board_info.nest_code[p]!=0&&top_color_code(w)!=board_info.nest_code[p];
            for(TowerBits a=w;a;a>>=4)++actual[a&15];
            const TowerBits suffix=matched==8?0:w>>(4*matched);
            for(unsigned mask=color_mask(suffix);mask;mask&=mask-1) {
                const int d=target_distance[1+__builtin_ctz(mask)][p];
                distance+=d;max_distance=max(max_distance,d);
            }
            for(TowerBits a=suffix;a;a>>=4)weighted_distance+=target_distance[a&15][p];
            top_distance+=suffix?target_distance[top_color_code(suffix)][p]:0;
            scaled_distance+=suffix?double(target_distance[top_color_code(suffix)][p])/max(1,h+int(value_background_height[p])):0;
            f[41]+=value_background_height[p];
            int support=0;
            for(int d=0;d<4;d++)if(board_info.adj[p][d]>=0)
                support=max(support,int(value_background_height[board_info.adj[p][d]]));
            f[42]+=support;
        }
        for(int p=0;p<board_info.cell_count;p++)if(goal_word[p]) {
            const int h=height(goal_word[p]);++goals;needed+=h;
            max_goal_height=max(max_goal_height,h);f[14]+=h*h;
            const int matched=common_tower_prefix(current[p],goal_word[p]);
            destinations+=h>matched;
            f[35]+=current[p]==0;
            f[36]+=current[p]!=0&&(color_mask(current[p])&color_mask(goal_word[p]))==0;
        }
        for(int c=1;c<=board_info.K;c++) {
            f[23]+=required[c]>0;f[24]+=actual[c]>0;
            f[25]+=actual[c]>0&&required[c]==0;
        }
        int shared=0,pairs=0;double shared_distance=0;int nearest=infinite_cost;
        for(int a=0;a<n;a++)for(int b=a+1;b<n;b++) {
            ++pairs;
            if(masks[a]&masks[b]) {
                ++shared;int d=board_info.floor_dist[positions[a]][positions[b]];
                shared_distance+=d;nearest=min(nearest,d);
            }
        }
        f[3]=sources;f[4]=destinations;f[5]=unsettled;f[6]=total;f[7]=needed;
        f[8]=total-needed;f[9]=n;f[10]=goals;f[11]=max_height;f[12]=max_goal_height;
        f[15]=matched_total;f[30]=weighted_distance;f[31]=distance;
        f[32]=max_distance;f[33]=top_distance;f[34]=scaled_distance;
        f[37]=total?float(matched_total)/total:1.0f;
        f[38]=shared;f[39]=shared?float(shared_distance/shared):0.0f;
        f[40]=shared?float(nearest):0.0f;
        int background_cells=0,background_pieces=0;
        for(int p=0;p<board_info.cell_count;p++) {
            background_cells+=value_background_height[p]>0;
            background_pieces+=value_background_height[p];
        }
        f[43]=background_pieces;f[44]=background_cells;
        f[45]=pairs?float(shared)/pairs:0.0f;f[46]=board_info.N;
        f[47]=float(board_info.cell_count)/(board_info.N*board_info.N);
        return f;
    }
