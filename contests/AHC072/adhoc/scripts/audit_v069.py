#!/usr/bin/env python3
"""Build and verify the preregistered indexed repair experiment; never tune it."""
from concurrent.futures import ThreadPoolExecutor
from collections import Counter
from pathlib import Path
import argparse
import csv
import difflib
import json
import os
import re
import shutil

from audit_v057 import FLAGS, MODES, assembly_calls
from audit_v068 import compute_lock, dump, sha, run, input_M, KINDS, SCHEDULES
from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v069_audit'
DEST = ROOT / 'results/analysis/v069'
BIN = 'v069_indexed_repair'
PARENT = 'v068_size_selector'
SAVED = ROOT / 'results/analysis/v068/outputs'
RUNS = {68: '20260930T100342+0900_v068_size_selector_79c880',
        59: '20260929T235217+0900_v059_repair_priority_6879ca'}


def normalize(text):
    text = text.replace(PARENT, 'solver').replace(BIN, 'solver')
    text = text.replace('"<stdin>"', '"solver.cpp"')
    return re.sub(r'("[^"\n]*solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', text)


def build():
    assert not (OUT / 'frozen.json').exists()
    source = ROOT / f'src/bin/{BIN}.cpp'
    parent = ROOT / f'src/bin/{PARENT}.cpp'
    stripped = source.read_text()
    for change in reversed(json.loads((OUT / 'registered_changes.json').read_text())):
        assert stripped.count(change['after']) == 1
        stripped = stripped.replace(change['after'], change['before'], 1)
    assert stripped == parent.read_text()
    hook = '\n#ifdef AHC072_SUPPORT_INDEX_AUDIT\n    friend struct SupportIndexProbe;\n#endif'
    assert (OUT / 'parent_access.cpp').read_text() == parent.read_text().replace(
        'class TemporalLNS {', 'class TemporalLNS {' + hook, 1)

    def mode_build(mode):
        flags = [*FLAGS, *MODES[mode]]
        commands, binaries = [], []

        def compile_cmd(args):
            commands.append(args)
            result = run(args)
            assert not result.stderr, result.stderr
            return result.stdout

        binary = OUT / f'{BIN}_{mode}'
        compile_cmd(['g++-15', *flags, str(source), '-o', str(binary)])
        binaries.append(binary)
        expanded = {}
        assembly = {}
        for name in (PARENT, BIN):
            path = ROOT / f'src/bin/{name}.cpp'
            expanded[name] = normalize(compile_cmd(['g++-15', *flags, '-E', '-P', str(path)]))
            (OUT / f'{name}_{mode}.ii').write_text(expanded[name])
            asm = OUT / f'{name}_{mode}.s'
            compile_cmd(['g++-15', *flags, '-S', str(path), '-o', str(asm)])
            assembly[name] = assembly_calls(asm)
            fixed = OUT / f'fixed_{name}_{mode}'
            compile_cmd(['g++-15', *flags, f'-DAUDIT_SOURCE="{path}"',
                         str(ROOT / 'adhoc/bin/probe_v068_fixed_clock.cpp'), '-o', str(fixed)])
            binaries.append(fixed)
        diff = ''.join(difflib.unified_diff(expanded[PARENT].splitlines(True), expanded[BIN].splitlines(True)))
        (OUT / f'{mode}_preprocessed.diff').write_text(diff)
        reversed_pp = run(['g++-15', *flags, '-E', '-P', '-x', 'c++', '-'], input=stripped)
        assert not reversed_pp.stderr and normalize(reversed_pp.stdout) == expanded[PARENT]
        for variant, path in (('parent', OUT / 'parent_access.cpp'), ('child', source)):
            bench = OUT / f'bench_{variant}_{mode}'
            macros = ['-DAUDIT_INDEXED'] if variant == 'child' else []
            compile_cmd(['g++-15', *flags, '-Wno-return-type', *macros, f'-DAUDIT_SOURCE="{path}"',
                         str(ROOT / 'adhoc/bin/check_v069_support_index.cpp'), '-o', str(bench)])
            binaries.append(bench)
        print(f'{mode}: builds, full preprocessing, and assembly saved', flush=True)
        return mode, {'commands': commands, 'registered_changes_only': True,
                      'insertAt': assembly, 'binaries': [str(p.relative_to(ROOT)) for p in binaries]}

    with compute_lock():
        with ThreadPoolExecutor(max_workers=2) as pool:
            modes = dict(pool.map(mode_build, MODES))
        sanitizer = OUT / 'check_sanitized'
        command = ['g++-15', *FLAGS, '-DLOCAL', '-Wno-return-type', '-DAUDIT_INDEXED',
                   '-fsanitize=undefined', '-fsanitize-undefined-trap-on-error',
                   f'-DAUDIT_SOURCE="{source}"', str(ROOT / 'adhoc/bin/check_v069_support_index.cpp'),
                   '-o', str(sanitizer)]
        result = run(command)
        assert not result.stderr, result.stderr
    inputs = sorted((ROOT / 'tools/in').glob('*.txt'))
    plans = sorted(SAVED.glob('*.txt'))
    assert len(inputs) == len(plans) == 100 and [p.name for p in inputs] == [p.name for p in plans]
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').open()]
    parents = {str(n): sorted((r for r in records if r['run_id'] == rid), key=lambda r: r['case_name'])
               for n, rid in RUNS.items()}
    assert all(len(rows) == 100 and all(r['status'] == 'ok' and r['local'] and r['input_dir'] == 'tools/in'
                                       for r in rows) for rows in parents.values())
    dump(OUT / 'parent_records.json', parents)
    shutil.copy2(ROOT / 'notes/experiments/v069.md', OUT / 'preregistration.md')
    protected = [source, parent, ROOT / 'src/bin/v059_repair_priority.cpp', ROOT / 'src/bin/v000_template.cpp',
                 ROOT / 'README.md', ROOT / 'notes/notations.md', ROOT / 'notes/important_properties.md',
                 ROOT / 'scripts/eval.py', ROOT / 'scripts/build_solver.sh',
                 ROOT / 'adhoc/bin/probe_v068_fixed_clock.cpp', ROOT / 'adhoc/bin/check_v069_support_index.cpp',
                 Path(__file__), OUT / 'registered_changes.json', OUT / 'parent_access.cpp',
                 OUT / 'preregistration.md', OUT / 'parent_records.json', sanitizer, *inputs, *plans]
    protected += [ROOT / ('adhoc/scripts/' + p + '.py') for p in
                  ('audit_v057', 'audit_v068', 'check_v037_results', 'check_v028_two_orders')]
    protected += [ROOT / p for m in modes.values() for p in m['binaries']]
    protected += sorted(SAVED.glob('*.txt.err'))
    dump(OUT / 'frozen.json', {'sha256': {str(p.relative_to(ROOT)): sha(p) for p in protected},
                             'modes': modes, 'sanitizer_command': command,
                             'compiler': run(['g++-15', '--version']).stdout.splitlines()[0],
                             'SDKROOT': os.environ.get('SDKROOT'),
                             'MACOSX_DEPLOYMENT_TARGET': os.environ.get('MACOSX_DEPLOYMENT_TARGET')})
    print('Frozen sources, checks, saved plans, baselines, and criteria before execution.', flush=True)


