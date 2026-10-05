#!/usr/bin/env python3
"""Build, verify, and summarize the preregistered selector; never tune it."""
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from collections import Counter
import argparse
import csv
import difflib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

from audit_v057 import FLAGS, MODES
from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v068_audit'
DEST = ROOT / 'results/analysis/v068'
BIN = 'v068_size_selector'
PARENTS = {58: 'v058_joint_priority', 59: 'v059_repair_priority'}
RUNS = {58: '20260930T003629+0900_v058_joint_priority_7a1ed3',
        59: '20260929T235217+0900_v059_repair_priority_6879ca'}
KINDS = ('route', 'pair', 'joint', 'mono', 'strict', 'slack', 'finite')
SCHEDULES = {58: ('joint', 'joint', 'joint', 'strict', 'joint', 'joint', 'joint', 'slack',
                  'joint', 'joint', 'joint', 'finite'),
             59: ('joint', 'strict', 'slack', 'finite')}


def dump(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(args, **kwargs):
    result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, **kwargs)
    if result.returncode:
        raise RuntimeError(f'{args}: {result.returncode}\n{result.stdout}\n{result.stderr}')
    return result


@contextmanager
def compute_lock():
    with (ROOT / 'results/.eval.lock').open('a+') as handle:
        fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield


def normalize(source):
    for name in (*PARENTS.values(), BIN):
        source = source.replace(name, 'solver')
    source = source.replace('"<stdin>"', '"solver.cpp"')
    return re.sub(r'("[^"\n]*solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', source)


def input_M(path):
    rows = path.read_text().splitlines()
    N = int(rows[0].split()[0])
    return sum('a' <= c <= 'l' for row in rows[1:N+1] for c in row)


