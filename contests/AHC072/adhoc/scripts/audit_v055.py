#!/usr/bin/env python3
"""Build and inspect v055 without executing its search."""
from concurrent.futures import ThreadPoolExecutor
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v055_audit'
PARENT = 'v050_joint_towers'
CHILD = 'v055_final_reductions'
FLAGS = ['-std=gnu++23', '-O2', '-Wall', '-Wextra', '-march=native', '-pthread',
         '-ftrivial-auto-var-init=zero', '-fopenmp']
MODES = {'local': ['-DLOCAL'], 'production': ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX']}


def run(args):
    result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
    if result.returncode or result.stderr:
        raise RuntimeError(result.stderr)
    return result.stdout


def strip_additions(source):
    source = source.replace('// v055_final_reductions.cpp', '// v050_joint_towers.cpp', 1)
    source = source.replace('constexpr double JUDGE_TIME_LIMIT_SEC = 1.90;\nconstexpr double LOCAL_TIME_RATIO = 1.0;\nconstexpr double PROGRAM_TIME_LIMIT_SEC = JUDGE_TIME_LIMIT_SEC;\n', '', 1)
    for start, end in [('// v055 additions:', '// End v055 additions.\n\n'),
                       ('    // v055 final-only orchestration;', '    // End v055 final call.\n')]:
        a = source.index(start)
        b = source.index(end, a) + len(end)
        source = source[:a] + source[b:]
    return source


def normalize(source):
    source = source.replace(PARENT, 'solver').replace(CHILD, 'solver')
    return re.sub(r'("[^"\n]*solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', source)


def assembly_calls(path):
    source = path.read_text()
    symbol = '__ZN11TemporalLNS8insertAtERK5BoardRKSt6vectorI4MoveSaIS4_EEiiiiRS6_'
    if symbol + ':\n' not in source:
        return {'available': False}
    body = re.split(r'^LFE\d+:', source.split(symbol + ':\n', 1)[1], maxsplit=1, flags=re.M)[0]
    targets = re.findall(r'^\tbl\t([^\n]+)', body, flags=re.M)
    return {'available': True, 'calls': len(targets),
            'board_copy_calls': sum('BoardC1ERKS_' in t or 'BoardC2ERKS_' in t for t in targets),
            'mono_color_calls': sum('monoColor' in t for t in targets),
            'clock_check_calls': sum('Clock5check' in t for t in targets)}


def build(mode):
    flags = [*FLAGS, *MODES[mode]]
    child = ROOT / f'src/bin/{CHILD}.cpp'
    binary = OUT / f'{CHILD}_{mode}'
    run(['g++-15', *flags, str(child), '-o', str(binary)])
    run(['g++-15', *flags, '-S', str(child), '-o', str(OUT / f'v055_{mode}.s')])
    expanded = []
    for name in (PARENT, CHILD):
        text = run(['g++-15', *flags, '-E', '-P', str(ROOT / f'src/bin/{name}.cpp')])
        (OUT / f'{name}_{mode}.ii').write_text(text)
        expanded.append(normalize(text))
    difference = ''.join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True)))
    (OUT / f'{mode}_preprocessed.diff').write_text(difference)
    # Remove registered source blocks, preprocess them again under the same
    # filename, then compare actual macro-expanded code including assertions.
    stripped = strip_additions(child.read_text())
    result = subprocess.run(['g++-15', *flags, '-E', '-P', '-x', 'c++', '-'],
                            input=stripped, capture_output=True, text=True, cwd=ROOT, check=True)
    assert not result.stderr, result.stderr
    expanded_stripped = normalize(result.stdout).replace('"<stdin>"', '"solver.cpp"')
    expanded_stripped = re.sub(r'("[^"\n]*solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', expanded_stripped)
    assert expanded_stripped == expanded[0], f'{mode}: unexpected preprocessed change'
    report = {'unchanged_parent_after_stripping': True,
              'diff_lines': len(difference.splitlines()), 'binary_bytes': binary.stat().st_size,
              'child_insertAt': assembly_calls(OUT / f'v055_{mode}.s'),
              'parent_insertAt': assembly_calls(ROOT / f'adhoc/v053_audit/v050_{mode}.s')}
    print(mode, 'build and inspection complete', flush=True)
    return mode, report


def main():
    OUT.mkdir(exist_ok=True)
    parent = ROOT / f'src/bin/{PARENT}.cpp'
    child = ROOT / f'src/bin/{CHILD}.cpp'
    assert strip_additions(child.read_text()) == parent.read_text()
    digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    assert digest(parent) == '22d846f1a4dacc816c4634fdc2755af13f5f329ca5398bf21abbc695e670c421'
    (OUT / 'source.diff').write_text(''.join(difflib.unified_diff(parent.read_text().splitlines(True), child.read_text().splitlines(True))))
    with ThreadPoolExecutor(max_workers=2) as pool:
        modes = dict(pool.map(build, MODES))
    report = {'solver_sha256': digest(child), 'parent_sha256': digest(parent), 'modes': modes,
              'compiler': run(['g++-15', '--version']).splitlines()[0],
              'deployment_target': os.environ.get('MACOSX_DEPLOYMENT_TARGET'), 'sdk': os.environ.get('SDKROOT'),
              'input_sha256': {p.name: digest(p) for p in sorted((ROOT / 'tools/in').glob('*.txt'))},
              'parent_output_sha256': {p.name: digest(p) for p in sorted((ROOT / 'results/out' / PARENT).glob('*.txt'))},
              'component_sha256': {p.name: digest(p) for p in [ROOT / f'src/bin/{n}.cpp' for n in (
                  'v048_mono_dispatch', 'v049_coupled_routes', 'v051_joint_polish',
                  'v052_finite_cooperation', 'v054_route_slack')]}}
    (OUT / 'static_verification.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if not k.endswith('_sha256') or isinstance(v,str)}, indent=2))


if __name__ == '__main__':
    main()
