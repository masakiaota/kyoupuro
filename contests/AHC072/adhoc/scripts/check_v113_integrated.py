#!/usr/bin/env python3
"""既存機構の移植を固定した保存解で照合する。性能選別は行わない。"""
import difflib
import json
from pathlib import Path
import re
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor

from build_v113_integrated import ROOT, RUN, nn_region
from check_v089_board import compile_binary
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save, sha, now, status

BASE = ROOT/'src/bin/v111_weight_portfolio.cpp'
SAVED = ROOT/'results/nn_rank/v111/20261004_weight_portfolio_studio/learned/evaluation/validation'


def load(path):
    return json.loads(path.read_text())


def normalize(text):
    text = re.sub(r'"[^"\n]*\.cpp"', '"source.cpp"', text)
    return re.sub(r'("source.cpp",\s*)\d+', r'\g<1>0', text)


def block(text, start):
    begin = text.index(start); left = text.index('{', begin)
    # 文字列中のJSON波括弧やコメントをブロック境界に数えない。
    token = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|//[^\n]*|/\*.*?\*/|[{}]', re.S)
    depth = 0
    for found in token.finditer(text, left):
        value = found.group()
        if value == '{': depth += 1
        elif value == '}':
            depth -= 1
            if depth == 0: return text[begin:found.end()]
    raise ValueError(start)


