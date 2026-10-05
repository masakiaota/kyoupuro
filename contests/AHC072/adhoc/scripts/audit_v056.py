#!/usr/bin/env python3
"""Prepare and inspect v056; this script never runs the solver."""
from concurrent.futures import ThreadPoolExecutor
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v056_audit'
NAMES = {'parent': 'v055_final_reductions', 'child': 'v056_combined_speedups'}
FLAGS = ['-std=gnu++23', '-O2', '-Wall', '-Wextra', '-march=native', '-pthread',
         '-ftrivial-auto-var-init=zero', '-fopenmp']
MODES = {'local': ['-DLOCAL'], 'production': ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX']}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(args, **kwargs):
    result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, **kwargs)
    if result.returncode or result.stderr:
        raise RuntimeError(result.stderr)
    return result.stdout


def normalize(source):
    for name in NAMES.values():
        source = source.replace(name, 'solver')
    source = source.replace('"<stdin>"', '"solver.cpp"')
    source = re.sub(r'"[^"\n]*/solver.cpp"', '"solver.cpp"', source)
    return re.sub(r'("solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', source)


def calls(path, prefix=''):
    source = path.read_text()
    if not prefix:
        pattern = r'^(__ZN11TemporalLNS8insertAt[^:\n]*):\n'
    else:
        pattern = rf'^(__ZN{len(prefix)}{prefix}11TemporalLNS8insertAt[^:\n]*):\n'
    found = re.search(pattern, source, re.M)
    if not found:
        return {'available': False}
    body = re.split(r'^LFE\d+:', source[found.end():], maxsplit=1, flags=re.M)[0]
    targets = re.findall(r'^\tbl\t([^\n]+)', body, flags=re.M)
    return {'available': True, 'calls': len(targets),
            'board_copy_calls': sum('BoardC1ERKS' in t or 'BoardC2ERKS' in t for t in targets),
            'mono_color_calls': sum('monoColor' in t for t in targets),
            'clock_check_calls': sum('Clock5check' in t for t in targets)}


def prepare():
    assert not (OUT / 'frozen.json').exists()
    parent = (ROOT / f'src/bin/{NAMES["parent"]}.cpp').read_text()
    child = (ROOT / f'src/bin/{NAMES["child"]}.cpp').read_text()
    restored = child
    for change in reversed(json.loads((OUT / 'registered_changes.json').read_text())):
        assert restored.count(change['after']) == 1, change['label']
        restored = restored.replace(change['after'], change['before'], 1)
    assert restored == parent
    (OUT / 'restored_parent.cpp.txt').write_text(restored)
    for label, source in [('parent', parent), ('child', child)]:
        exposed = source.replace('class TemporalLNS {', 'class TemporalLNS {\npublic:', 1)
        exposed = exposed.replace('class Constructor {', 'class Constructor {\npublic:', 1)
        exposed = exposed.replace('int main() {', 'int retained_solver_entry() {', 1)
        # The old B representation names the global height function explicitly.
        # These copies live inside a namespace only in the diagnostic executable.
        exposed = exposed.replace('::height(', label + '::height(')
        assert exposed.endswith('}\n')
        exposed = exposed[:-2] + '    return 0;\n}\n'
        (OUT / f'{label}_exposed.cpp.txt').write_text(exposed)
        fixed = source.replace('    double elapsed() const {', '    mutable uint64_t ticks=0;\n    double elapsed() const {', 1)
        fixed = fixed.replace('return chrono::duration<double>(chrono::steady_clock::now()-start).count();',
                              'return double(++ticks)*0.000100003;', 1)
        fixed = fixed[:-2] + '    cerr<<"[fixed.rng] "<<rng.x<<"\\n[fixed.ticks] "<<clk.ticks<<"\\n";\n}\n'
        (OUT / f'{label}_fixed.cpp.txt').write_text(fixed)
    print('Prepared diagnostic copies; registered source changes verified', flush=True)


