#!/usr/bin/env python3
"""Build, compare, and evaluate the preregistered v058 cleanup once."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import argparse
import csv
import difflib
import json
import os
import re
import shutil
import subprocess
import sys

from audit_v057 import FLAGS, MODES
from audit_v068 import KINDS, SCHEDULES, compute_lock, dump, sha, run
from check_v037_results import ERRORS, log_values, verify_output
from refactor_v072 import prepare, rename

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v072_audit'
DEST = ROOT / 'results/analysis/v072'
BIN = 'v072_joint_refactored'
PARENT = 'v058_joint_priority'
PARENT_RUN = '20260930T003629+0900_v058_joint_priority_7a1ed3'
LOG_KEYS = {'state_payload_bytes': 'board_payload_bytes', 'state_handle_bytes': 'board_handle_bytes',
            'state_slots': 'board_pool_slots', 'state_pool_bytes': 'board_pool_bytes',
            'state_pool_free_at_end': 'board_pool_free_at_end'}


def normalize_pp(source):
    source = re.sub(r'"[^"\n]*(?:v058_joint_priority|v072_joint_refactored|expected_structure)\.cpp"', '"solver.cpp"', source)
    source = re.sub(r'("solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', source)
    return source


def build():
    assert not (OUT / 'frozen.json').exists()
    expected, edits, mapping = prepare()
    assert (ROOT / f'src/bin/{BIN}.cpp').read_text() == expected
    # This projection retains old identifiers. Preprocess independently, then
    # undo only the child's identifier mapping to check the registered changes.
    structure = (ROOT / f'src/bin/{PARENT}.cpp').read_text()
    for edit in edits:
        assert structure.count(edit['old']) == edit['count'], edit['reason']
        structure = structure.replace(edit['old'], edit['new'])
    projection = OUT / 'expected_structure.cpp'
    projection.write_text(structure)
    inverse = {v: k for k, v in mapping.items()}

    def mode_build(mode):
        flags = [*FLAGS, *MODES[mode]]
        commands, binaries, pp = [], [], {}
        def compile(args):
            commands.append(args)
            result = run(args)
            assert not result.stderr, result.stderr
            return result.stdout
        for name, source in ((PARENT, ROOT / f'src/bin/{PARENT}.cpp'),
                             (BIN, ROOT / f'src/bin/{BIN}.cpp'), ('expected_structure', projection)):
            pp[name] = compile(['g++-15', *flags, '-E', '-P', str(source)])
            (OUT / f'{name}_{mode}.ii').write_text(pp[name])
            if name == 'expected_structure':
                continue
            source_asm = OUT / f'{name}_{mode}.s'
            compile(['g++-15', *flags, '-S', str(source), '-o', str(source_asm)])
            fixed = OUT / f'fixed_{name}_{mode}'
            board, pool = ('geo', 'board_pool') if name == PARENT else ('board_info', 'state_pool')
            compile(['g++-15', *flags, f'-DAUDIT_SOURCE="{source}"', f'-DAUDIT_BOARD={board}',
                     f'-DAUDIT_POOL={pool}', str(ROOT / 'adhoc/bin/check_v071_fixed_clock.cpp'), '-o', str(fixed)])
            binaries.append(fixed)
        header, body = pp[BIN].split('constexpr double JUDGE_TIME_LIMIT_SEC', 1)
        comparison = normalize_pp(header + 'constexpr double JUDGE_TIME_LIMIT_SEC' + rename(body, inverse))
        # assert stringifies its expression; renamed identifiers also occur in
        # that diagnostic literal, while the actual condition was checked above.
        comparison = re.sub(r'(__assert_rtn\(__func__, "solver\.cpp", 0, ")([^"]*)(")',
                            lambda m: m[1] + rename(m[2], inverse) + m[3], comparison)
        assert comparison == normalize_pp(pp['expected_structure']), mode
        (OUT / f'{mode}_preprocessed.diff').write_text(''.join(difflib.unified_diff(
            normalize_pp(pp[PARENT]).splitlines(True), comparison.splitlines(True))))
        binary = OUT / f'{BIN}_{mode}'
        compile(['g++-15', *flags, str(ROOT / f'src/bin/{BIN}.cpp'), '-o', str(binary)])
        binaries.append(binary)
        print(mode, 'built; preprocessed changes matched registration', flush=True)
        return mode, {'commands': commands, 'binaries': [str(p.relative_to(ROOT)) for p in binaries]}

    with compute_lock():
        with ThreadPoolExecutor(max_workers=2) as pool:
            modes = dict(pool.map(mode_build, MODES))
    parent_rows = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').open()
                   if PARENT_RUN in line]
    assert len(parent_rows) == 100 and all(r['run_id'] == PARENT_RUN and r['status'] == 'ok' for r in parent_rows)
    dump(OUT / 'parent_records.json', parent_rows)
    shutil.copy2(ROOT / 'notes/experiments/v072.md', OUT / 'preregistration.md')
    protected = [ROOT / f'src/bin/{PARENT}.cpp', ROOT / f'src/bin/{BIN}.cpp', ROOT / 'src/bin/v000_template.cpp',
                 ROOT / 'src/bin/v059_repair_priority.cpp', ROOT / 'src/bin/v071_refactored.cpp',
                 ROOT / 'notes/experiments/v058.md', ROOT / 'notes/experiments/v059.md', ROOT / 'notes/experiments/v071.md',
                 ROOT / 'notes/notations.md', ROOT / 'notes/important_properties.md', ROOT / 'README.md',
                 ROOT / 'adhoc/bin/check_v071_fixed_clock.cpp', Path(__file__),
                 ROOT / 'adhoc/scripts/refactor_v071.py', ROOT / 'adhoc/scripts/refactor_v072.py',
                 ROOT / 'adhoc/scripts/audit_v057.py', ROOT / 'adhoc/scripts/audit_v068.py', ROOT / 'adhoc/scripts/check_v037_results.py',
                 ROOT / 'scripts/eval.py', ROOT / 'scripts/build_solver.sh',
                 OUT / 'registered_changes.json', OUT / 'parent_records.json', OUT / 'preregistration.md',
                 *sorted((ROOT / 'tools/in').glob('*.txt'))]
    protected += [ROOT / p for m in modes.values() for p in m['binaries']]
    assert len(list((ROOT / 'tools/in').glob('*.txt'))) == 100
    dump(OUT / 'frozen.json', {'sha256': {str(p.relative_to(ROOT)): sha(p) for p in protected},
                             'modes': modes, 'compiler': run(['g++-15', '--version']).stdout.splitlines()[0]})


def unchanged():
    for name, expected in json.loads((OUT / 'frozen.json').read_text())['sha256'].items():
        assert sha(ROOT / name) == expected, name


def normalize_log(text):
    for child, parent in LOG_KEYS.items():
        text = text.replace('[summary.count] '+child+'=', '[summary.count] '+parent+'=')
    text = re.sub(r'^\[refactor.cpu\].*\n', '', text, flags=re.M)
    # TraceStats sorts keys when printing; the renamed state keys move in that order.
    counts = sorted(re.findall(r'^\[summary.count\].*\n', text, re.M))
    return re.sub(r'^\[summary.count\].*\n', '', text, flags=re.M) + ''.join(counts)


def diagnostic():
    unchanged()
    assert not (OUT / 'diagnostic.json').exists()
    def compare(args):
        mode, i, path = args
        rows = {}
        order = (PARENT, BIN) if i % 2 == 0 else (BIN, PARENT)
        for name in order:
            result = run([str(OUT / f'fixed_{name}_{mode}')], input=path.read_text(), timeout=60)
            dest = OUT / 'fixed' / mode / name / path.name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text(result.stdout)
            dest.with_suffix('.txt.err').write_text(result.stderr)
            verify_output(str(path), dest)
            rows[name] = result
        assert rows[PARENT].stdout == rows[BIN].stdout, (mode, path.name, 'moves')
        assert normalize_log(rows[PARENT].stderr) == normalize_log(rows[BIN].stderr), (mode, path.name, 'log')
        field = re.search(r'\[refactor.audit\] clock_calls=(\d+) rng=(\d+) M=(\d+) pool_free=(\d+)', rows[BIN].stderr)
        assert field and int(field[4]) == 4
        times = {}
        for name, row in rows.items():
            ticks, per_second = map(int, re.search(r'\[refactor.cpu\] ticks=(\d+) per_second=(\d+)', row.stderr).groups())
            times[name] = ticks / per_second
        return {'mode': mode, 'case': path.name, 'T': len(rows[BIN].stdout.splitlines()),
                'clock_calls': int(field[1]), 'rng': int(field[2]), 'cpu_sec': times}
    args = [(mode, i, p) for mode in MODES for i, p in enumerate(sorted((ROOT / 'tools/in').glob('*.txt')))]
    results = []
    with compute_lock():
        with ThreadPoolExecutor(max_workers=2) as pool:
            for row in pool.map(compare, args):
                results.append(row)
                if len(results) % 50 == 0:
                    print('Fixed clock:', len(results), '/ 200 pairs', flush=True)
    times = {}
    for mode in MODES:
        sums = {name: sum(r['cpu_sec'][name] for r in results if r['mode'] == mode) for name in (PARENT, BIN)}
        times[mode] = {**sums, 'child_parent_ratio': sums[BIN] / sums[PARENT]}
    unchanged()
    dump(OUT / 'diagnostic.json', {'passed': True, 'pairs': len(results), 'times': times, 'cases': results})
    print(json.dumps(times, indent=2))


def evaluate():
    unchanged()
    assert json.loads((OUT / 'diagnostic.json').read_text())['passed']
    assert not any(json.loads(line)['bin'] == BIN for line in (ROOT / 'results/eval_records.jsonl').open())
    command = [sys.executable, str(ROOT / 'scripts/eval.py'), BIN, 'tools/in', '-j', '2', '--label', BIN]
    env = os.environ.copy(); env['CARGO_BUILD_JOBS'] = '2'
    with (OUT / 'eval.log').open('w') as log:
        result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, env=env)
    assert result.returncode == 0, (OUT / 'eval.log').read_text()
    unchanged()


def summarize():
    unchanged()
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').open()]
    rows = sorted((r for r in records if r['bin'] == BIN), key=lambda r: r['case_name'])
    assert len(rows) == 100 and len({r['run_id'] for r in rows}) == 1
    parents = {r['case_name']: r for r in json.loads((OUT / 'parent_records.json').read_text())}
    comparisons, error_totals = [], {k: 0 for k in ERRORS}
    calls = {k: 0 for k in ('joint', 'strict', 'slack', 'finite')}
    involvement = {k: 0 for k in KINDS}
    for row in rows:
        assert row['status'] == 'ok' and row['local'] and row['input_dir'] == 'tools/in'
        path = ROOT / row['stdout_path']
        T = verify_output(row['case_name'], path)
        counts, times, errors = log_values(path.with_suffix('.txt.err'))
        assert not errors and counts['E'] == 0 and T == counts['T'] == row['score'] <= 100000
        assert counts['state_pool_free_at_end'] == counts['state_slots'] == 4
        assert not any(key.startswith('lns_repair_priority_') for key in counts)
        schedule = SCHEDULES[58]
        rounds, remaining = divmod(counts['search_reductions_heavy_calls'], len(schedule))
        for kind in calls:
            expected = rounds * schedule.count(kind) + schedule[:remaining].count(kind)
            assert counts['search_reductions_'+kind+'_calls'] == expected, (row['case_name'], kind)
            calls[kind] += expected
        for kind in KINDS:
            key = 'search_reductions_'+kind
            contributed = counts[key+'_best_created_with']
            assert 0 <= contributed <= counts['search_reductions_best_created']
            assert contributed <= counts[key+'_accepted_saved'] <= counts[key+'_saved']
            involvement[kind] += contributed
        assert sum(counts['search_reductions_'+k+'_best_created_with'] for k in KINDS) >= counts['search_reductions_best_created']
        for key in ERRORS:
            error_totals[key] += counts[key]
        comparisons.append({'case': row['case_name'], 'T': T, 'v058_T': parents[row['case_name']]['score'],
                            'delta_T': T - parents[row['case_name']]['score'], 'elapsed_ms': row['elapsed'],
                            'lns_attempts': counts['lns_attempts'], 'lns_insertions': counts['lns_insertions']})
    DEST.mkdir(exist_ok=True)
    with (DEST / 'comparison.csv').open('w') as handle:
        writer = csv.DictWriter(handle, fieldnames=list(comparisons[0])); writer.writeheader(); writer.writerows(comparisons)
    before = len((ROOT / f'src/bin/{PARENT}.cpp').read_text().splitlines())
    after = len((ROOT / f'src/bin/{BIN}.cpp').read_text().splitlines())
    maximum = max(r['elapsed'] for r in rows)
    assert before > after and not any(error_totals.values()) and all(calls.values())
    summary = {'run_id': rows[0]['run_id'], 'cases': 100, 'T': sum(r['T'] for r in comparisons),
               'v058_T': sum(r['v058_T'] for r in comparisons), 'delta_T': sum(r['delta_T'] for r in comparisons),
               'win_tie_loss': [sum(r['delta_T'] < 0 for r in comparisons), sum(r['delta_T'] == 0 for r in comparisons), sum(r['delta_T'] > 0 for r in comparisons)],
               'max_elapsed_ms': maximum, 'avg_elapsed_ms': sum(r['elapsed'] for r in rows) / 100,
               'all_legal_E0': True, 'errors': error_totals, 'lines_before': before, 'lines_after': after,
               'schedule_verified_cases': len(rows), 'heavy_calls': calls, 'best_created_with': involvement,
               'refactor_accepted': maximum <= 2000, 'fixed_clock': json.loads((OUT / 'diagnostic.json').read_text())}
    dump(DEST / 'summary.json', summary)
    shutil.copytree(ROOT / 'results/out' / BIN, DEST / 'outputs')
    dump(DEST / 'output_manifest.json', {p.name: sha(p) for p in sorted((DEST / 'outputs').iterdir()) if p.is_file()})
    print(json.dumps({k: v for k, v in summary.items() if k != 'fixed_clock'}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('stage', choices=('build', 'diagnostic', 'eval', 'summary'))
    args = parser.parse_args()
    if os.uname().sysname == 'Darwin':
        os.environ.setdefault('SDKROOT', run(['xcrun', '--sdk', 'macosx', '--show-sdk-path']).stdout.strip())
        os.environ.setdefault('MACOSX_DEPLOYMENT_TARGET', '15.0')
    {'build': build, 'diagnostic': diagnostic, 'eval': evaluate, 'summary': summarize}[args.stage]()
