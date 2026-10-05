"""v060: 凍結v800と同じ8ケース・2探索で、毎回自己最良解から再出発する。"""
from __future__ import annotations

import csv
import json
from pathlib import Path
import shutil

from experiment_batch_support import (ROOT, RESTART_CASES, digest, load_module, now, write_csv, write_json)

CONFIG = dict(workers=2, minutes_per_case=5, restart_seconds=60, progress_seconds=10, seed=80020260929)


def parent_worker(ctx, case, mode):
    paths = list(ctx.parent(f'cases/{case}/attempts').glob(f'*/cases/{case}/{mode}/status.json'))
    complete = [p.parent for p in paths if json.loads(p.read_text())['status'] == 'completed']
    if len(complete) != 1:
        raise RuntimeError(f'{case}/{mode}: 比較元の完了探索を一意に特定できない')
    return complete[0]


def verify_sources(ctx):
    parent = ctx.parent('snapshot/v800.cpp')
    source = ctx.source('adhoc/bin/v060_long_search.cpp')
    if digest(parent) != '974610eec5377d946857c10b7b1898dfc332f66dc9860e5ab4abe7109c57e99d':
        raise RuntimeError('v800の凍結ソースが事前登録と異なる')
    if source.read_text().replace('// v060_long_search.cpp\n', '// v800.cpp\n', 1) != parent.read_text():
        raise RuntimeError('v060の探索処理が凍結v800と異なる')
    runtime = ctx.source('adhoc/scripts/v060_worker.py').read_text()
    for c in reversed(json.loads(ctx.source('adhoc/v060_audit/registered_changes.json').read_text())):
        if runtime.count(c['new']) != 1:
            raise RuntimeError('v060実行管理の登録差分が一致しない')
        runtime = runtime.replace(c['new'], c['old'], 1)
    if runtime != ctx.source('adhoc/v060_audit/runtime_reference.py').read_text():
        raise RuntimeError('v060実行管理に登録外の差分がある')
    parent_config = json.loads(ctx.parent('manifest.json').read_text())['config']
    for key, value in CONFIG.items():
        if parent_config[key] != value:
            raise RuntimeError(f'v800の設定が事前登録と異なる: {key}')
    write_json(ctx.audit('v060') / 'static_verification.json',
               dict(search_matches_frozen_v800=True, parent_sha256=digest(parent), source_sha256=digest(source), config=CONFIG))


def prepare(ctx, worker):
    run = ROOT / 'results/long_search/v060' / ctx.id
    run.mkdir(parents=True, exist_ok=False)
    manifest = dict(schema_version=1, experiment='v060', created_at=now(), config=CONFIG,
                    parent_run='20260929T150407_c945d3d4', batch=str(ctx.run), cases=[], files={})
    for case in RESTART_CASES:
        folder = run / 'cases' / case
        (folder / 'seeds').mkdir(parents=True)
        original = ctx.parent(f'cases/{case}')
        shutil.copy2(original / 'input.txt', folder / 'input.txt')
        shutil.copy2(original / 'seeds/00.txt', folder / 'seeds/00.txt')
        problem = worker.Problem.read(folder / 'input.txt')
        metrics = problem.replay((folder / 'seeds/00.txt').read_text())
        entry = dict(case=case, input=str((folder / 'input.txt').relative_to(run)), reference_T=metrics['T'],
                     seeds=[dict(path=str((folder / 'seeds/00.txt').relative_to(run)), T=metrics['T'])])
        manifest['cases'].append(entry)
        for f in (folder / 'input.txt', folder / 'seeds/00.txt'):
            manifest['files'][str(f.relative_to(run))] = digest(f)
    write_json(run / 'manifest.json', manifest)
    write_json(run / 'status.json', dict(status='prepared', updated_at=now()))
    return run, manifest


