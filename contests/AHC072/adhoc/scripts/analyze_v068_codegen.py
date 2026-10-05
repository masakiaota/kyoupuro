#!/usr/bin/env python3
"""Inspect unchanged solver code generation and saved runs; execute no solver."""
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

from audit_v057 import FLAGS, MODES
from check_v037_results import log_values

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'results/analysis/v068/compiler_review_20260930'
VERSIONS = ('v058_joint_priority', 'v059_repair_priority', 'v068_size_selector')


def write_json(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def call(args, **kwargs):
    return subprocess.run(args, capture_output=True, text=True, check=True, cwd=ROOT, **kwargs)


def compile_one(task):
    version, mode = task
    stem = f'{version}_{mode}'
    command = ['g++-15', *FLAGS, *MODES[mode], '-S', str(ROOT / f'src/bin/{version}.cpp'),
               '-o', str(OUT / f'{stem}.s'),
               f'-fopt-info-inline-optimized-missed={OUT}/{stem}.inline',
               f'-fdump-lang-class={OUT}/{stem}.classes']
    result = call(command)
    (OUT / f'{stem}.compile.log').write_text(result.stderr)
    assert not result.stderr, result.stderr
    print(stem, 'compiled', flush=True)
    return {'version': version, 'mode': mode, 'command': command}


def inspect_assembly(path):
    text = path.read_text()
    functions = {}
    for match in re.finditer(r'^([_A-Za-z][^\s:]*):\nLFB\d+:\n(.*?)^LFE\d+:', text, re.M | re.S):
        symbol, body = match.groups()
        instructions = re.findall(r'^\t([a-z][a-z0-9.]*\s[^\n]*|ret)\s*$', body, re.M)
        calls = Counter(re.findall(r'^\tbl\t([^\n]+)', body, re.M))
        functions[symbol] = {'instructions': len(instructions), 'call_sites': sum(calls.values()),
                             'calls': dict(calls)}
    assert functions and '_main' in functions
    mangled = list(functions)
    names = call(['c++filt'], input='\n'.join(s[1:] if s.startswith('__Z') else s for s in mangled)).stdout.splitlines()
    assert len(names) == len(mangled)
    for symbol, name in zip(mangled, names):
        functions[symbol]['name'] = name
    return functions


def saved_runs():
    old58 = json.loads((ROOT / 'results/analysis/v058/summary.json').read_text())
    old58 = {r['case_name']: r for r in old58['cases']}
    old59 = ROOT / 'results/analysis/v059/20260929T235018_0cd76222/outputs'
    current = ROOT / 'results/analysis/v068/outputs'
    metrics = ('pre_lns_ops', 'pre_pair_ops', 'T', 'lns_attempts', 'lns_insertions',
               'single_insert_calls', 'packet_insert_calls', 'event_layers',
               'lns_accepted', 'lns_improvements', 'construction_attempts',
               'search_reductions_candidates', 'lns_repair_priority_evaluated')
    timings = ('construction', 'temporal_lns', 'lns_candidate_build', 'lns_repair_priority',
               'lns_two_order_primary_search', 'lns_two_order_alternate_search',
               'search_reductions_heavy')
    cases = []
    for path in sorted((ROOT / 'tools/in').glob('*.txt')):
        M = sum('a' <= c <= 'l' for row in path.read_text().splitlines()[1:] for c in row)
        new_c, new_t, errors = log_values(current / (path.name + '.err'))
        assert not errors
        if M < 80:
            old_c, old_t = old58[path.name]['counts'], old58[path.name]['times_ms']
        else:
            old_c, old_t, errors = log_values(old59 / (path.name + '.err'))
            assert not errors
        cases.append({'case': path.name, 'M': M, 'parent': 58 if M < 80 else 59,
                      'old_counts': {k: old_c.get(k, 0) for k in metrics},
                      'new_counts': {k: new_c.get(k, 0) for k in metrics},
                      'old_times': {k: old_t.get(k, 0) for k in timings},
                      'new_times': {k: new_t.get(k, 0) for k in timings}})
    groups = {}
    for parent in (58, 59):
        rows = [r for r in cases if r['parent'] == parent]
        report = {'cases': len(rows), 'counts': {}, 'times_ms': {}}
        for field, keys, dest in [('counts', metrics, 'counts'), ('times', timings, 'times_ms')]:
            for key in keys:
                old, new = (sum(r[p+'_'+field][key] for r in rows) for p in ('old', 'new'))
                report[dest][key] = {'parent': old, 'v068': new, 'delta': new-old,
                                     'change_percent': (100*(new/old-1)) if old else None}
        report['pre_lns_changed_cases'] = sum(r['old_counts']['pre_lns_ops'] != r['new_counts']['pre_lns_ops'] for r in rows)
        groups[str(parent)] = report
    write_json(OUT / 'saved_run_comparison.json', {'groups': groups, 'cases': cases})


def main():
    OUT.mkdir(parents=True, exist_ok=False)
    sources = [ROOT / f'src/bin/{name}.cpp' for name in VERSIONS]
    original = {str(p.relative_to(ROOT)): digest(p) for p in sources}
    if os.uname().sysname == 'Darwin':
        os.environ.setdefault('SDKROOT', call(['xcrun', '--show-sdk-path']).stdout.strip())
        os.environ.setdefault('MACOSX_DEPLOYMENT_TARGET', '15.0')
    with (ROOT / 'results/.eval.lock').open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with ThreadPoolExecutor(max_workers=2) as pool:
            builds = list(pool.map(compile_one, ((name, mode) for mode in MODES for name in VERSIONS)))
    summary = {}
    for build in builds:
        stem = build['version'] + '_' + build['mode']
        functions = inspect_assembly(OUT / f'{stem}.s')
        classes = (OUT / f'{stem}.classes').read_text()
        sizes = {}
        for name in ('Removal', 'SearchReductions', 'TemporalLNS'):
            m = re.search(r'^Class '+name+r'\n\s+size=(\d+) align=(\d+)', classes, re.M)
            assert m, name
            sizes[name] = {'bytes': int(m[1]), 'alignment': int(m[2])}
        hot = {s: f for s, f in functions.items() if s == '_main' or any(
            target in f['name'] for target in ('TemporalLNS::insertAt(', 'TemporalLNS::makeCandidates(',
                                             'TemporalLNS::optimize(', 'SearchReductions::candidate(',
                                             'TemporalLNS::brokenSupportJumps(', 'Constructor::run('))}
        summary[stem] = {'classes': sizes, 'emitted_functions': len(functions),
                         'total_instructions': sum(f['instructions'] for f in functions.values()), 'hot_functions': hot}
        write_json(OUT / f'{stem}.functions.json', functions)
    saved_runs()
    assert all(digest(ROOT / p) == h for p, h in original.items())
    write_json(OUT / 'codegen_summary.json', summary)
    write_json(OUT / 'manifest.json', {'sources': original, 'builds': builds,
                                      'compiler': call(['g++-15', '--version']).stdout.splitlines()[0],
                                      'SDKROOT': os.environ.get('SDKROOT'),
                                      'MACOSX_DEPLOYMENT_TARGET': os.environ.get('MACOSX_DEPLOYMENT_TARGET'),
                                      'solver_executions': 0})
    print('Read-only compiler and saved-result analysis completed.', flush=True)


if __name__ == '__main__':
    main()
