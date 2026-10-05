#!/usr/bin/env python3
"""v059→120秒休止→v060→120秒休止→v061を、固定条件で1回ずつ手動実行する。"""
from __future__ import annotations

import argparse
from contextlib import contextmanager, redirect_stderr, redirect_stdout
from datetime import datetime
import fcntl
import json
import os
from pathlib import Path
import shutil
import signal
import sys
import time
import traceback
import uuid

from experiment_batch_support import (ROOT, PARENT_RUN, PARENT_ID, AUDIT_CASES, Context,
                                      digest, load_module, now, write_json)

STAGES = ('v059', 'v060', 'v061')
BATCH_NAME = 'v059_v061'
EXTRA_DATA = ()
COOLDOWN_SECONDS = 120
ENTRYPOINTS = dict(v059='audit_v059.py', v060='run_v060_long_search.py', v061='analyze_v061_boundaries.py')
SOURCES = (
    'src/bin/v057_search_reductions.cpp', 'src/bin/v059_repair_priority.cpp',
    'adhoc/bin/check_v059_repair_priority.cpp', 'adhoc/bin/v060_long_search.cpp',
    'adhoc/bin/v061_bounded_lns_probe.cpp', 'scripts/eval.py',
    'adhoc/scripts/experiment_batch_support.py', 'adhoc/scripts/run_v059_v061.py',
    'adhoc/scripts/audit_v059.py', 'adhoc/scripts/run_v060_long_search.py',
    'adhoc/scripts/v060_worker.py', 'adhoc/scripts/analyze_v061_boundaries.py',
    'adhoc/scripts/check_v059_v061_control.py', 'adhoc/scripts/analyze_v047_coordination.py',
    'adhoc/scripts/replay_slime_output.py', 'adhoc/v059_audit/registered_changes.json',
    'adhoc/v060_audit/registered_changes.json', 'adhoc/v060_audit/runtime_reference.py',
    'notes/experiments/v059.md', 'notes/experiments/v060.md', 'notes/experiments/v061.md',
    'notes/backlog.md', 'notes/notations.md', 'notes/important_properties.md',
    'problem_description.txt', 'AGENTS.md',
)


@contextmanager
def evaluation_lock():
    path = ROOT / 'results/.eval.lock'
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a+') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as exc:
            raise RuntimeError('v800または別の評価が実行中。終了後に同じコマンドを実行すること。') from exc
        lock.seek(0)
        lock.truncate()
        lock.write(json.dumps(dict(tool=BATCH_NAME + '_batch', pid=os.getpid(), started_at=now())) + '\n')
        lock.flush()
        os.fsync(lock.fileno())
        try:
            yield
        finally:
            fcntl.flock(lock, fcntl.LOCK_UN)


class Tee:
    def __init__(self, first, second):
        self.first, self.second = first, second

    def write(self, text):
        self.first.write(text)
        self.second.write(text)
        self.flush()
        return len(text)

    def flush(self):
        self.first.flush()
        self.second.flush()


def preflight():
    state = json.loads((ROOT / PARENT_RUN / 'status.json').read_text())
    if state.get('status') != 'completed' or not state.get('published'):
        raise RuntimeError('指定したv800の全ケース完了・登録をまだ確認できない。完了後に実行すること。')
    latest = ROOT / 'results/experiment_batches' / BATCH_NAME / 'latest.json'
    if latest.is_file():
        previous = Path(json.loads(latest.read_text())['run']) / 'status.json'
        if previous.is_file() and json.loads(previous.read_text())['status'] == 'completed':
            raise RuntimeError('この一括実験は完了済み。同じ条件の再実行を防ぐため停止する。')
    for name in SOURCES:
        if not (ROOT / name).is_file():
            raise RuntimeError(f'必要なソース・資料がない: {name}')
    inputs = sorted((ROOT / 'tools/in').glob('*.txt'))
    if [p.name for p in inputs] != [f'{i:04d}.txt' for i in range(100)]:
        raise RuntimeError('通常評価の入力0000〜0099がそろっていない')
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').read_text().splitlines() if line.strip()]
    if any(r['bin'] == 'v059_repair_priority' for r in records):
        raise RuntimeError('v059の通常評価は既に記録されている。同じ実装の再評価を防ぐため停止する。')
    baseline = [r for r in records if r['run_id'] == PARENT_ID]
    if len(baseline) != 100 or len({r['case_name'] for r in baseline}) != 100 or any(r['status'] != 'ok' for r in baseline):
        raise RuntimeError('保存済みv057の比較100件を確認できない')
    return baseline


