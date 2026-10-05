#!/usr/bin/env python3
"""NN側target指定の追加効果を生成物で照合する。solverは実行しない。"""
import argparse
import difflib
import hashlib
import json
from pathlib import Path
import shutil
import subprocess


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    result_path = out / 'result.json'
    assert not result_path.exists(), '既存の診断結果を上書きしない'
    original = args.source.read_bytes()
    assert sha(original) == '1dce487ed293ba3e8cd44cf93ecddc8ed0e8b5cb3ac6f42d48ff15fd50769141'
    pragma = b'#pragma GCC target("avx2,bmi,bmi2,lzcnt,popcnt")'
    assert original.count(pragma) == 2
    # 同じ行数と同じビルドパスで、NN側の指定だけを除く。
    removed = original.replace(pragma, b'// NN target pragma removed for isolated compilation check.', 1)
    compiler = shutil.which('g++-15')
    assert compiler
    flags = ['-std=gnu++23', '-O2', '-Wall', '-Wextra', '-march=native',
             '-pthread', '-fopenmp', '-ftrivial-auto-var-init=zero',
             '-fconstexpr-depth=1024', '-fconstexpr-loop-limit=524288',
             '-fconstexpr-ops-limit=2097152', '-fmodules',
             '-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX']
    result = {'compiler': subprocess.check_output([compiler, '--version'], text=True).splitlines()[0],
              'flags': flags, 'source_sha256': sha(original), 'solver_executions': 0, 'conditions': {}}
    (out / 'compiler_target_options.txt').write_bytes(subprocess.check_output(
        [compiler, '-march=native', '-Q', '--help=target']))
    work = out / 'build'
    work.mkdir(exist_ok=True)
    for label, source in [('specified', original), ('removed', removed)]:
        (work / 'Main.cpp').write_bytes(source)
        with (out / f'{label}.build.log').open('wb') as log:
            for mode, output in [('preprocess', 'Main.ii'), ('assembly', 'Main.s'), ('binary', 'a.out')]:
                extra = {'preprocess': ['-E', '-P'], 'assembly': ['-S'], 'binary': []}[mode]
                subprocess.run([compiler, *flags, *extra, 'Main.cpp', '-o', output],
                               cwd=work, stdout=log, stderr=subprocess.STDOUT, check=True)
                shutil.copy2(work / output, out / f'{label}.{output}')
        result['conditions'][label] = {suffix: sha((out / f'{label}.{suffix}').read_bytes())
                                      for suffix in ['Main.ii', 'Main.s', 'a.out']}
        print(label, 'compiled', flush=True)
    pp_on = (out / 'specified.Main.ii').read_bytes()
    pp_off = (out / 'removed.Main.ii').read_bytes()
    assert pp_on.count(pragma + b'\n') == 2
    assert pp_on.replace(pragma + b'\n', b'', 1) == pp_off, '意図しない前処理差分'
    result['preprocessing_difference_nn_target_only'] = True
    result['assembly_byte_identical'] = (out / 'specified.Main.s').read_bytes() == (out / 'removed.Main.s').read_bytes()
    result['binary_byte_identical'] = (out / 'specified.a.out').read_bytes() == (out / 'removed.a.out').read_bytes()
    if not result['assembly_byte_identical']:
        (out / 'assembly.diff').write_text(''.join(difflib.unified_diff(
            (out / 'specified.Main.s').read_text().splitlines(True),
            (out / 'removed.Main.s').read_text().splitlines(True))))
    result_path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result, ensure_ascii=False, indent=2), flush=True)


if __name__ == '__main__':
    main()