def build_mode(mode):
    flags = [*FLAGS, *MODES[mode]]
    expanded = []
    for label, name in NAMES.items():
        source = ROOT / f'src/bin/{name}.cpp'
        if label == 'child':
            run(['g++-15', *flags, str(source), '-o', str(OUT / f'{label}_{mode}')])
            run(['g++-15', *flags, '-S', str(source), '-o', str(OUT / f'{label}_{mode}.s')])
        text = run(['g++-15', *flags, '-E', '-P', str(source)])
        (OUT / f'{label}_{mode}.ii').write_text(text)
        expanded.append(normalize(text))
    restored = run(['g++-15', *flags, '-E', '-P', '-x', 'c++', '-'],
                   input=(OUT / 'restored_parent.cpp.txt').read_text())
    assert normalize(restored) == expanded[0], mode
    difference = ''.join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True)))
    (OUT / f'{mode}_preprocessed.diff').write_text(difference)
    report = {'registered_changes_only': True, 'diff_lines': len(difference.splitlines()),
              'parent_insertAt': calls(ROOT / f'adhoc/v055_audit/v055_{mode}.s'),
              'child_insertAt': calls(OUT / f'child_{mode}.s')}
    assert report['child_insertAt']['available'] and report['child_insertAt']['board_copy_calls'] == 0, report
    print(mode, report, flush=True)
    return mode, report


def build():
    with ThreadPoolExecutor(max_workers=2) as pool:
        modes = dict(pool.map(build_mode, MODES))
    report = {'modes': modes, 'compiler': run(['g++-15', '--version']).splitlines()[0],
              'sources': {name: digest(ROOT / f'src/bin/{name}.cpp') for name in NAMES.values()},
              'sdk': os.environ.get('SDKROOT'), 'deployment_target': os.environ.get('MACOSX_DEPLOYMENT_TARGET')}
    (OUT / 'static_verification.json').write_text(json.dumps(report, indent=2) + '\n')


def helpers():
    jobs = [(OUT / f'{label}_fixed.cpp.txt', OUT / f'{label}_fixed',
             ['-DLOCAL', '-x', 'c++', '-fsanitize=undefined', '-fsanitize-undefined-trap-on-error']) for label in NAMES]
    jobs += [(ROOT / 'adhoc/bin/bench_v056_speedups.cpp', OUT / 'bench', MODES['production']),
             (ROOT / 'adhoc/bin/check_v056_speedups.cpp', OUT / 'check',
              ['-DLOCAL', '-fsanitize=undefined', '-fsanitize-undefined-trap-on-error'])]
    def one(job):
        source, dest, extra = job
        run(['g++-15', *FLAGS, *extra, str(source), '-o', str(dest)])
        print('built', dest.name, flush=True)
    with ThreadPoolExecutor(max_workers=2) as pool:
        list(pool.map(one, jobs))
    run(['g++-15', *FLAGS, *MODES['production'], '-S', str(ROOT / 'adhoc/bin/bench_v056_speedups.cpp'), '-o', str(OUT / 'bench.s')])
    report = {label: calls(OUT / 'bench.s', label) for label in NAMES}
    (OUT / 'bench_assembly.json').write_text(json.dumps(report, indent=2) + '\n')
    print('bench assembly', report, flush=True)


def freeze():
    paths = [ROOT / f'src/bin/{name}.cpp' for name in NAMES.values()]
    paths += [ROOT / 'src/bin/v000_template.cpp', ROOT / 'notes/notations.md', ROOT / 'notes/important_properties.md', ROOT / 'README.md']
    paths += list((ROOT / 'tools/in').glob('*.txt'))
    paths += list((ROOT / 'results/out' / NAMES['parent']).glob('*.txt*'))
    paths += list((ROOT / 'adhoc/bin').glob('*v056*')) + list((ROOT / 'adhoc/scripts').glob('*v056*'))
    paths += [p for p in OUT.glob('*') if p.is_file() and p.name != 'frozen.json']
    (OUT / 'frozen.json').write_text(json.dumps({str(p.relative_to(ROOT)): digest(p) for p in paths}, indent=2) + '\n')
    print('Frozen', len(paths), 'files', flush=True)


if __name__ == '__main__':
    {'prepare': prepare, 'build': build, 'helpers': helpers, 'freeze': freeze}[sys.argv[1]]()
