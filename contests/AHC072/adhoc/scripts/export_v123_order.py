#!/usr/bin/env python3
"""学習済み重みを単一C++へ出力する。隠れ層の積和を並列に処理する。"""
import json
from build_v123_order import ROOT, RUN, build_submission
from check_v113_integrated import block
from v089_data import sha, save


def build():
    source=build_submission()
    model=json.loads((RUN/'training/model.json').read_text())['parameters']
    def value(x):
        if isinstance(x,list):return '{'+','.join(value(y) for y in x)+'}'
        return float(x).hex()+'f'
    weights=[list(row) for row in zip(*model['0.weight'])]
    code='''namespace order_policy {
static constexpr float first[80][16]='''+value(weights)+''';
static constexpr float bias[16]='''+value(model['0.bias'])+''';
static constexpr float last[16]='''+value(model['2.weight'][0])+''';
static constexpr float intercept='''+value(model['2.bias'][0])+''';
float predict(const array<float,80>& input) {
    // 各出力の加算順は入力0..79。隠れ層方向を連続配置しSIMDで並列に足す。
    array<float,16> hidden;copy(bias,bias+16,hidden.begin());
    for(int k=0;k<80;++k)for(int j=0;j<16;++j)hidden[j]+=first[k][j]*input[k];
    float answer=intercept;
    for(int j=0;j<16;++j)answer+=last[j]*max(0.f,hidden[j]);
    return answer;
}
}
'''
    text=source.read_text()
    text=text.replace(block(text,'namespace order_policy {'),code,1)
    # 追加操作数は非負。残存予定自体が上限を超える場合、どの順序も完了不能。
    text=text.replace('if(n>=3) {\n            LOCAL_ONLY(const auto order_begin=',
                      'if(n>=3&&int(base.size())<=allowance) {\n            LOCAL_ONLY(const auto order_begin=',1)
    # LOCAL_ONLYのブロックを越えて開始時刻を使うため、この宣言だけ同じスコープに置く。
    text=text.replace('LOCAL_ONLY(const auto order_begin=chrono::steady_clock::now();)',
                      '#ifdef LOCAL\n            const auto order_begin=chrono::steady_clock::now();\n#endif',1)
    text=text.replace('chrono::steady_clock::now()-order_begin).count());)',
                      'chrono::steady_clock::now()-order_begin).count()););',1)
    source.write_text(text)
    metadata=json.loads((RUN/'sources.json').read_text())
    metadata['sources']['learned']['sha256']=sha(source)
    metadata['exporter_sha256']=sha(ROOT/'adhoc/scripts/export_v123_order.py')
    save(RUN/'sources.json',metadata)
    return source


if __name__=='__main__':print(build())