def snapshot(baseline):
    identifier = datetime.now().strftime('%Y%m%dT%H%M%S') + '_' + uuid.uuid4().hex[:8]
    run = ROOT / 'results/experiment_batches' / BATCH_NAME / identifier
    run.mkdir(parents=True, exist_ok=False)
    manifest = dict(schema_version=1, id=identifier, created_at=now(), stages=STAGES,
                    cooldown_seconds=COOLDOWN_SECONDS, max_search_processes=2, threads_per_process=1,
                    parent_v800=str(PARENT_RUN), baseline_run_id=PARENT_ID, source_files={}, data_files={})
    manifest['output_paths'] = {
        stage: dict(audit=str(ROOT / 'adhoc' / (stage + '_audit') / identifier),
                    analysis=str(ROOT / 'results/analysis' / stage / identifier)) for stage in STAGES
    }
    manifest['output_paths']['v060']['search'] = str(ROOT / 'results/long_search/v060' / identifier)
    write_json(run / 'status.json', dict(status='preparing', updated_at=now()))
    print(f'一括実験の保存先: {run}', flush=True)
    def copy(relative, category):
        relative = Path(relative)
        source = (ROOT / relative).resolve()
        if not source.is_relative_to(ROOT) or not source.is_file():
            raise RuntimeError(f'凍結元のファイルが不正: {relative}')
        target = run / category / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        file_hash = digest(target)
        if digest(source) != file_hash:
            raise RuntimeError(f'凍結中に元ファイルが変化した: {source}')
        manifest['source_files' if category == 'snapshot' else 'data_files'][str(relative)] = file_hash
    for relative in SOURCES:
        copy(relative, 'snapshot')
    for p in sorted((ROOT / 'tools/in').glob('*.txt')):
        copy(p.relative_to(ROOT), 'data')
    for row in baseline:
        relative = Path(row['stdout_path'])
        copy(relative, 'data')
    copy('tools/target/release/vis', 'data')
    copy('adhoc/v057_audit/static_verification.json', 'data')
    for filename in ('manifest.json', 'status.json', 'snapshot/v800.cpp', 'snapshot/run_v047_long_search.py'):
        copy(PARENT_RUN / filename, 'data')
    # All reference data for the three stages is frozen BEFORE stage v059.
    # Copy only the nine relevant cases; no mutation or summarisation of v800.
    for case in AUDIT_CASES:
        original = ROOT / PARENT_RUN / 'cases' / case
        for p in sorted(original.rglob('*')):
            if p.is_file():
                copy(p.relative_to(ROOT), 'data')
    for p in sorted((ROOT / 'results/analysis/v800/20260929_through_0047').glob('*')):
        if p.is_file():
            copy(p.relative_to(ROOT), 'data')
    copy('adhoc/v057_audit/witness_manifest.tsv', 'data')
    for line in (ROOT / 'adhoc/v057_audit/witness_manifest.tsv').read_text().splitlines():
        _, _, _, _, before, reference = line.split('\t')
        for p in (Path(before), Path(reference)):
            copy(p.relative_to(ROOT), 'data')
    for relative in EXTRA_DATA:
        if str(relative) not in manifest['data_files']:
            copy(relative, 'data')
    write_json(run / 'data/baseline_records.json', baseline)
    manifest['data_files']['baseline_records.json'] = digest(run / 'data/baseline_records.json')
    write_json(run / 'manifest.json', manifest)
    write_json(run.parent / 'latest.json', dict(run=str(run), created_at=now()))
    return run


def verify_frozen(ctx):
    for key, folder in (('source_files', ctx.snapshot), ('data_files', ctx.data)):
        for relative, expected in ctx.manifest[key].items():
            if digest(folder / relative) != expected:
                raise RuntimeError(f'凍結ファイルの変更を検出: {relative}')


def cooldown(previous, upcoming, seconds=COOLDOWN_SECONDS, *, sleep=time.sleep, monotonic=time.monotonic, emit=print):
    emit(f'{previous}完了。{upcoming}まで{seconds}秒休止する。', flush=True)
    start = monotonic()
    while True:
        remaining = seconds - (monotonic() - start)
        if remaining <= 0:
            break
        sleep(min(1, remaining))
    return monotonic() - start