def build():
    assert not (OUT / 'frozen.json').exists()
    source = ROOT / f'src/bin/{BIN}.cpp'
    stripped = source.read_text()
    for change in reversed(json.loads((OUT / 'registered_changes.json').read_text())):
        assert stripped.count(change['after']) == 1
        stripped = stripped.replace(change['after'], change['before'], 1)
    assert stripped == (ROOT / f'src/bin/{PARENTS[59]}.cpp').read_text()

    def build_mode(mode):
        flags = [*FLAGS, *MODES[mode]]
        commands = []

        def compile_cmd(args):
            commands.append(args)
            result = run(args)
            assert not result.stderr, result.stderr
            return result.stdout

        compile_cmd(['g++-15', *flags, str(source), '-o', str(OUT / f'{BIN}_{mode}')])
        expanded = {}
        for name in (*PARENTS.values(), BIN):
            expanded[name] = normalize(compile_cmd(
                ['g++-15', *flags, '-E', '-P', str(ROOT / f'src/bin/{name}.cpp')]))
            (OUT / f'{name}_{mode}.ii').write_text(expanded[name])
        for number, name in PARENTS.items():
            difference = ''.join(difflib.unified_diff(
                expanded[name].splitlines(True), expanded[BIN].splitlines(True)))
            (OUT / f'{mode}_v{number:03d}_preprocessed.diff').write_text(difference)
        stripped_result = run(['g++-15', *flags, '-E', '-P', '-x', 'c++', '-'], input=stripped)
        assert not stripped_result.stderr
        assert normalize(stripped_result.stdout) == expanded[PARENTS[59]]
        for name in (*PARENTS.values(), BIN):
            compile_cmd(['g++-15', *flags, f'-DAUDIT_SOURCE="{ROOT}/src/bin/{name}.cpp"',
                         str(ROOT / 'adhoc/bin/probe_v068_fixed_clock.cpp'),
                         '-o', str(OUT / f'fixed_{name}_{mode}')])
        print(f'{mode}: build and full preprocessing passed', flush=True)
        return mode, {'registered_changes_only': True, 'commands': commands}

    with compute_lock():
        with ThreadPoolExecutor(max_workers=2) as pool:
            modes = dict(pool.map(build_mode, MODES))
        schedule_command = ['g++-15', *FLAGS, '-DLOCAL', '-fsanitize=undefined',
                            '-fsanitize-undefined-trap-on-error',
                            str(ROOT / 'adhoc/bin/check_v068_schedule.cpp'),
                            '-o', str(OUT / 'check_schedule')]
        checked = run(schedule_command)
        assert not checked.stderr, checked.stderr
    inputs = sorted((ROOT / 'tools/in').glob('*.txt'))
    assert len(inputs) == 100
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').open()]
    parents = {str(n): sorted((r for r in records if r['run_id'] == rid),
                             key=lambda r: r['case_name']) for n, rid in RUNS.items()}
    for rows in parents.values():
        assert len(rows) == 100 and all(r['status'] == 'ok' and r['local'] and
                                       r['input_dir'] == 'tools/in' for r in rows)
    dump(OUT / 'parent_records.json', parents)
    diagnostic_inputs = []
    for repair in (False, True):
        selected = [p for p in inputs if (input_M(p) >= 80) == repair][:3]
        assert len(selected) == 3
        diagnostic_inputs.extend(selected)
    synthetic = OUT / 'inputs'
    synthetic.mkdir()
    for M in (79, 80, 81):
        grid = [['.'] * 20 for _ in range(20)]
        for (i, j), color in zip(((0, 0), (0, 19), (19, 0), (19, 19)), 'ABCD'):
            grid[i][j] = color
        cells = [(i, j) for i in range(20) for j in range(20) if grid[i][j] == '.']
        for index, (i, j) in enumerate(cells[:M]):
            grid[i][j] = 'abcd'[index % 4]
        path = synthetic / f'M{M:03d}.txt'
        path.write_text('20 4\n' + '\n'.join(''.join(row) for row in grid) + '\n')
        diagnostic_inputs.append(path)
    shutil.copy2(ROOT / 'notes/experiments/v068.md', OUT / 'preregistration.md')
    protected = [source, *(ROOT / f'src/bin/{name}.cpp' for name in PARENTS.values()),
                 ROOT / 'src/bin/v000_template.cpp', ROOT / 'notes/notations.md',
                 ROOT / 'notes/important_properties.md', ROOT / 'README.md',
                 ROOT / 'scripts/eval.py', ROOT / 'scripts/build_solver.sh',
                 ROOT / 'adhoc/bin/check_v068_schedule.cpp',
                 ROOT / 'adhoc/bin/probe_v068_fixed_clock.cpp', Path(__file__),
                 ROOT / 'adhoc/scripts/audit_v057.py', ROOT / 'adhoc/scripts/check_v037_results.py',
                 ROOT / 'adhoc/scripts/check_v028_two_orders.py', OUT / 'registered_changes.json',
                 OUT / 'preregistration.md', OUT / 'parent_records.json', *inputs, *diagnostic_inputs]
    binaries = [OUT / f'{BIN}_{mode}' for mode in MODES]
    binaries += [OUT / f'fixed_{name}_{mode}' for mode in MODES for name in (*PARENTS.values(), BIN)]
    binaries += [OUT / 'check_schedule']
    frozen = {'sha256': {str(p.relative_to(ROOT)): sha(p) for p in protected + binaries},
              'diagnostic_inputs': [str(p.relative_to(ROOT)) for p in diagnostic_inputs],
              'modes': modes, 'schedule_command': schedule_command,
              'compiler': run(['g++-15', '--version']).stdout.splitlines()[0],
              'SDKROOT': os.environ.get('SDKROOT'),
              'MACOSX_DEPLOYMENT_TARGET': os.environ.get('MACOSX_DEPLOYMENT_TARGET')}
    dump(OUT / 'frozen.json', frozen)
    print('Frozen solver, parents, inputs, checks, and criteria before execution.', flush=True)


def unchanged():
    frozen = json.loads((OUT / 'frozen.json').read_text())
    for filename, expected in frozen['sha256'].items():
        assert sha(ROOT / filename) == expected, filename
    return frozen