def check(root=RUN):
    where = root/'mechanism'; where.mkdir(parents=True, exist_ok=True)
    marker = where/'result.json'; manifest = load(root/'sources.json')
    sources = {'base': BASE, **{k: ROOT/v['path'] for k,v in manifest['sources'].items()}}
    if marker.exists():
        result = load(marker)
        assert all(result['sources'].get(k)==sha(v) for k,v in sources.items() if k!='hrp')
        if 'hrp' in sources and not result.get('v319_original_preprocessing_identical'):
            # 条件固定後にGitへ到着した既成版を、通常評価開始前に追加した。
            source=sources['hrp'];dest=where/'hrp';dest.mkdir(exist_ok=True)
            assert sha(source)=='931b5b40e4595c9a500f1d5a2f978e067fc867a26137d8b9a55f4d13e5ee8cbb'
            packed=lambda text: re.search(r'static constexpr char nn_packed\[\] =.*?;',text,re.S).group()
            assert packed(source.read_text())==packed(BASE.read_text())
            for local in (True,False):
                mode='local' if local else 'judge'
                compile_binary(source.stem,dest/('solver_'+mode),local)
                value=normalize(preprocess(source,dest/(mode+'.ii'),local))
                original=normalize(preprocess(root/'frozen'/source.name,dest/('original_'+mode+'.ii'),local))
                assert value==original,(mode,'v319 changed on import')
            result['sources']['hrp']=sha(source)
            result['v319_original_preprocessing_identical']=True
            result['v319_weights_identical_to_v111']=True
            result['v319_completed_at']=now();save(marker,result)
        assert result['sources']=={k:sha(v) for k,v in sources.items()}
        return result
    for key,path in sources.items():
        if key!='hrp':assert nn_region(path.read_text()) == nn_region(BASE.read_text())
        if key != 'base': assert sha(path) == manifest['sources'][key]['sha256']
    status(root/'pipeline', 'mechanism_builds')
    # 同一bin名のLOCAL/非LOCALは逐次、独立した条件間だけを並列ビルドする。
    def build_one(item):
        label,source = item; dest=where/label;dest.mkdir(exist_ok=True)
        for local in (True,False):
            mode='local' if local else 'judge'
            artifacts=[dest/('solver_'+mode),dest/(mode+'.ii')]
            if all(p.exists() and p.stat().st_mtime>=source.stat().st_mtime for p in artifacts):
                continue  # 同じ固定ソースで完了済みのビルド・前処理を再利用する。
            compile_binary(source.stem,dest/('solver_'+mode),local)
            preprocess(source,dest/(mode+'.ii'),local)
        return label
    with ThreadPoolExecutor(max_workers=3) as pool:
        list(pool.map(build_one,sources.items()))
    # 完全な前処理差分を保存し、中心変更の外にあるNN・時計・構築・短縮を照合する。
    blocks=['namespace nn {','struct TimeKeeper','struct Board {','struct NNSharedDeadline',
            'static pair<int,int> neural_transform_cell']
    for local in (True,False):
        mode='local' if local else 'judge'; reference=normalize((where/'base'/(mode+'.ii')).read_text())
        for label in manifest['sources']:
            if label=='hrp':continue
            expanded=normalize((where/label/(mode+'.ii')).read_text())
            for name in blocks:
                assert block(expanded,name)==block(reference,name), (label,mode,name)
            (where/label/(mode+'.patch')).write_text(''.join(difflib.unified_diff(reference.splitlines(True),expanded.splitlines(True),fromfile='v111',tofile=label)))

    cases=load(SAVED/'input_manifest.json')
    selected=[cases[i]['filename'] for i in [0,31,63,95,127,159,191,223]]
    summary={'features':0,'synthetic':0,'ordinary':0,'conditioned':0,'demands':0,'race_checks':0}
    for label,features in [('base',''),('hrs','HRS')]:
        wrapper=ROOT/'adhoc/bin'/f'check_v113_{label}.cpp'
        wrapper.write_text('// '+wrapper.name+'\n'+''.join('#define V113_'+f+'\n' for f in features)+
                           '#define V113_SOURCE "../../'+str(sources[label].relative_to(ROOT))+'"\n'+
                           '#include "check_v113_components.cpp"\n')
        binary=where/label/'components';compile_binary(wrapper.stem,binary,True)
        for case in selected:
            out=where/label/'components_outputs'/case
            proc=subprocess.run([binary,SAVED/'outputs'/case,out],input=(SAVED/'inputs'/case).read_text(),
                                text=True,capture_output=True,timeout=60)
            (where/label/('components_'+case+'.err')).write_text(proc.stderr)
            assert proc.returncode==0, (label,case,proc.stderr[-2000:])
            result=load(out/'result.json')
            if label=='hrs':
                old=where/'base/components_outputs'/case
                assert (out/'features.txt').read_bytes()==(old/'features.txt').read_bytes(), (case,'features or RNG')
                assert (out/'ordinary.txt').read_bytes()==(old/'ordinary.txt').read_bytes(), (case,'ordinary status')
                for plan in old.glob('ordinary_*.txt'):
                    assert (out/plan.name).read_bytes()==plan.read_bytes(), (case,plan.name)
                for key in summary:summary[key]+=result[key]
                for plan in out.glob('conditioned_*.txt'):
                    assert validated_output(SAVED/'inputs'/case,plan.read_text())['E']==0
    assert summary['conditioned']>0 and summary['ordinary']>0 and summary['features']>0

    # Hは時計を読まない集計置換なので、同じ固定時計の完成列とRNGも一致する。
    fixed=[]
    for label in ('base','h'):
        wrapper=ROOT/'adhoc/bin'/f'check_v113_clock_{label}.cpp'
        wrapper.write_text('// '+wrapper.name+'\n#define AUDIT_BOARD board_info\n#define AUDIT_POOL state_pool\n'+
                           '#define AUDIT_SOURCE "../../'+str(sources[label].relative_to(ROOT))+'"\n'+
                           '#include "check_v071_fixed_clock.cpp"\n')
        for local in (True,False):
            mode='local' if local else 'judge';binary=where/label/('clock_'+mode)
            compile_binary(wrapper.stem,binary,local)
            for case in selected[:4]:
                proc=subprocess.run([binary],input=(SAVED/'inputs'/case).read_text(),text=True,capture_output=True,timeout=30)
                assert proc.returncode==0,proc.stderr[-1000:]
                dest=where/label/f'clock_{mode}_{case}';dest.write_text(proc.stdout)
                dest.with_suffix('.err').write_text(proc.stderr)
                assert validated_output(SAVED/'inputs'/case,proc.stdout)['E']==0
                audit=re.search(r'\[refactor.audit\] (.+)',proc.stderr).group(1)
                if label=='h':
                    old=where/'base'/dest.name
                    assert old.read_bytes()==dest.read_bytes(),(mode,case,'fixed clock output')
                    assert re.search(r'\[refactor.audit\] (.+)',old.with_suffix('.err').read_text()).group(1)==audit
                    fixed.append(dict(mode=mode,case=case,audit=audit))
    result=dict(passed=True,sources={k:sha(v) for k,v in sources.items()},components=summary,
                fixed_clock=fixed,unchanged_blocks=blocks,completed_at=now())
    save(marker,result)
    return check(root) if 'hrp' in sources else result


if __name__=='__main__':
    print(json.dumps(check(),ensure_ascii=False,indent=2))
