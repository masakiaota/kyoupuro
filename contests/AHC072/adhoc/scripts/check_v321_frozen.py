#!/usr/bin/env python3
"""深さ0の提出物を親と照合。固定時計の診断はスコア比較に使用しない。"""
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

from train_v321_router import fit, tune, shortlist

ROOT = Path(__file__).resolve().parents[2]


def main():
    run = Path(sys.argv[1]).resolve()
    manifest = json.loads((run/'manifest.json').read_text())
    cv = json.loads((run/'cv_result.json').read_text())
    rows = json.loads((run/'training_data.json').read_text())
    out = run/'mechanism_check'
    out.mkdir(exist_ok=True)
    result = {'compiler': '', 'body_equal_v113': False, 'modes': [],
              'fixed_clock_comparisons': [], 'diagnostic_solver_executions': 0}
    # 各外側の評価スコアを壊しても、内側の選択と訓練結果が変わらない。
    configs = [(d, n) for d in manifest['depths'] for n in manifest['min_leaves']]
    for fold in cv['outer']:
        mutated = copy.deepcopy(rows)
        train_names = set(fold['train'])
        indices = [i for i, r in enumerate(rows) if r['case'] in train_names]
        for r in mutated:
            if r['case'] not in train_names:
                r['scores'] = {v: 10000000*(i+1) for i, v in enumerate(sorted(manifest['candidates']))}
        chosen, _, _ = tune(mutated, indices, sorted(manifest['candidates']), 4, configs)
        pool = shortlist(mutated, indices, sorted(manifest['candidates']))
        assert chosen == fold['chosen'] and pool == fold['candidates']
        assert fit(mutated, indices, pool, chosen['depth'], chosen['min_leaf']) == fold['tree']
    result['outer_held_label_mutation_checks'] = 5
    a = ROOT/'src/bin/v113_integrated_nn_lns.cpp'
    b = ROOT/'src/bin/v321_cv_portfolio.cpp'
    assert a.read_bytes().split(b'\n', 1)[1] == b.read_bytes().split(b'\n', 3)[3]
    assert hashlib.sha256(b.read_bytes()).hexdigest() == manifest['final_source_sha256']
    result['body_equal_v113'] = True
    env = os.environ.copy()
    if sys.platform == 'darwin':
        env.setdefault('SDKROOT', subprocess.check_output(['xcrun', '--show-sdk-path'], text=True).strip())
        env.setdefault('MACOSX_DEPLOYMENT_TARGET', '15.0')
    cxx = shutil.which(env.get('CXX', 'g++-15'))
    assert cxx
    result['compiler'] = subprocess.check_output([cxx, '--version'], text=True).splitlines()[0]
    flags = ['-std=gnu++23', '-O2', '-Wall', '-Wextra', '-march=native', '-pthread',
             '-ftrivial-auto-var-init=zero', '-fopenmp']
    cases = sorted(rows, key=lambda r: r['sha256'])
    cases = [cases[0]['case'], cases[-1]['case']]
    for mode, macros in [('local', ['-DLOCAL']), ('judge', ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX'])]:
        expanded = []
        for tag, source in [('parent', a), ('final', b)]:
            prefix = out/f'{tag}_{mode}'
            with prefix.with_suffix('.build.log').open('w') as log:
                subprocess.run([cxx, *flags, *macros, str(source), '-o', str(prefix)],
                               env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
            text = subprocess.check_output([cxx, *flags, *macros, '-E', '-P', str(source)], env=env).decode()
            text = text.replace(str(source), 'SOURCE.cpp').replace(source.name, 'SOURCE.cpp')
            # __assert_rtnの診断行番号だけを正規化。処理内の数値は触らない。
            text = re.sub(r'(__assert_rtn\s*\([^,\n]*,\s*"SOURCE.cpp",\s*)\d+(\s*,)', r'\g<1>0\2', text)
            (out/f'{tag}_{mode}.ii').write_text(text)
            expanded.append(text)
            diagnostic = source.read_text().replace('struct TimeKeeper {',
                'static unsigned long long fixed_clock_calls=0;\nstruct TimeKeeper {', 1)
            target = 'return chrono::duration<double>(chrono::steady_clock::now()-start_).count();'
            assert diagnostic.count(target) == 1
            diagnostic = diagnostic.replace(target, 'return (++fixed_clock_calls)*0.0001;')
            insertion = '    for(auto m:best)cout<<board_info.row[m.p]'
            position = diagnostic.rfind(insertion)
            assert position >= 0 and position > diagnostic.index('int main()')
            diagnostic = (diagnostic[:position]
                +'    cerr<<"\\nfixed_check calls="<<fixed_clock_calls<<" rng="<<rng.x<<"\\n";\n'
                +diagnostic[position:])
            diagnostic_path = ROOT/'adhoc/bin'/f'check_v321_{tag}_{mode}.cpp'
            diagnostic_path.write_text(diagnostic)
            binary = out/f'{tag}_{mode}_fixed'
            with (out/f'{tag}_{mode}_fixed.build.log').open('w') as log:
                subprocess.run([cxx, *flags, *macros, str(diagnostic_path), '-o', str(binary)],
                               env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
            for case in cases:
                with (ROOT/'tools/validation1'/case).open('rb') as inp:
                    completed = subprocess.run([str(binary)], stdin=inp, stdout=subprocess.PIPE,
                                               stderr=subprocess.PIPE, timeout=30, check=True)
                result['diagnostic_solver_executions'] += 1
                (out/f'{tag}_{mode}_{case}.out').write_bytes(completed.stdout)
                (out/f'{tag}_{mode}_{case}.err').write_bytes(completed.stderr)
        assert expanded[0] == expanded[1], mode+' full preprocessing differs'
        result['modes'].append({'mode': mode, 'build': 'passed', 'full_preprocessing_equal': True,
                               'normalized': 'diagnostic source filename and assert line number only'})
        for case in cases:
            parent = (out/f'parent_{mode}_{case}.out').read_bytes()
            final = (out/f'final_{mode}_{case}.out').read_bytes()
            assert parent == final
            pattern = rb'fixed_check calls=(\d+) rng=(\d+)'
            left = re.search(pattern, (out/f'parent_{mode}_{case}.err').read_bytes()).groups()
            right = re.search(pattern, (out/f'final_{mode}_{case}.err').read_bytes()).groups()
            assert left == right
            result['fixed_clock_comparisons'].append({'mode': mode, 'case': case,
                'output_sha256': hashlib.sha256(parent).hexdigest(), 'clock_calls': int(left[0]),
                'rng_state': int(left[1]), 'equal': True})
        print(mode, 'build/preprocessing/fixed-clock passed', flush=True)
    result['feature_runtime'] = 'No features or dispatch are needed for the selected constant model.'
    (out/'result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