def unchanged():
    for name, expected in json.loads((OUT / 'frozen.json').read_text())['sha256'].items():
        assert sha(ROOT / name) == expected, name


def diagnostic():
    unchanged()
    assert not (OUT / 'diagnostic.json').exists()
    inputs = sorted((ROOT / 'tools/in').glob('*.txt'))

    def bench_pair(args):
        mode, index, path = args
        rows = {}
        order = ('parent', 'child') if index % 2 == 0 else ('child', 'parent')
        for variant in order:
            dest = OUT / 'bench' / mode / path.stem / variant
            result = run([str(OUT / f'bench_{variant}_{mode}'), str(SAVED / path.name), str(dest), '64', '128'],
                         input=path.read_text(), timeout=60)
            assert not result.stdout and not result.stderr, result.stderr
            rows[variant] = json.loads((dest / 'summary.json').read_text())
        p, c = rows['parent'], rows['child']
        assert {k: v for k, v in p.items() if not k.endswith('_ns')} == {
            k: v for k, v in c.items() if not k.endswith('_ns')}, (mode, path.name)
        base = OUT / 'bench' / mode / path.stem
        assert (base / 'parent/candidates.txt').read_bytes() == (base / 'child/candidates.txt').read_bytes()
        assert p['M'] == input_M(path) and p['synthetic_checks'] == 1536
        assert p['build_cpu_ns'] > 0 and c['build_cpu_ns'] > 0
        return {'mode': mode, 'case': path.name, 'M': p['M'], 'order': order,
                'parent': p, 'child': c, 'exact_match': True}

    def fixed_pair(args):
        mode, index, path = args
        results = {}
        order = (PARENT, BIN) if index % 2 == 0 else (BIN, PARENT)
        for name in order:
            result = run([str(OUT / f'fixed_{name}_{mode}')], input=path.read_text(), timeout=60)
            dest = OUT / 'fixed' / mode / name / path.name
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_text(result.stdout); dest.with_suffix('.txt.err').write_text(result.stderr)
            verify_output(str(path), dest)
            results[name] = result
        parent, child = results[PARENT], results[BIN]
        assert parent.stdout == child.stdout, (mode, path.name, 'moves')
        stripped = re.sub(r'^\[summary.count\] lns_support_index_[^\n]*\n', '', child.stderr, flags=re.M)
        assert parent.stderr == stripped, (mode, path.name, 'counters/clock/RNG')
        assert 'diagnostic:' not in stripped
        fields = re.search(r'\[selector.audit\] clock_calls=(\d+) rng=(\d+) M=(\d+) pool_free=(\d+)', stripped)
        assert fields and int(fields[3]) == input_M(path) and int(fields[4]) == 4
        return {'mode': mode, 'case': path.name, 'T': len(child.stdout.splitlines()),
                'clock_calls': int(fields[1]), 'rng': int(fields[2]), 'exact_match': True}

    with compute_lock():
        path = next(p for p in inputs if input_M(p) >= 80)
        result = run([str(OUT / 'check_sanitized'), str(SAVED / path.name), str(OUT / 'sanitized'), '1', '1'],
                     input=path.read_text(), timeout=60)
        assert not result.stderr and not result.stdout
        print('Synthetic fixtures and independent replay passed with undefined-behavior traps.', flush=True)
        args = [(mode, i, p) for mode in MODES for i, p in enumerate(inputs)]
        with ThreadPoolExecutor(max_workers=2) as pool:
            benchmarks = []
            for row in pool.map(bench_pair, args):
                benchmarks.append(row)
                if len(benchmarks) % 20 == 0: print(f'Fixed work: {len(benchmarks)}/200 pairs', flush=True)
        dump(OUT / 'benchmark_cases.json', benchmarks)
        with ThreadPoolExecutor(max_workers=2) as pool:
            fixed = []
            for row in pool.map(fixed_pair, args):
                fixed.append(row)
                if len(fixed) % 20 == 0: print(f'Fixed clock: {len(fixed)}/200 pairs', flush=True)
    unchanged()
    speeds = {}
    for mode in MODES:
        speeds[mode] = {}
        for group, high in (('M_below80', False), ('M_atleast80', True)):
            rows = [r for r in benchmarks if r['mode'] == mode and (r['M'] >= 80) == high]
            totals = {variant: {key: sum(r[variant][key] for r in rows) for key in
                               ('build_cpu_ns', 'repair_cpu_ns', 'candidates', 'full_lower_checks', 'indexed_links')}
                      for variant in ('parent', 'child')}
            ratios = {key: totals['child'][key] / totals['parent'][key] for key in
                      ('build_cpu_ns', 'repair_cpu_ns') if totals['parent'][key]}
            speeds[mode][group] = {'cases': len(rows), **totals, 'child_parent_ratio': ratios}
        r = speeds[mode]['M_atleast80']['child_parent_ratio']
        speeds[mode]['speed_target_passed'] = r['build_cpu_ns'] <= .95 and r['repair_cpu_ns'] <= .5
    report = {'passed': True, 'fixed_work_pairs': len(benchmarks), 'fixed_clock_pairs': len(fixed),
              'synthetic_subsets_per_process': 1536, 'speed': speeds, 'fixed_clock': fixed}
    dump(OUT / 'diagnostic.json', report)
    dump(DEST / 'speed_summary.json', speeds)
    print(json.dumps({'correctness_passed': True, 'fixed_clock_pairs': len(fixed), 'speed': speeds}, indent=2))


