#!/usr/bin/env python3
"""Compile and inspect the frozen v067 experiment without running its search."""
from concurrent.futures import ThreadPoolExecutor
import difflib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

import audit_v057 as common

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v067_audit'
PARENT = 'v059_repair_priority'
CHILD = 'v067_repair_joint_priority'
common.PARENT, common.CHILD = PARENT, CHILD
FLAGS, MODES = common.FLAGS, common.MODES


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def strip_additions(source):
    for change in reversed(json.loads((OUT/'registered_changes.json').read_text())):
        assert source.count(change['after']) == 1
        source = source.replace(change['after'], change['before'], 1)
    return source


def build(mode):
    flags = [*FLAGS, *MODES[mode]]
    child = ROOT/f'src/bin/{CHILD}.cpp'
    binary = OUT/f'{CHILD}_{mode}'
    common.run(['g++-15', *flags, str(child), '-o', str(binary)])
    assembly = OUT/f'v067_{mode}.s'
    common.run(['g++-15', *flags, '-S', str(child), '-o', str(assembly)])
    parent_assembly = OUT/f'v059_{mode}.s'
    common.run(['g++-15', *flags, '-S', str(ROOT/f'src/bin/{PARENT}.cpp'), '-o', str(parent_assembly)])
    expanded = []
    for name in (PARENT, CHILD):
        source = common.run(['g++-15', *flags, '-E', '-P', str(ROOT/f'src/bin/{name}.cpp')])
        (OUT/f'{name}_{mode}.ii').write_text(source)
        expanded.append(common.normalize(source))
    difference = ''.join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True)))
    (OUT/f'{mode}_preprocessed.diff').write_text(difference)
    stripped = subprocess.run(['g++-15', *flags, '-E', '-P', '-x', 'c++', '-'],
                              input=strip_additions(child.read_text()), cwd=ROOT,
                              capture_output=True, text=True, check=True)
    assert not stripped.stderr, stripped.stderr
    normalized = common.normalize(stripped.stdout).replace('"<stdin>"', '"solver.cpp"')
    normalized = re.sub(r'("[^"\n]*solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', normalized)
    assert normalized == expanded[0], mode
    print(mode, 'build and preprocessing passed', flush=True)
    return mode, {'unchanged_parent_after_stripping': True, 'diff_lines': len(difference.splitlines()),
                  'binary_bytes': binary.stat().st_size, 'child_insertAt': common.assembly_calls(assembly),
                  'parent_insertAt': common.assembly_calls(parent_assembly)}


def main():
    prior = json.loads((OUT/'source_comparison.json').read_text())
    parent, child = [ROOT/f'src/bin/{name}.cpp' for name in (PARENT, CHILD)]
    assert digest(parent) == prior['parent_sha256'] and digest(child) == prior['solver_sha256']
    assert strip_additions(child.read_text()) == parent.read_text()
    assert not (OUT/'static_verification.json').exists()
    with (ROOT/'results/.eval.lock').open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with ThreadPoolExecutor(max_workers=2) as pool:
            modes = dict(pool.map(build, MODES))
    fixed = [ROOT/p for p in prior['protected_sha256']]
    fixed += [OUT/'registered_changes.json', OUT/'source_comparison.json', Path(__file__),
              ROOT/'adhoc/scripts/audit_v057.py', ROOT/'adhoc/scripts/check_v067_results.py',
              ROOT/'adhoc/scripts/check_v037_results.py', ROOT/'adhoc/scripts/check_v028_two_orders.py',
              ROOT/'adhoc/bin/check_v067_schedule.cpp', ROOT/'scripts/build_solver.sh', ROOT/'scripts/eval.py']
    inputs = sorted((ROOT/'tools/in').glob('*.txt'))
    outputs = sorted((ROOT/'results/out'/PARENT).glob('*.txt'))
    assert len(inputs) == len(outputs) == 100
    fixed += [p.with_suffix('.txt.err') for p in outputs]
    report = {'solver_sha256': digest(child), 'parent_sha256': digest(parent), 'modes': modes,
              'compiler': common.run(['g++-15', '--version']).splitlines()[0],
              'sdk': os.environ.get('SDKROOT'), 'deployment_target': os.environ.get('MACOSX_DEPLOYMENT_TARGET'),
              'input_sha256': {p.name: digest(p) for p in inputs},
              'parent_output_sha256': {p.name: digest(p) for p in outputs},
              'protected_sha256': {str(p.relative_to(ROOT)): digest(p) for p in fixed},
              'protected_changes_since_implementation': [p for p, value in prior['protected_sha256'].items()
                                                       if digest(ROOT/p) != value]}
    (OUT/'preregistration.md').write_text((ROOT/'notes/experiments/v067.md').read_text())
    (OUT/'static_verification.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({k:v for k,v in report.items() if not isinstance(v,dict) or k=='modes'}, indent=2))


if __name__ == '__main__':
    main()