def summarize(ctx, run, worker):
    rows, rounds = [], []
    for case in RESTART_CASES:
        problem = worker.Problem.read(run / f'cases/{case}/input.txt')
        for mode in worker.MODES:
            folder = run / 'cases' / case / mode
            status = json.loads((folder / 'status.json').read_text())
            baseline = parent_worker(ctx, case, mode)
            before = problem.replay((baseline / 'best.txt').read_text())['T']
            after = problem.replay((folder / 'best.txt').read_text())['T']
            seedT = problem.replay((run / f'cases/{case}/seeds/00.txt').read_text())['T']
            if status['status'] != 'completed' or after != status['T'] or after > seedT:
                raise RuntimeError('v060保存解と完了状態が一致しない')
            parent_status = json.loads((baseline / 'status.json').read_text())
            if before != parent_status['T']:
                raise RuntimeError('比較元v800の保存解と完了状態が一致しない')
            rows.append(dict(case=case, mode=mode, parent_T=before, T=after, saved=before-after, initial_T=seedT))
            metas = sorted(folder.glob('round_*/round.json'))
            requested_total = 0.0
            for metadata in metas:
                meta = json.loads(metadata.read_text())
                initial = metadata.parent / 'input_plan.txt'
                if (meta['status'] != 'completed' or digest(initial) != meta['start_best_sha256'] or
                        meta['initial_sha256'] != meta['start_best_sha256'] or
                        problem.replay(initial.read_text())['T'] != meta['start_best_T']):
                    raise RuntimeError('v060の自己最良解の引継ぎを確認できない')
                duration = meta['requested_seconds']
                if not 0 < duration <= (60 if mode == 'multistart' else 300):
                    raise RuntimeError('v060のラウンド予算が登録範囲外')
                requested_total += duration
                if meta['ended_offset_sec'] > 310:
                    raise RuntimeError('v060のケース期限を10秒以上超過')
                events, _ = worker.read_events(metadata.parent / 'search/events.jsonl')
                rounds.append(dict(case=case, mode=mode, round=meta['round'], initial_T=meta['initial_T'],
                                   start_best_T=meta['start_best_T'], end_best_T=meta['end_best_T'], new_saved=meta['new_saved'],
                                   seed=meta['seed'], initial_sha256=meta['initial_sha256'],
                                   requested_sec=duration, elapsed_sec=meta['ended_offset_sec']-meta['started_offset_sec'],
                                   cpu_sec=events[-1]['cpu_sec']))
            if requested_total > 300.001:
                raise RuntimeError('v060の合計探索予算が300秒を超えた')
    status = json.loads((run / 'status.json').read_text())
    if status['status'] != 'completed' or status['max_active_search_processes'] != 2:
        raise RuntimeError('v060の探索数または完了状態が不正')
    analysis = ctx.analysis('v060')
    write_csv(analysis / 'comparison.csv', rows)
    write_csv(analysis / 'rounds.csv', rounds)
    shutil.copy2(run / 'learning_curve.csv', analysis / 'learning_curve.csv')
    saved = sum(r['saved'] for r in rows if r['mode'] == 'multistart')
    paired = sum(min(r['parent_T'] for r in rows if r['case'] == case) -
                 min(r['T'] for r in rows if r['case'] == case) for case in RESTART_CASES)
    result = dict(status='evaluated', adopted=saved > 0, multistart_saved=saved, paired_saved=paired,
                  max_active_search_processes=status['max_active_search_processes'], run=str(run))
    write_json(analysis / 'summary.json', result)
    return result


def run(ctx):
    verify_sources(ctx)
    worker = load_module(ctx.source('adhoc/scripts/v060_worker.py'), 'v060_frozen_worker')
    worker.ROOT = ROOT
    if not (ctx.audit('v060') / 'control_verification.json').is_file():
        raise RuntimeError('一括入口で実行制御の検証を先に完了すること')
    binary = ctx.compile('adhoc/bin/v060_long_search.cpp', 'v060_long_search')
    folder, manifest = prepare(ctx, worker)
    shutil.copy2(binary, folder / 'v060_long_search')
    manifest['binary_sha256'] = digest(binary)
    write_json(folder / 'manifest.json', manifest)
    code = worker.run_search(folder, manifest, folder / 'v060_long_search')
    if code == 130:
        raise KeyboardInterrupt('v060を中断した')
    if code:
        raise RuntimeError(f'v060の探索が完了しなかった: {folder / "status.json"}')
    return summarize(ctx, folder, worker)
