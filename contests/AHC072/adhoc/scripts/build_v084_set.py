#!/usr/bin/env python3
"""v079から個体共有の集合モデルを生成する。初期重みはv079の関数を保持する。"""
from pathlib import Path
import hashlib
import re

ROOT = Path(__file__).resolve().parents[2]
source = ROOT / 'src/bin/v079_nn_immediate.cpp'
assert hashlib.sha256(source.read_bytes()).hexdigest() == 'a58af5f053bfbf6ce33463c60e21d0bbf6f659479c6aadb5d452d158da65bb8a'
s = source.read_text()

def replace(old, new):
    global s
    assert s.count(old) == 1, (old[:100], s.count(old))
    s = s.replace(old, new)

old_model = s[s.index('namespace neural_rank {'):s.index('\n#ifdef LOCAL\nstruct NeuralRankStats')]
def declaration(name):
    found = re.search(r'constexpr float ' + name + r'\[[^;]+;', old_model)
    assert found
    return found.group(0)

model = '// V084_MODEL_BEGIN\nnamespace neural_rank {\n'
model += declaration('mean') + '\n' + declaration('scale') + '\n'
model += 'constexpr float item_mean[20]={};\nconstexpr float item_scale[20]={' + ','.join(['1.0f']*20) + '};\n'
model += 'constexpr float ew0[16][20]={},eb0[16]={},ew1[8][16]={},eb1[8]={};\n'
model += declaration('w1').replace('w1[16][32]', 'hw0[16][48]') + '\n'
model += declaration('b1').replace('b1[16]', 'hb0[16]') + '\n'
model += 'constexpr float hw1[1][16]={' + declaration('w2').split('=',1)[1].removesuffix(';') + '};\n'
model += declaration('b2').replace('b2[1]', 'hb1[1]') + '\n'
model += '''// 各積和は列番号の昇順。正規化前にfloat32へ丸める。
array<float,8> encode(const array<double,20>& raw){
    array<float,20> x;array<float,16> hidden;array<float,8> output;
    for(int j=0;j<20;j++)x[j]=(float(raw[j])-item_mean[j])/item_scale[j];
    for(int i=0;i<16;i++){
        float value=eb0[i];for(int j=0;j<20;j++)value+=ew0[i][j]*x[j];
        hidden[i]=max(0.0f,value);
    }
    for(int i=0;i<8;i++){
        float value=eb1[i];for(int j=0;j<16;j++)value+=ew1[i][j]*hidden[j];
        output[i]=max(0.0f,value);
    }
    return output;
}
float predict(const array<double,32>& raw,const array<float,8>& pooled_mean,const array<float,8>& pooled_max){
    array<float,48> x;
    for(int j=0;j<32;j++)x[j]=(float(raw[j])-mean[j])/scale[j];
    for(int j=0;j<8;j++){x[32+j]=pooled_mean[j];x[40+j]=pooled_max[j];}
    float answer=hb1[0];
    for(int i=0;i<16;i++){
        float value=hb0[i];for(int j=0;j<48;j++)value+=hw0[i][j]*x[j];
        answer+=hw1[0][i]*max(0.0f,value);
    }
    return answer;
}
}
// V084_MODEL_END'''
replace('// v079_nn_immediate.cpp', '// v084_nn_set.cpp')
replace('// v078 immediate、120 epochの固定重み。低い出力の候補を選ぶ。\n' + old_model,
        '// 個体の共有表現と候補集合の順位。学習前の初期関数はv079と同じ。\n#ifdef AHC084_MODEL_HEADER\n#include AHC084_MODEL_HEADER\n#else\n' + model + '\n#endif\n')
member, choose = (ROOT / 'adhoc/scripts/v084_set_support.cpp.txt').read_text().split('// NN084_CHOOSE_CODE\n')
replace('    vector<PacketCut> packet_cuts;', member + '\n    vector<PacketCut> packet_cuts;')
replace('    void make_candidates(const vector<Move>& answer) {', '    void make_candidates(const vector<Move>& answer) {\n        nn084_items_ready=false;nn084_encoded.fill(false);')
start=s.index('    int nn_choose(')
end=s.index('\n    void optimize(',start)
s=s[:start]+choose+s[end:]
# 保存局面の検査では探索用mainを含めず、統合された特徴・推論だけを使う。
replace('int main() {', '#ifndef AHC072_NN_PROBE\nint main() {')
s+='\n#endif\n'
output=ROOT/'src/bin/v084_nn_set.cpp'
output.write_text(s)
print(output)