def evaluation():
    unchanged()
    diagnostic_report = json.loads((OUT / 'diagnostic.json').read_text())
    assert diagnostic_report['passed']
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').open()]
    current = sorted((r for r in records if r['bin'] == BIN and r['input_dir'] == 'tools/in'),
                     key=lambda r: r['case_name'])
    assert len(current) == 100 and len({r['run_id'] for r in current}) == 1
    parents = {int(n): {r['case_name']: r for r in rows} for n, rows in
               json.loads((OUT / 'parent_records.json').read_text()).items()}
    cases, counts, times, errors = [], Counter(), Counter(), []
    groups = {key: {'parent_counts': Counter(), 'child_counts': Counter(),
                    'parent_times_ms': Counter(), 'child_times_ms': Counter()}
              for key in ('M_below80', 'M_atleast80')}
    for r in current:
        assert r['status'] == 'ok' and r['local'] and r['label'] == BIN
        path = ROOT / r['stdout_path']
        T = verify_output(r['case_name'], path)
        c, t, diagnostics = log_values(path.with_suffix('.txt.err'))
        assert T == r['score'] == c['T'] == c['final_ops'] == c['validated_moves'] and T <= 100000
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
            assert not any(v for k, v in c.items() if k.startswith(('lns_repair_priority_', 'lns_support_index_')))
            assert sum(c['search_reductions_'+k+'_best_created_with'] for k in KINDS) >= c['search_reductions_best_created']
        else:
            assert c['lns_support_index_builds'] > 0
            assert c['lns_support_index_queries'] == c['lns_repair_priority_evaluated']
            assert 0 < c['lns_support_index_links_visited'] < c['lns_support_index_full_lower_checks']
        for k in KINDS:
            assert 0 <= c['search_reductions_'+k+'_accepted_saved'] <= c['search_reductions_'+k+'_saved']
            if selected == 58:
                assert 0 <= c['search_reductions_'+k+'_best_created_with'] <= min(c['search_reductions_best_created'], c['search_reductions_'+k+'_accepted_saved'])
        row = {'case': r['case_name'], 'M': M, 'selected': selected, 'T': T, 'elapsed_ms': r['elapsed'],
               'v068_T': parents[68][r['case_name']]['score'], 'v059_T': parents[59][r['case_name']]['score']}
        minimum = min(row[k] for k in ('T', 'v068_T', 'v059_T'))
        for k in ('T', 'v068_T', 'v059_T'):
            row[k+'_relative'] = (2 * 10**9 * minimum + row[k]) // (2 * row[k])
        cases.append(row); counts.update(c); times.update(t); errors.extend(diagnostics)
        pc, pt, _ = log_values((SAVED / r['case_name']).with_suffix('.txt.err'))
        g = groups['M_atleast80' if M >= 80 else 'M_below80']
        g['parent_counts'].update(pc); g['child_counts'].update(c)
        g['parent_times_ms'].update(pt); g['child_times_ms'].update(t)

    def aggregate(rows):
        result = {'cases': len(rows)}
        for k in ('T', 'v068_T', 'v059_T'):
            result[k] = sum(r[k] for r in rows)
            result[k+'_relative'] = sum(r[k+'_relative'] for r in rows) / len(rows) / 1e9
        for k in ('v068_T', 'v059_T'):
            result['delta_'+k] = result['T'] - result[k]
            result['win_tie_loss_'+k] = [sum(r['T'] < r[k] for r in rows), sum(r['T'] == r[k] for r in rows),
                                        sum(r['T'] > r[k] for r in rows)]
        return result

    overall = aggregate(cases)
    valid = not errors and not any(counts[k] for k in ERRORS) and max(r['elapsed_ms'] for r in cases) <= 2000 and cases[0]['T'] <= 43
    speed_passed = diagnostic_report['speed']['local']['speed_target_passed']
    result = {'run_id': current[0]['run_id'], 'input_set': 'tools/in', 'parent_runs': RUNS,
              'overall': overall, 'M_below80': aggregate([r for r in cases if r['M'] < 80]),
              'M_atleast80': aggregate([r for r in cases if r['M'] >= 80]),
              'all_outputs_legal_E0': True, 'schedule_verified_cases': 100,
              'errors': {k: counts[k] for k in ERRORS}, 'diagnostics': errors,
              'case0000_T': cases[0]['T'], 'max_elapsed_ms': max(r['elapsed_ms'] for r in cases),
              'avg_elapsed_ms': sum(r['elapsed_ms'] for r in cases) / 100,
              'validity_passed': valid, 'local_speed_target_passed': speed_passed,
              'adopt': valid and speed_passed and overall['T'] < min(overall['v068_T'], overall['v059_T'])}
    with (DEST / 'comparison.csv').open('w') as handle:
        writer = csv.DictWriter(handle, fieldnames=list(cases[0])); writer.writeheader(); writer.writerows(cases)
    dump(DEST / 'summary.json', result)
    dump(DEST / 'mechanism_summary.json', {'counts': dict(counts), 'times_ms_sum': dict(times), 'groups': groups})
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