def run_stages(ctx, runners, *, pause=cooldown):
    results = {}
    pauses = []
    for number, stage in enumerate(STAGES):
        if number:
            write_json(ctx.run / 'status.json', dict(status='cooldown', after=STAGES[number-1], next=stage,
                       seconds=COOLDOWN_SECONDS, updated_at=now()))
            elapsed = pause(STAGES[number-1], stage)
            pauses.append(dict(after=STAGES[number-1], before=stage, requested_seconds=120, elapsed_seconds=elapsed))
            write_json(ctx.run / 'cooldowns.json', pauses)
        write_json(ctx.run / 'status.json', dict(status='running', stage=stage, updated_at=now()))
        print(f'=== {stage} 開始 ===', flush=True)
        results[stage] = runners[stage](ctx)
        write_json(ctx.run / f'{stage}.json', results[stage])
        write_json(ctx.run / 'summary.json', dict(status='running', experiments=results))
    return results


def interrupted(signum, frame):
    raise KeyboardInterrupt(f'signal {signum}')


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='起動条件だけを確認する。ビルド・探索・評価は行わない')
    args = parser.parse_args(argv)
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, interrupted)
    run = None
    try:
        with evaluation_lock():
            baseline = preflight()
            if args.check:
                print('起動条件を確認した。探索・評価は実行していない。')
                return 0
            run = snapshot(baseline)
            with (run / 'batch.log').open('w') as log, redirect_stdout(Tee(sys.stdout, log)), redirect_stderr(Tee(sys.stderr, log)):
                ctx = Context(run)
                os.environ.update(ctx.env)
                os.environ['AHC072_BATCH_ROOT'] = str(ROOT)
                sys.path.insert(0, str(ctx.source('adhoc/scripts')))
                verify_frozen(ctx)
                modules = {stage: load_module(ctx.source('adhoc/scripts/' + ENTRYPOINTS[stage]), 'batch_' + stage) for stage in STAGES}
                reader = load_module(ctx.source('adhoc/scripts/v060_worker.py'), 'batch_reference_reader')
                parent_audit = json.loads(ctx.saved('adhoc/v057_audit/static_verification.json').read_text())
                for row in baseline:
                    input_path = ctx.saved('tools/in/' + row['case_name'])
                    if digest(input_path) != parent_audit['input_sha256'][row['case_name']]:
                        raise RuntimeError('v057当時と入力が異なる: ' + row['case_name'])
                    metrics = reader.Problem.read(input_path).replay(ctx.saved(row['stdout_path']).read_text())
                    if metrics['T'] != row['score']:
                        raise RuntimeError('保存済みv057の手順が評価記録と異なる')
                version_log = run / 'build/compiler.txt'
                ctx.command([ctx.compiler, '--version'], output=version_log)
                # Catch build/contract issues in all three implementations before
                # starting the first experiment. Builds are sequential, one core.
                modules['v059'].static_audit(ctx)
                modules['v060'].verify_sources(ctx)
                if 'v062' in modules:
                    modules['v062'].static_audit(ctx)
                for source, name in (('adhoc/bin/check_v059_repair_priority.cpp', 'check_v059_repair_priority'),
                                     ('adhoc/bin/v060_long_search.cpp', 'v060_long_search'),
                                     ('adhoc/bin/v061_bounded_lns_probe.cpp', 'v061_bounded_lns_probe')):
                    ctx.compile(source, name)
                if 'v062' in modules:
                    ctx.compile('adhoc/bin/check_v062_deferred_passenger.cpp', 'check_v062_deferred_passenger')
                if 'v063' in modules:
                    ctx.compile('adhoc/bin/v063_sparse_lns_probe.cpp', 'v063_sparse_lns_probe')
                checks = load_module(ctx.source('adhoc/scripts/check_v059_v061_control.py'), 'batch_control_checks')
                checks.verify_worker(reader, ctx.audit('v060') / 'control_verification.json')
                results = run_stages(ctx, {name: module.run for name, module in modules.items()})
                verify_frozen(ctx)
                write_json(run / 'summary.json', dict(status='completed', experiments=results))
                write_json(run / 'status.json', dict(status='completed', updated_at=now()))
                print(f'{len(STAGES)}実験が完了した。結果: {run / "summary.json"}', flush=True)
        return 0
    except BaseException as exc:
        status = 'interrupted' if isinstance(exc, KeyboardInterrupt) else 'failed'
        if run is not None:
            previous = json.loads((run / 'status.json').read_text())
            write_json(run / 'status.json', previous | dict(status=status, error=str(exc), updated_at=now()))
            with (run / 'error.log').open('w') as log:
                traceback.print_exc(file=log)
        print(f'停止: {exc}' + (f'\n保存先: {run}' if run else ''), file=sys.stderr)
        return 130 if isinstance(exc, KeyboardInterrupt) else 1


if __name__ == '__main__':
    raise SystemExit(main())
