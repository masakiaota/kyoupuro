#!/usr/bin/env python3
"""最初の選抜へ輸送構造の固定モデルを組み込み、以後の育成を保持する。"""
from build_v129_allocation import ROOT, BASE, BASE_SHA, grow_tracking
from check_v113_integrated import block
from train_v129_allocation import load
from v089_data import sha


def build_candidate(model_path, name):
    assert sha(BASE) == BASE_SHA
    model = load(model_path)
    assert model['architecture'] == [42, 64, 64, 1]
    parameters = model['parameters']

    def literal(value):
        if isinstance(value, list):
            return '{' + ','.join(literal(v) for v in value) + '}'
        return float(value).hex() + 'f'

    weights = 'namespace allocation_policy {\n'
    for layer, input_width, output_width in [(0, 42, 64), (2, 64, 64)]:
        matrix = [list(v) for v in zip(*parameters[f'{layer}.weight'])]
        weights += f'static constexpr float weights_{layer}[{input_width}][{output_width}]=' + literal(matrix) + ';\n'
        weights += f'static constexpr float bias_{layer}[{output_width}]=' + literal(parameters[f'{layer}.bias']) + ';\n'
    weights += 'static constexpr float output[64]=' + literal(parameters['4.weight'][0]) + ';\n'
    weights += 'static constexpr float intercept=' + literal(parameters['4.bias'][0]) + ';\n'
    weights += '''float predict(const array<float,42>& f) {
    array<float,64> a,b;copy(bias_0,bias_0+64,a.begin());copy(bias_2,bias_2+64,b.begin());
    for(int i=0;i<42;++i)for(int j=0;j<64;++j)a[j]+=weights_0[i][j]*f[i];
    for(int i=0;i<64;++i)for(int j=0;j<64;++j)b[j]+=weights_2[i][j]*max(0.f,a[i]);
    float result=intercept;for(int j=0;j<64;++j)result+=output[j]*max(0.f,b[j]);return result;
}
}
'''
    text = BASE.read_text()
    grow = grow_tracking(text)
    rank = '''    auto allocation_rank=[&](vector<int>& ids) {
        const double now=time_keeper.exact_elapsed_sec();
        int shortest=INT_MAX;for(const auto& entry:pool.entries)shortest=min(shortest,int(entry.moves.size()));
        array<double,5> values{};
        for(int id:ids) {
            const int current=int(pool.entries[id].moves.size());
            const auto basic=allocation_policy::features(allocation_tracks[id],current,shortest,now,start,end);
            const auto structural=allocation_policy::structure(pool.entries[id].moves);
            array<float,42> f;copy(basic.begin(),basic.end(),f.begin());copy(structural.begin(),structural.end(),f.begin()+18);
            const double gain=clamp(20.0*allocation_policy::predict(f),0.0,double(current));
            if(!isfinite(gain))throw logic_error("invalid allocation prediction");
            values[id]=current-gain;
        }
        sort(ids.begin(),ids.end(),[&](int a,int b) {
            if(values[a]!=values[b])return values[a]<values[b];
            return pool.entries[a].moves.size()!=pool.entries[b].moves.size()?pool.entries[a].moves.size()<pool.entries[b].moves.size():a<b;
        });
        LOCAL_ONLY(trace.count_by("allocation_rank_calls",1);
                   trace.count_by("allocation_selected_longer",int(pool.entries[ids[0]].moves.size())>shortest);
                   trace.add_time_ms("allocation_prediction",(time_keeper.exact_elapsed_sec()-now)*1000));
    };
'''
    marker = '    try {\n        // 最初の2段階'
    assert grow.count(marker) == 1
    grow = grow.replace(marker, rank + marker, 1)
    old = '''            sort(active.begin(),active.end(),[&](int a,int b) {
                const size_t left=pool.entries[a].moves.size(),right=pool.entries[b].moves.size();
                return left!=right?left<right:a<b;
            });'''
    assert grow.count(old) == 1
    grow = grow.replace(old, '            if(round==0)allocation_rank(active);\n            else {\n' + old + '\n            }', 1)
    resize = '            active.resize(round==0?(active.size()+1)/2:min<size_t>(2,active.size()));'
    assert grow.count(resize) == 1
    # 生き残った候補の育成順は親に合わせる。以後の二候補配分は変更しない。
    grow = grow.replace(resize, resize + '\n            if(round==0) {\n' + old + '\n            }', 1)
    helpers = (ROOT / 'adhoc/scripts/v124_features.cpp.txt').read_text() + '\n'
    helpers += (ROOT / 'adhoc/scripts/v129_structure.cpp.txt').read_text() + '\n' + weights
    text = text.replace(block(text, 'void grow_initial_solutions('), helpers + '\n' + grow, 1)
    target = ROOT / 'adhoc/bin' / (name + '.cpp')
    target.write_text('// ' + target.name + '\n' + text.split('\n', 1)[1])
    return target