def diagnostic():
    frozen = unchanged()
    assert not (OUT / 'diagnostic.json').exists()

    def compare(args):
        mode, filename = args
        path = ROOT / filename
        M = input_M(path)
        selected = 59 if M >= 80 else 58
        outputs = []
        for name in (PARENTS[selected], BIN):
            result = run([str(OUT / f'fixed_{name}_{mode}')], input=path.read_text(), timeout=60)
            saved = OUT / f'{path.stem}_{name}_{mode}.txt'
            saved.write_text(result.stdout)
            saved.with_suffix('.txt.err').write_text(result.stderr)
            verify_output(str(path), saved)
            outputs.append(result)
        assert outputs[0].stdout == outputs[1].stdout, (mode, filename, 'moves')
        stripped_stderr = re.sub(r'^\[summary.count\] selector_[^\n]*\n', '', outputs[1].stderr, flags=re.M)
        assert outputs[0].stderr == stripped_stderr, (mode, filename, 'counters/clock/RNG')
        assert 'diagnostic:' not in stripped_stderr
        fields = re.search(r'\[selector.audit\] clock_calls=(\d+) rng=(\d+) M=(\d+) pool_free=(\d+)',
                           stripped_stderr)
        assert fields and int(fields[3]) == M and int(fields[4]) == 4
        if mode == 'local':
            for key, value in (('M', M), ('v058', selected == 58), ('v059', selected == 59)):
                assert f'[summary.count] selector_{key}={int(value)}\n' in outputs[1].stderr
        return {'mode': mode, 'input': filename, 'M': M, 'selected': selected,
                'T': len(outputs[1].stdout.splitlines()), 'clock_calls': int(fields[1]),
                'rng': int(fields[2]), 'pool_free': int(fields[4]), 'exact_match': True}

    with compute_lock():
        schedule = run([str(OUT / 'check_schedule')], timeout=60)
        assert not schedule.stderr, schedule.stderr
        schedule_rows = [json.loads(line) for line in schedule.stdout.splitlines()]
        assert [r['mode'] for r in schedule_rows] == [58, 59]
        for r in schedule_rows:
            assert r['calls'] == r['validated_outputs'] == 24 and r['skipped'] == 4
            assert r['rng_consumption'] == 0 and r['pool_free'] == 4
            assert r['condition_checks'] == 4 and r['budget_checks'] == 3
            assert r['by_kind'] == dict(Counter(SCHEDULES[r['mode']] * (24 // len(SCHEDULES[r['mode']]))))
        with ThreadPoolExecutor(max_workers=2) as pool:
            comparisons = list(pool.map(compare, ((mode, name) for mode in MODES
                                                  for name in frozen['diagnostic_inputs'])))
    unchanged()
    result = {'schedule': schedule_rows, 'fixed_clock': comparisons, 'passed': True}
    dump(OUT / 'diagnostic.json', result)
    print(json.dumps({'schedule': schedule_rows, 'matched_pairs': len(comparisons), 'passed': True}))


def evaluation():
    unchanged()
    assert json.loads((OUT / 'diagnostic.json').read_text())['passed']
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').open()]
    current = sorted((r for r in records if r['bin'] == BIN and r['input_dir'] == 'tools/in'),
                     key=lambda r: r['case_name'])
    assert len(current) == 100 and len({r['run_id'] for r in current}) == 1
    parents = {int(n): {r['case_name']: r for r in rows} for n, rows in
               json.loads((OUT / 'parent_records.json').read_text()).items()}
    cases, counts, times, diagnostics = [], Counter(), Counter(), []
    for r in current:
        assert r['status'] == 'ok' and r['local'] and r['label'] == 'v068_m80_selector'
        path = ROOT / r['stdout_path']
        T = verify_output(r['case_name'], path)
        c, t, errors = log_values(path.with_suffix('.txt.err'))
        assert T == r['score'] == c['T'] == c['final_ops'] == c['validated_moves']
        assert c['E'] == 0 and c['board_pool_free_at_end'] == c['board_pool_slots']
        assert c['pre_joint_ops'] - c['pre_final_reductions_ops'] == c['joint_window_saved']
        assert c['pre_final_reductions_ops'] - T == c['final_reductions_saved']
        assert c['final_reductions_saved'] == sum(c['final_reductions_'+k+'_saved'] for k in KINDS)
        assert c['pre_pair_ops'] - c['pre_joint_ops'] == c['pair_transfer_saved'] - c['final_reductions_pair_saved'] - c['search_reductions_pair_saved']
        assert c['pre_lns_ops'] - c['pre_pair_ops'] == c['lns_saved']
        assert c['lns_saved'] == c['lns_initial_shortcut_saved'] + c['lns_initial_reorder_saved'] + c['lns_best_saved']
        assert t['search_limit'] == 1544.0
        M = input_M(ROOT / 'tools/in' / r['case_name'])
        selected = 59 if M >= 80 else 58
        assert c['selector_M'] == M and c['selector_v058'] == (selected == 58) and c['selector_v059'] == (selected == 59)
        cycle = SCHEDULES[selected]
        rounds, remaining = divmod(c['search_reductions_heavy_calls'], len(cycle))
        for k in ('joint', 'strict', 'slack', 'finite'):
            assert c['search_reductions_'+k+'_calls'] == rounds * cycle.count(k) + cycle[:remaining].count(k)
        if selected == 58:
            assert not any(v for k, v in c.items() if k.startswith('lns_repair_priority_'))
            assert sum(c['search_reductions_'+k+'_best_created_with'] for k in KINDS) >= c['search_reductions_best_created']
        for k in KINDS:
            assert 0 <= c['search_reductions_'+k+'_accepted_saved'] <= c['search_reductions_'+k+'_saved']
            if selected == 58:
                assert 0 <= c['search_reductions_'+k+'_best_created_with'] <= min(c['search_reductions_best_created'], c['search_reductions_'+k+'_accepted_saved'])
        row = {'case': r['case_name'], 'M': M, 'selected': selected, 'T': T, 'elapsed_ms': r['elapsed'],
               'v058_T': parents[58][r['case_name']]['score'], 'v059_T': parents[59][r['case_name']]['score'],
               'selected_parent_T': parents[selected][r['case_name']]['score']}
        minimum = min(row['v058_T'], row['v059_T'], T)
        for key in ('v058_T', 'v059_T', 'T', 'selected_parent_T'):
            row[key+'_relative'] = (2 * 10**9 * minimum + row[key]) // (2 * row[key])
        cases.append(row)
        counts.update(c); times.update(t); diagnostics.extend(errors)
    assert counts['selector_v058'] > 0 and counts['selector_v059'] > 0
    assert counts['lns_repair_priority_penalized'] > 0 and counts['lns_repair_priority_selected_broken_jumps'] > 0

    def aggregate(rows):
        summary = {'cases': len(rows)}
        for key in ('v058_T', 'v059_T', 'T', 'selected_parent_T'):
            summary[key] = sum(r[key] for r in rows)
            summary[key+'_relative'] = sum(r[key+'_relative'] for r in rows) / len(rows)
        for parent in ('v058_T', 'v059_T', 'selected_parent_T'):
            summary['delta_'+parent] = summary['T'] - summary[parent]
            summary['win_tie_loss_'+parent] = [sum(r['T'] < r[parent] for r in rows),
                                              sum(r['T'] == r[parent] for r in rows),
                                              sum(r['T'] > r[parent] for r in rows)]
        return summary

    whole = aggregate(cases)
    valid = not diagnostics and not any(counts[k] for k in ERRORS) and max(r['elapsed_ms'] for r in cases) <= 2000 and cases[0]['T'] <= 43
    result = {'run_id': current[0]['run_id'], 'input_set': 'tools/in', 'parent_runs': RUNS,
              'overall': whole, 'M_below80': aggregate([r for r in cases if r['M'] < 80]),
              'M_atleast80': aggregate([r for r in cases if r['M'] >= 80]),
              'all_outputs_legal_E0': True, 'schedule_verified_cases': 100,
              'errors': {k: counts[k] for k in ERRORS}, 'diagnostics': diagnostics,
              'case0000_T': cases[0]['T'], 'max_elapsed_ms': max(r['elapsed_ms'] for r in cases),
              'avg_elapsed_ms': sum(r['elapsed_ms'] for r in cases) / 100,
              'absolute_better_than_both': whole['T'] < min(whole['v058_T'], whole['v059_T']),
              'relative_atleast_both': whole['T_relative'] >= max(whole['v058_T_relative'], whole['v059_T_relative']),
              'validity_passed': valid}
    result['adopt'] = valid and result['absolute_better_than_both'] and result['relative_atleast_both']
    DEST.mkdir(parents=True, exist_ok=True)
    with (DEST / 'comparison.csv').open('w') as handle:
        writer = csv.DictWriter(handle, fieldnames=list(cases[0]))
        writer.writeheader(); writer.writerows(cases)
    dump(DEST / 'summary.json', result)
    dump(DEST / 'mechanism_summary.json', {'counts': dict(counts), 'times_ms_sum': dict(times)})
    outputs = DEST / 'outputs'
    shutil.copytree(ROOT / 'results/out' / BIN, outputs)
    dump(DEST / 'output_manifest.json', {p.name: sha(p) for p in sorted(outputs.iterdir()) if p.is_file()})
    unchanged()
    print(json.dumps(result, ensure_ascii=False, indent=2))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('stage', choices=('build', 'diagnostic', 'evaluation'))
    args = parser.parse_args()
    if os.uname().sysname == 'Darwin':
        os.environ.setdefault('SDKROOT', run(['xcrun', '--sdk', 'macosx', '--show-sdk-path']).stdout.strip())
        os.environ.setdefault('MACOSX_DEPLOYMENT_TARGET', '15.0')
    {'build': build, 'diagnostic': diagnostic, 'evaluation': evaluation}[args.stage]()


if __name__ == '__main__':
    main()
