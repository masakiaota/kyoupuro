#!/usr/bin/env python3
"""Compile and inspect v053 without executing a solution program."""
from concurrent.futures import ThreadPoolExecutor
import difflib
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v053_audit'
PARENT = 'v052_finite_cooperation'
CHILD = 'v053_cold_finite_cooperation'
FLAGS = ['-std=gnu++23', '-O2', '-Wall', '-Wextra', '-march=native', '-pthread',
         '-ftrivial-auto-var-init=zero', '-fopenmp']
MODES = {'local': ['-DLOCAL'], 'production': ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX']}
SYMBOLS = {
    'insertAt': '__ZN11TemporalLNS8insertAtERK5BoardRKSt6vectorI4MoveSaIS4_EEiiiiRS6_',
    'optimize': '__ZN11TemporalLNS8optimizeERSt6vectorI4MoveSaIS1_EE',
    'constructor': '__ZN11Constructor3runEv',
    'planPair': '__ZN11Constructor8planPairERK13PairCandidate',
}


def run(args):
    result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, check=True)
    assert not result.stderr, result.stderr
    return result.stdout


def normalize(text):
    for name in (PARENT, CHILD):
        text = text.replace(name, 'solver')
    return re.sub(r'("[^"\n]*solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', text)


def assembly(path):
    text = path.read_text()
    result = {}
    for label, symbol in SYMBOLS.items():
        # Darwin GCC ends functions with LFE labels, not .cfi_endproc.
        body = re.split(r'^LFE\d+:', text.split(symbol + ':\n', 1)[1], maxsplit=1, flags=re.M)[0]
        instructions = [line.strip() for line in body.splitlines() if re.match(r'\t[a-z][a-z0-9.]*\s', line)]
        targets = [line.split('\t', 1)[1] for line in instructions if line.startswith('bl\t')]
        result[label] = dict(instructions=len(instructions), calls=len(targets),
                             board_copy_calls=sum('BoardC1ERKS_' in t or 'BoardC2ERKS_' in t for t in targets),
                             clock_check_calls=sum('Clock5check' in t for t in targets),
                             vector_move_calls=sum('M_move_assign' in t for t in targets),
                             reverse_calls=sum('reverseWord' in t for t in targets),
                             join_calls=sum('joinWord' in t for t in targets))
    return result


def build(mode):
    defines = MODES[mode]
    binary = OUT / f'{CHILD}_{mode}'
    source = ROOT / f'src/bin/{CHILD}.cpp'
    # Outputs stay in this audit directory so simultaneous builds cannot race.
    run(['g++-15', *FLAGS, *defines, str(source), '-o', str(binary)])
    run(['g++-15', *FLAGS, *defines, '-S', str(source), '-o', str(OUT / f'v053_{mode}.s')])
    expanded = []
    for name in (PARENT, CHILD):
        text = run(['g++-15', *FLAGS, *defines, '-E', '-P', str(ROOT / f'src/bin/{name}.cpp')])
        (OUT / f'{name}_{mode}.ii').write_text(text)
        expanded.append(normalize(text))
    difference = ''.join(difflib.unified_diff(expanded[0].splitlines(True), expanded[1].splitlines(True)))
    (OUT / f'{mode}_preprocessed.diff').write_text(difference)
    stripped = expanded[1].replace('[[gnu::cold, gnu::noinline, gnu::optimize("Os")]] ', '')
    assert stripped == expanded[0], 'change beyond function attributes'
    result = dict(only_attributes_changed=True, sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                  binary_bytes=binary.stat().st_size, assembly={})
    for version in ('v050', 'v052', 'v053'):
        result['assembly'][version] = assembly(OUT / f'{version}_{mode}.s')
    result['required_copy_inlining_restored'] = result['assembly']['v053']['insertAt']['board_copy_calls'] == 0
    print(mode, 'static inspection complete', flush=True)
    return mode, result


def main():
    hashes = {name: hashlib.sha256((ROOT / f'src/bin/{name}.cpp').read_bytes()).hexdigest()
              for name in ('v050_joint_towers', PARENT, CHILD)}
    assert hashes[PARENT] == 'b843447ff5a6050f29b9731d760a22a9cc90d67bff3dc3ed5c4c9779d5db2da6'
    assert hashes['v050_joint_towers'] == '22d846f1a4dacc816c4634fdc2755af13f5f329ca5398bf21abbc695e670c421'
    parent = (ROOT / f'src/bin/{PARENT}.cpp').read_text()
    child = (ROOT / f'src/bin/{CHILD}.cpp').read_text()
    (OUT / 'source.diff').write_text(''.join(difflib.unified_diff(parent.splitlines(True), child.splitlines(True))))
    with ThreadPoolExecutor(max_workers=2) as pool:
        modes = dict(pool.map(build, MODES))
    result = dict(source_sha256=hashes, compiler=run(['g++-15', '--version']).splitlines()[0],
                  sdk=os.environ.get('SDKROOT'), deployment_target=os.environ.get('MACOSX_DEPLOYMENT_TARGET'),
                  modes=modes, solver_executions=0, evaluation_executions=0)
    result['mechanism_passed'] = all(m['required_copy_inlining_restored'] for m in modes.values())
    (OUT / 'static_verification.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
