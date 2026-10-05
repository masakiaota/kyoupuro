#!/usr/bin/env python3
"""最初の育成状態を固定したまま、継続探索の乱数だけを替えて採取する。"""
from build_v124_allocation import BASE, BASE_SHA, grow_tracking
from check_v113_integrated import block
from v089_data import ROOT, sha

RUN = ROOT / 'results/nn_rank/v129/20261004_structure_allocation_studio'


def build_collector():
    assert sha(BASE) == BASE_SHA
    text = BASE.read_text()
    grow = grow_tracking(text)
    grow = grow.replace('searches.back()->random_state=rng.next();',
                        'searches.back()->random_state=rng.next()^v129_mix(v129_repeat);', 1)
    grow = grow.replace('            sort(active.begin(),active.end(),[&](int a,int b) {',
                        '            if(round==0)v129_collect(0,pool,searches,allocation_tracks,start,end);\n'
                        '            sort(active.begin(),active.end(),[&](int a,int b) {', 1)
    helper = (ROOT / 'adhoc/scripts/v124_features.cpp.txt').read_text() + '\n'
    helper += (ROOT / 'adhoc/scripts/v129_structure.cpp.txt').read_text() + '\n'
    helper += (ROOT / 'adhoc/scripts/v129_collect.cpp.txt').read_text()
    text = text.replace(block(text, 'void grow_initial_solutions('), helper + '\n' + grow, 1)
    text = text.replace('int main() {', 'int v129_parent_main() {', 1)
    main = block(text, 'int v129_parent_main() {')
    text = text.replace(main, main[:-1] + '    return 0;\n}', 1)
    text += '''
int main(int argc,char** argv) {
    if(argc!=3)return 2;
    v129_repeat=stoull(argv[1]);v129_labels.open(argv[2]);if(!v129_labels)return 3;
    const int code=v129_parent_main();v129_labels.close();return code;
}
'''
    target = ROOT / 'adhoc/bin/collect_v129_allocation.cpp'
    target.write_text('// ' + target.name + '\n#include <unistd.h>\n#include <sys/wait.h>\n' + text.split('\n', 1)[1])
    return target


if __name__ == '__main__':
    print(build_collector())
