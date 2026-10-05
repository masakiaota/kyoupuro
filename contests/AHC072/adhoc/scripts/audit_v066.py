#!/usr/bin/env python3
"""v066の両ビルド・凍結・保存解診断。通常100件評価は起動しない。"""
from pathlib import Path
import argparse, csv, difflib, fcntl, hashlib, json, os, shutil, subprocess
from replay_slime_output import replay
from analyze_v047_coordination import inspect

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'adhoc/v066_audit/20260930_submit'
NAME = 'v066_repair_deferred'
PARENTS = {
    'v057': ('20260929T145653+0900_v057_search_reductions_af817c', ROOT / 'results/out/v057_search_reductions'),
    'v059': ('20260929T235217+0900_v059_repair_priority_6879ca', ROOT / 'results/analysis/v059/20260929T235018_0cd76222/outputs'),
}
WITNESSES = ROOT / 'results/analysis/v800/20260929_all_100/complete/deferred_passengers.csv'
LONG_SEARCH = ROOT / 'results/long_search/v800/20260929T150407_c945d3d4'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def write(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')

def env():
    e = os.environ.copy()
    if os.uname().sysname == 'Darwin':
        e.setdefault('SDKROOT', subprocess.check_output(['xcrun', '--show-sdk-path'], text=True).strip())
        e.setdefault('MACOSX_DEPLOYMENT_TARGET', '15.0')
    for key in ['OMP_NUM_THREADS', 'OPENBLAS_NUM_THREADS', 'MKL_NUM_THREADS', 'VECLIB_MAXIMUM_THREADS']:
        e[key] = '1'
    return e

def run(command, stem, input_path=None, timeout=180):
    with (OUT / (stem + '.out')).open('wb') as stdout, (OUT / (stem + '.err')).open('wb') as stderr:
        with (input_path or Path(os.devnull)).open('rb') as stdin:
            subprocess.run([str(x) for x in command], cwd=ROOT, env=env(), stdin=stdin,
                           stdout=stdout, stderr=stderr, check=True, timeout=timeout)

def prepare():
    OUT.mkdir(parents=True, exist_ok=True)
    assert not (OUT / 'manifest.json').exists(), '凍結済み'
    source = ROOT / 'src/bin' / f'{NAME}.cpp'
    parent = ROOT / 'src/bin/v059_repair_priority.cpp'
    changes = json.loads((ROOT / 'adhoc/v066_audit/registered_changes.json').read_text())
    donor_changes = json.loads((ROOT / 'adhoc/v062_audit/registered_changes.json').read_text())['changes']
    assert changes[1:] == donor_changes[1:], '短縮処理・適用位置をv062から変えない'
    restored = source.read_text()
    for c in reversed(changes):
        assert restored.count(c['new']) == 1
        restored = restored.replace(c['new'], c['old'], 1)
    assert restored == parent.read_text()
    reconstructed = OUT / 'parent_reconstructed.cpp'
    reconstructed.write_text(restored)
    (OUT / 'source.diff').write_text(''.join(difflib.unified_diff(
        parent.read_text().splitlines(True), source.read_text().splitlines(True))))
    for mode, args in [('local', []), ('production', ['--no-local'])]:
        print('build', mode, flush=True)
        run(['scripts/build_solver.sh', *args, NAME], f'build_{mode}')
        shutil.copy2(ROOT / 'target/release' / NAME, OUT / mode)
    run(['scripts/build_solver.sh', 'check_v066_repair_deferred'], 'build_check')
    shutil.copy2(ROOT / 'target/release/check_v066_repair_deferred', OUT / 'check')
    compiler = os.environ.get('CXX', 'g++-15')
    for mode, defines in [('local', ['-DLOCAL']), ('production', ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX'])]:
        expanded = {}
        for label, file in [('parent', parent), ('reconstructed', reconstructed), ('child', source)]:
            output = OUT / f'{mode}_{label}.ii'
            with output.open('wb') as f:
                subprocess.run([compiler, '-std=gnu++23', '-E', '-P', *defines, str(file)],
                               cwd=ROOT, env=env(), stdout=f, check=True)
            text = output.read_text().replace(json.dumps(str(file)), '"SOURCE.cpp"').replace(json.dumps(file.name), '"SOURCE.cpp"')
            expanded[label] = text
        assert expanded['parent'] == expanded['reconstructed'], mode
        (OUT / f'{mode}_preprocessed.diff').write_text(''.join(difflib.unified_diff(
            expanded['parent'].splitlines(True), expanded['child'].splitlines(True))))
    files = [source, parent, ROOT / 'src/bin/v062_deferred_passenger.cpp',
             ROOT / 'adhoc/bin/check_v066_repair_deferred.cpp', ROOT / 'adhoc/scripts/build_v066_source.py',
             ROOT / 'adhoc/scripts/audit_v066.py', ROOT / 'notes/experiments/v066.md',
             ROOT / 'adhoc/v066_audit/registered_changes.json']
    snapshots = OUT / 'snapshot'
    snapshots.mkdir()
    hashes = {}
    for p in files:
        shutil.copy2(p, snapshots / p.name)
        hashes[str(p.relative_to(ROOT))] = sha(p)
    inputs = {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / 'tools/in').glob('*.txt'))}
    assert len(inputs) == 100
    data_hashes = dict(inputs)
    records = [json.loads(x) for x in (ROOT / 'results/eval_records.jsonl').read_text().splitlines() if x]
    for version, (run_id, folder) in PARENTS.items():
        selected = {r['case_name']: r for r in records if r['run_id'] == run_id}
        assert len(selected) == 100
        for case, r in sorted(selected.items()):
            p = folder / case
            metrics = replay(ROOT / 'tools/in' / case, p)['metrics']
            assert r['status'] == 'ok' and metrics['T'] == r['score'] and metrics['E'] == 0
            data_hashes[str(p.relative_to(ROOT))] = sha(p)
            stderr = p.with_suffix('.txt.err')
            data_hashes[str(stderr.relative_to(ROOT))] = sha(stderr)
        write(OUT / f'{version}_records.json', selected)
    witnesses = list(csv.DictReader(WITNESSES.open()))
    assert len(witnesses) == 6
    data_hashes[str(WITNESSES.relative_to(ROOT))] = sha(WITNESSES)
    for r in witnesses:
        for key in ['before_plan', 'plan']:
            p = LONG_SEARCH / r[key]
            data_hashes[str(p.relative_to(ROOT))] = sha(p)
    protected = {name: sha(ROOT / name) for name in [
        'AGENTS.md', 'README.md', 'src/bin/v000_template.cpp', 'notes/notations.md',
        'notes/important_properties.md', 'scripts/build_solver.sh', 'scripts/eval.py']}
    write(OUT / 'manifest.json', dict(source_files=hashes, protected_files=protected,
          data_hashes=data_hashes, input_hashes={Path(p).name: h for p, h in inputs.items()},
          parent_output_hashes={p.name: sha(p) for p in PARENTS['v059'][1].glob('*.txt')},
          binary_hashes={name: sha(OUT / name) for name in ['local', 'production', 'check']}))
    write(OUT / 'build_verification.json', dict(status='passed', local_and_production_builds=True,
          registered_difference_reversal=True, donor_changes_equal=True,
          preprocessed_reconstructed_parent_equal=['LOCAL', 'nonLOCAL'], solver_executions=0))
    print('v066 source, binaries and data frozen; no solver executed', flush=True)

def mechanism():
    assert not (OUT / 'mechanism_started.json').exists(), '診断を重複実行しない'
    manifest = json.loads((OUT / 'manifest.json').read_text())
    for group in ['source_files', 'protected_files', 'data_hashes']:
        for name, h in manifest[group].items():
            assert sha(ROOT / name) == h, name
    assert sha(OUT / 'check') == manifest['binary_hashes']['check']
    write(OUT / 'mechanism_started.json', dict(started=True))
    witness_rows, plans = [], []
    for row in csv.DictReader(WITNESSES.open()):
        case = row['case']
        identifier = case + '_' + Path(row['plan']).stem
        inp, before = ROOT / 'tools/in' / f'{case}.txt', LONG_SEARCH / row['before_plan']
        original, reference = inspect(inp, before), inspect(inp, LONG_SEARCH / row['plan'])
        start, expected_end = int(row['first_turn']), int(row['before_end'])
        assert len(original['operations']) - len(reference['operations']) == 1
        assert original['states'][start] == reference['states'][int(row['after_start'])]
        assert original['states'][expected_end] == reference['states'][int(row['after_end'])]
        folder = OUT / 'witnesses' / identifier
        run([OUT / 'check', before, start, folder], 'witness_' + identifier, inp)
        data = json.loads((folder / 'summary.json').read_text())
        result = inspect(inp, folder / 'result.txt')
        end = data['end']
        assert data['found'] and data['additional_saved'] == 1 and end > start
        assert result['states'][end - 1] == original['states'][end]
        assert len(result['operations']) == data['after_T']
        assert case != '0081' or end - start > 64
        witness_rows.append(dict(id=identifier, case=case, **data, independently_verified=True))
        plans.append(('witness', identifier, case, before))
    write(OUT / 'witness_verification.json', witness_rows)
    for version, (_, folder) in PARENTS.items():
        plans += [(version, p.stem + '_' + version, p.stem, p) for p in sorted(folder.glob('*.txt'))]
    rows = []
    for number, (group, identifier, case, before) in enumerate(plans):
        inp = ROOT / 'tools/in' / f'{case}.txt'
        folder = OUT / 'after_parent' / identifier
        run([OUT / 'check', before, -1, folder], 'after_' + identifier, inp)
        data = json.loads((folder / 'summary.json').read_text())
        parent = replay(inp, folder / 'parent.txt')['metrics']
        result = replay(inp, folder / 'result.txt')['metrics']
        assert parent['E'] == 0 and result['E'] == 0
        assert data['before_T'] - data['parent_saved'] == parent['T']
        assert data['after_T'] == result['T'] and parent['T'] - result['T'] == data['additional_saved']
        assert data['additional_saved'] >= 0 and data['rng_consumption'] == 0
        rows.append(dict(group=group, id=identifier, case=case, **data))
        if number % 20 == 0 or number == len(plans) - 1:
            print(f'fixed plans: {number + 1}/{len(plans)}', flush=True)
    with (OUT / 'after_parent.csv').open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    totals = {group: dict(plans=sum(r['group'] == group for r in rows),
              additional_saved=sum(r['additional_saved'] for r in rows if r['group'] == group),
              shortened=sum(r['additional_saved'] > 0 for r in rows if r['group'] == group))
              for group in ['witness', 'v057', 'v059']}
    passed = totals['witness']['additional_saved'] + totals['v057']['additional_saved'] >= 1
    write(OUT / 'mechanism.json', dict(status='passed' if passed else 'gate_not_met', witnesses=6,
          witness_saved=sum(r['additional_saved'] for r in witness_rows), plans=len(rows), groups=totals,
          independently_replayed_outputs=6 + 2 * len(rows), rng_consumption=0, solver_revisions_after_execution=0))
    print(json.dumps(totals, ensure_ascii=False), flush=True)
    if not passed:
        raise SystemExit('追加短縮がなく通常評価への進行条件を満たさない')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('stage', choices=['prepare', 'mechanism'])
    args = parser.parse_args()
    with (ROOT / 'results/.eval.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        (prepare if args.stage == 'prepare' else mechanism)()
