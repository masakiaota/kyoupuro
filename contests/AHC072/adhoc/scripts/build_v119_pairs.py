#!/usr/bin/env python3
"""v111の条件を保ち、LNSで実際に短くなった完成列の対を有界に記録する。"""
from pathlib import Path
from build_v116_guidance import ROOT,PARENT,PARENT_SHA,sha,replace_once

def build():
    assert sha(PARENT)==PARENT_SHA
    text=PARENT.read_text()
    recorder=r'''
namespace v119_pairs {
struct Record {int kind;vector<Move> before,after;};
vector<Record> records;
array<int,2> seen{},kept{};
void capture(int kind,const vector<Move>& before,const vector<Move>& after) {
    if(after.size()>=before.size())return;
    const int group=kind<0?0:1;++seen[group];
    if(kept[group]>=32)return;
    records.push_back({kind,before,after});++kept[group];
}
struct Writer {
    ~Writer() {
        const char* path=getenv("V119_PAIRS");if(!path)return;
        ofstream out(path);out<<records.size()<<' '<<seen[0]<<' '<<seen[1]<<'\n';
        for(const auto& record:records) {
            out<<record.kind<<' '<<record.before.size()<<' '<<record.after.size()<<'\n';
            for(const auto* moves:{&record.before,&record.after})for(Move m:*moves)
                out<<m.p<<' '<<int(m.k)<<' '<<int(m.d)<<' '<<int(m.l)<<'\n';
        }
    }
};
}
'''
    text=replace_once(text,'struct SearchReductions {',recorder+'\nstruct SearchReductions {')
    text=replace_once(text,'            saved[kind]+=answer.size()-result.size();answer=move(result);',
                     '            v119_pairs::capture(kind,answer,result);\n            saved[kind]+=answer.size()-result.size();answer=move(result);')
    text=replace_once(text,'            if(accept) {','            if(accept) {\n                if(delta<0)v119_pairs::capture(-1,current,polished);')
    text=replace_once(text,'int main() {','int main() {\n    v119_pairs::Writer pair_writer;')
    output=ROOT/'adhoc/bin/v119_record_pairs.cpp'
    output.write_text('// v119_record_pairs.cpp\n'+text.split('\n',1)[1])
    print(output,sha(output))
    helper=(ROOT/'adhoc/bin/collect_v116_finite.cpp').read_text()
    helper=helper[:helper.index('int main(int argc,char** argv)')]
    helper=helper.replace('double estimate;int lower','double estimate=0;int lower')
    checker=ROOT/'adhoc/bin/check_v119_pairs.cpp'
    checker.write_text('// check_v119_pairs.cpp\n'+helper.split('\n',1)[1]+(ROOT/'adhoc/scripts/v119_pair_analysis.cpp.txt').read_text())
    print(checker,sha(checker))

if __name__=='__main__':build()
