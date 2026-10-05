"""B-81の機構確認、既存短縮との重複確認、条件成立時の通常100件評価。"""
from __future__ import annotations

import csv
import difflib
import json
from pathlib import Path
import re

from experiment_batch_support import FLAGS, digest, load_module, write_csv, write_json

CHILD = 'v062_deferred_passenger'
WITNESSES = 'results/analysis/v800/20260929_all_100/complete/deferred_passengers.csv'


def static_audit(ctx):
    audit = ctx.audit('v062')
    parent = ctx.source('src/bin/v057_search_reductions.cpp')
    child = ctx.source(f'src/bin/{CHILD}.cpp')
    registered = json.loads(ctx.source('adhoc/v062_audit/registered_changes.json').read_text())
    if digest(parent) != registered['parent_sha256']:
        raise RuntimeError('v062の親ソースが登録と異なる')
    restored = child.read_text()
    for change in reversed(registered['changes']):
        if restored.count(change['new']) != 1:
            raise RuntimeError('v062の登録差分を一意に戻せない')
        restored = restored.replace(change['new'], change['old'], 1)
    if restored != parent.read_text():
        raise RuntimeError('v062に未登録の差分がある')
    rebuilt = ctx.binary('v062_parent_reconstructed.cpp')
    rebuilt.write_text(restored)
    (audit / 'source.diff').write_text(''.join(difflib.unified_diff(parent.read_text().splitlines(True), child.read_text().splitlines(True))))
    def normalize(text):
        text = re.sub(r'"[^"\n]*(?:v057_search_reductions|v062_deferred_passenger|v062_parent_reconstructed)\.cpp"', '"solver.cpp"', text)
        return re.sub(r'("solver\.cpp",\s*)\d+(,)', r'\g<1>0\2', text)
    for local, mode in ((True, 'local'), (False, 'production')):
        ctx.compile(f'src/bin/{CHILD}.cpp', CHILD if local else CHILD + '_production', local=local)
        flags = ['-DLOCAL'] if local else ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX']
        expanded = {}
        for name, source in (('parent', parent), ('child', child), ('restored', rebuilt)):
            output = audit / f'{mode}_{name}.ii'
            ctx.command([ctx.compiler, *FLAGS, *flags, '-E', '-P', source], output=output)
            expanded[name] = normalize(output.read_text())
        if expanded['parent'] != expanded['restored']:
            raise RuntimeError(f'v062 {mode}: 復元した完全前処理結果が親と異なる')
        (audit / f'{mode}_preprocessed.diff').write_text(''.join(difflib.unified_diff(
            expanded['parent'].splitlines(True), expanded['child'].splitlines(True))))
    write_json(audit / 'static_verification.json', dict(parent_sha256=digest(parent), child_sha256=digest(child),
               restored_parent_equal=True, fully_preprocessed_modes=['local', 'production']))


def run(ctx):
    audit, analysis = ctx.audit('v062'), ctx.analysis('v062')
    if not (audit / 'static_verification.json').exists():
        raise RuntimeError('v062の事前ビルド・差分照合が未完了')
    inspector = load_module(ctx.source('adhoc/scripts/analyze_v047_coordination.py'), 'v062_independent_replay')
    inspect = inspector.inspect
    worker = load_module(ctx.source('adhoc/scripts/v060_worker.py'), 'v062_reference_reader')
    binary = ctx.compile('adhoc/bin/check_v062_deferred_passenger.cpp', 'check_v062_deferred_passenger')
    witnesses = list(csv.DictReader(ctx.saved(WITNESSES).open()))
    if len(witnesses) != 6:
        raise RuntimeError('B-81の固定6例がそろっていない')
    witness_rows, plans = [], []
    for row in witnesses:
        case = row['case']
        identifier = case + '_' + Path(row['plan']).stem
        folder = audit / 'witnesses' / identifier
        folder.mkdir(parents=True)
        inp, before = ctx.saved(f'tools/in/{case}.txt'), ctx.parent(row['before_plan'])
        source, reference = inspect(inp, before), inspect(inp, ctx.parent(row['plan']))
        start, expected_end = int(row['first_turn']), int(row['before_end'])
        if (len(source['operations'])-len(reference['operations']) != 1 or
                source['states'][start] != reference['states'][int(row['after_start'])] or
                source['states'][expected_end] != reference['states'][int(row['after_end'])]):
            raise RuntimeError('B-81の出典・全盤面境界が一致しない')
        ctx.command([binary, before, start, folder], input_path=inp, output=folder / 'probe.log', timeout=120)
        data = json.loads((folder / 'summary.json').read_text())
        replayed = inspect(inp, folder / 'result.txt')
        end = data['end']
        if (not data['found'] or data['additional_saved'] != 1 or end <= start or
                replayed['states'][end-1] != source['states'][end] or
                len(replayed['operations']) != data['after_T'] or
                (case == '0081' and end-start <= 64)):
            raise RuntimeError(f'B-81の機構確認に失敗: {identifier}')
        witness_rows.append(dict(id=identifier, case=case, **data, independently_verified=True))
        plans.append((identifier, case, before))
    write_json(audit / 'witness_verification.json', witness_rows)
    baseline = json.loads((ctx.data / 'baseline_records.json').read_text())
    plans += [(Path(r['case_name']).stem + '_v057', Path(r['case_name']).stem, ctx.saved(r['stdout_path'])) for r in baseline]
    rows = []
    for number, (identifier, case, before) in enumerate(plans):
        folder = audit / 'after_parent' / identifier
        folder.mkdir(parents=True)
        print(f'v062 既存短縮後の診断 {number+1}/{len(plans)}: {identifier}', flush=True)
        inp = ctx.saved(f'tools/in/{case}.txt')
        ctx.command([binary, before, -1, folder], input_path=inp, output=folder / 'probe.log', timeout=180)
        data = json.loads((folder / 'summary.json').read_text())
        parent = worker.Problem.read(inp).replay((folder / 'parent.txt').read_text())
        checked = worker.Problem.read(inp).replay((folder / 'result.txt').read_text())
        if (data['before_T']-data['parent_saved'] != parent['T'] or data['after_T'] != checked['T'] or
                parent['T']-checked['T'] != data['additional_saved'] or data['additional_saved'] < 0 or data['rng_consumption']):
            raise RuntimeError('v062の固定出力の独立照合に失敗')
        rows.append(dict(id=identifier, case=case, input_sha256=digest(inp), source_sha256=digest(before), **data))
    write_csv(analysis / 'after_parent.csv', rows)
    saved = sum(r['additional_saved'] for r in rows)
    write_json(audit / 'diagnostic_verification.json', dict(witnesses=6, plans=len(plans), additional_saved=saved,
                                                         evaluation_gate=saved > 0))
    if saved == 0:
        result = dict(status='evaluation_skipped', adopted=False, additional_saved=0,
                      reason='既存の短縮後に追加削減がないため、登録条件どおり通常評価を省略')
        write_json(analysis / 'summary.json', result)
        return result
    evaluator = load_module(ctx.source('adhoc/scripts/audit_v059.py'), 'v062_shared_evaluation')
    return evaluator.evaluate(ctx, ctx.binary(CHILD), worker.Problem, version='v062', child=CHILD,
                              label_prefix='deferred_passenger', require_long_gain=False,
                              mechanism_key='deferred_passenger_accepted_saved')
