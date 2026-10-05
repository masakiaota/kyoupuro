#!/usr/bin/env python3
"""Prepare and compile v039 diagnostics without running a solution."""
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v039_exact_board"
NAMES = {"parent": "v037_pair_transfer_compression", "child": "v039_exact_board_lns"}
FLAGS = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread", "-ftrivial-auto-var-init=zero", "-fopenmp"]
DEFINES = {"local": ["-DLOCAL"], "production": ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"]}


def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()


def environment():
    env = os.environ.copy()
    if sys.platform == "darwin":
        env["SDKROOT"] = subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip()
        env["MACOSX_DEPLOYMENT_TARGET"] = "15.0"
    return env


def instrument_pool(s):
    s = s.replace('    int free_count = 0;', '''    int free_count = 0;
    bool used[slots]{};
    int peak_live=0;
    uint64_t takes=0,gives=0;
''', 1)
    s = s.replace('    void prepare(int floors) {', '''    void prepare(int floors) {
        assert(!storage || free_count==slots);
        fill(begin(used),end(used),false);peak_live=0;takes=gives=0;
''', 1)
    s = s.replace('    Word* take() { return free_slot[--free_count]; }', '''    Word* take() {
        assert(free_count>0 && free_count<=slots);
        Word* p=free_slot[--free_count];
        const ptrdiff_t offset=p-storage.get(), stride=ptrdiff_t(bytes/sizeof(Word));
        assert(offset>=0 && offset<stride*slots && offset%stride==0);
        const int at=int(offset/stride);assert(!used[at]);used[at]=true;
        peak_live=max(peak_live,slots-free_count);++takes;return p;
    }''', 1)
    s = s.replace('    void give(Word* cell) { free_slot[free_count++]=cell; }', '''    void give(Word* cell) {
        assert(free_count>=0 && free_count<slots);
        const ptrdiff_t offset=cell-storage.get(), stride=ptrdiff_t(bytes/sizeof(Word));
        assert(offset>=0 && offset<stride*slots && offset%stride==0);
        const int at=int(offset/stride);assert(used[at]);used[at]=false;
        ++gives;free_slot[free_count++]=cell;
    }''', 1)
    return s


def prepare():
    assert not (OUT / "frozen.json").exists()
    protected = [ROOT / "src/bin/v000_template.cpp"]
    for name in NAMES.values():
        protected.append(ROOT / "src/bin" / (name + ".cpp"))
    for folder in (ROOT / "tools/in", ROOT / "results/out" / NAMES["parent"]):
        for p in sorted(folder.glob("*.txt*")):
            q=OUT / "fixture" / p.relative_to(ROOT);q.parent.mkdir(parents=True,exist_ok=True)
            shutil.copy2(p,q);protected.append(p)
    shutil.copy2(ROOT / "notes/experiments/v039.md", OUT / "preregistration.md")
    (OUT / "protected.json").write_text(json.dumps({str(p.relative_to(ROOT)):digest(p) for p in protected},indent=2)+'\n')
    for label, name in NAMES.items():
        source=(ROOT / "src/bin" / (name+'.cpp')).read_text()
        exposed=source.replace('class TemporalLNS {','class TemporalLNS {\npublic:',1).replace('int main() {','int retained_solver_entry() {',1)
        assert exposed.endswith('}\n')
        exposed=exposed[:-2]+'    return 0;\n}\n'
        (OUT / (label+'_exposed.cpp.txt')).write_text(exposed)
        debug=instrument_pool(exposed) if label=='child' else exposed
        (OUT / (label+'_diagnostic.cpp.txt')).write_text(debug)
        fixed=instrument_pool(source) if label=='child' else source
        fixed=fixed.replace('    double elapsed() const {','    mutable uint64_t ticks=0;\n    double elapsed() const {',1)
        fixed=fixed.replace('return chrono::duration<double>(chrono::steady_clock::now()-start).count();','return double(++ticks)*0.000100003;',1)
        trailer='    cerr<<"[fixed.rng] "<<rng.x<<"\\n[fixed.ticks] "<<clk.ticks<<"\\n";\n'
        if label=='child':
            trailer+='    assert(board_pool.free_count==BoardPool::slots && board_pool.takes==board_pool.gives);\n'
            trailer+='    cerr<<"[pool.peak] "<<board_pool.peak_live<<"\\n[pool.takes] "<<board_pool.takes<<"\\n";\n'
        fixed=fixed[:-2]+trailer+'}\n'
        (OUT / (label+'_fixed.cpp.txt')).write_text(fixed)

    previous=(ROOT / 'adhoc/bin/bench_v036_real_clock.cpp').read_text()
    common=previous[previous.index('using namespace std;'):previous.index('template<size_t capacity>\nstruct V035')]
    common=common.replace('    static auto smooth(', '    static auto compress(const vector<Move>& moves){return parent::compress_pair_transfers(moves);}\n    static auto smooth(',1)
    adapter=common[common.index('struct Parent {'):].replace('Parent','Child').replace('parent::','child::')
    adapter=adapter.replace('child::geo.read();','child::geo.read();child::board_pool.prepare(child::geo.V);',1)
    mid=previous[previous.index('static uint64_t cpu_ns()'):previous.index('struct WorkResult {')]
    load=previous[previous.index('static Case load_case'):previous.index('static constexpr array<const char*,3> variant_names')]
    load=load.replace('results/out/v028_two_order_lns','results/out/'+NAMES['parent'])
    header='// bench_v039_exact_board.cpp\n#include <bits/stdc++.h>\n#include <time.h>\n'
    for label in NAMES:
        header+=f'namespace {label} {{\n#include "../v039_exact_board/{label}_exposed.cpp.txt"\n}}\n'
    tail=(ROOT / 'adhoc/scripts/v039_bench_main.cpp.txt').read_text()
    (ROOT / 'adhoc/bin/bench_v039_exact_board.cpp').write_text(header+common+adapter+mid+load+tail)
    print('Prepared frozen-input fixtures and sources',flush=True)


def compile_one(source,binary,flags,tag):
    env=environment(); cmd=['g++-15',*FLAGS,*flags,str(source),'-o',str(binary)]
    timer=['/usr/bin/time','-l'] if sys.platform=='darwin' else ['/usr/bin/time','-v']
    started=time.perf_counter()
    with (OUT / (tag+'_build.log')).open('w') as f:
        subprocess.run(timer+cmd,env=env,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT,check=True)
    elapsed=time.perf_counter()-started
    text=(OUT / (tag+'_build.log')).read_text()
    m=re.search(r'([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys',text)
    rss=re.search(r'(\d+)\s+maximum resident set size',text)
    result={'command':cmd,'wall_seconds':elapsed,'binary_bytes':binary.stat().st_size,'sha256':digest(binary)}
    if m:result.update(cpu_user_seconds=float(m[2]),cpu_sys_seconds=float(m[3]))
    if rss:result['max_rss_bytes']=int(rss[1])
    print(tag,round(elapsed,3),'seconds',flush=True)
    return result


def build():
    assert not (OUT/'frozen.json').exists()
    results={}
    for mode,defines in DEFINES.items():
        texts=[]
        for label,name in NAMES.items():
            src=ROOT/'src/bin'/(name+'.cpp')
            key=label+'_'+mode
            results[key]=compile_one(src,OUT/key,defines,key)
            run=subprocess.run(['g++-15',*FLAGS,*defines,'-E','-P',str(src)],env=environment(),capture_output=True,text=True,check=True)
            assert not run.stderr,run.stderr
            (OUT/(key+'.ii')).write_text(run.stdout)
            normalized=run.stdout
            for source_name in NAMES.values():
                normalized=normalized.replace(str(ROOT/'src/bin'/(source_name+'.cpp')),'solver.cpp')
            normalized=re.sub(r'("solver\.cpp",\s*)\d+(,)',r'\g<1>0\2',normalized)
            texts.append(normalized)
        (OUT/(mode+'_preprocessed.diff')).write_text(''.join(difflib.unified_diff(texts[0].splitlines(True),texts[1].splitlines(True),fromfile='parent',tofile='child')))
    for label in NAMES:
        flags=['-DLOCAL','-fsanitize=undefined','-fsanitize-undefined-trap-on-error','-x','c++']
        results[label+'_fixed']=compile_one(OUT/(label+'_fixed.cpp.txt'),OUT/(label+'_fixed'),flags,label+'_fixed')
    results['probe']=compile_one(ROOT/'adhoc/bin/probe_v039_board.cpp',OUT/'probe',['-fsanitize=undefined','-fsanitize-undefined-trap-on-error'],'probe')
    results['bench']=compile_one(ROOT/'adhoc/bin/bench_v039_exact_board.cpp',OUT/'bench',DEFINES['production'],'bench')
    (OUT/'build_summary.json').write_text(json.dumps(results,indent=2)+'\n')


def freeze():
    hashes=json.loads((OUT/'protected.json').read_text())
    for name,want in hashes.items():assert digest(ROOT/name)==want,name
    for p in sorted(OUT.rglob('*')):
        if p.is_file():hashes[str(p.relative_to(ROOT))]=digest(p)
    for pattern in ('adhoc/scripts/*v039*','adhoc/bin/*v039*'):
        for p in ROOT.glob(pattern):hashes[str(p.relative_to(ROOT))]=digest(p)
    (OUT/'frozen.json').write_text(json.dumps({'hashes':hashes,'compiler':subprocess.check_output(['g++-15','--version'],text=True).splitlines()[0]},indent=2)+'\n')
    print('Frozen',len(hashes),'files',flush=True)


if __name__=='__main__':
    {'prepare':prepare,'build':build,'freeze':freeze}[sys.argv[1]]()
