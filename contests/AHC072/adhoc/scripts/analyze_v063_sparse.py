"""B-80: 保存316更新の範囲を分類し、同一境界の密な時間層と省略版を比較する。"""
from __future__ import annotations

from collections import defaultdict
import csv
import json
from pathlib import Path
import time

from experiment_batch_support import digest, load_module, write_csv, write_json

INTERVALS = 'results/analysis/v800/20260929_all_100/complete/shortest_intervals.csv'
WITNESSES = 'results/analysis/v800/20260929_all_100/complete/deferred_passengers.csv'
LONG_CASES = ('0014', '0015', '0030', '0034', '0042', '0047', '0070', '0071', '0075', '0081', '0082', '0084', '0089', '0092')


def eligible_rows(path):
    return [r for r in csv.DictReader(path.open()) if r['group'] == 'long' and int(r['before_moves']) > 64]


def select_examples(rows):
    selected, missing = [], []
    for case in LONG_CASES:
        for slot in range(5):
            choices = [r for r in rows if r['case'] == case and slot*60 <= float(r['elapsed_sec']) and
                       (float(r['elapsed_sec']) < (slot+1)*60 or slot == 4 and float(r['elapsed_sec']) <= 300)]
            if choices:
                selected.append(min(choices, key=lambda r: (float(r['elapsed_sec']), r['plan'])))
            else:
                missing.append(dict(case=case, slot=slot))
    return selected, missing


def contact_history(report, start, end):
    cells = defaultdict(list)
    for op in report['operations'][start:end]:
        # Record identities and the real tower contents, not just the commands.
        for role in ('source', 'destination'):
            cells[tuple(op[role])].append((role, op['action'], tuple(op['moving']),
                                          tuple(op['support']), tuple(op['landing'])))
    return cells


def describe(ctx, inspect, row):
    inp, plan = ctx.saved(f'tools/in/{row["case"]}.txt'), ctx.parent(row['before_plan'])
    before, after = inspect(inp, plan), inspect(inp, ctx.parent(row['plan']))
    start, end, astart, aend = (int(row[k]) for k in ('before_start', 'before_end', 'after_start', 'after_end'))
    if (before['states'][start] != after['states'][astart] or before['states'][end] != after['states'][aend] or
            end-start != int(row['before_moves']) or aend-astart != int(row['after_moves']) or end-start <= aend-astart):
        raise RuntimeError('v063の参照境界と元データが一致しない')
    old, new = contact_history(before, start, end), contact_history(after, astart, aend)
    changed = {p for p in set(old) | set(new) if old.get(p) != new.get(p)}
    related = sum(tuple(op['source']) in changed or tuple(op['destination']) in changed for op in before['operations'][start:end])
    description = dict(id=row['case'] + '_' + Path(row['plan']).stem, case=row['case'], plan=str(plan), input=str(inp),
                       source_sha256=digest(plan), reference_sha256=digest(ctx.parent(row['plan'])),
                       start=start, end=end, reference_start=astart, reference_end=aend,
                       original_T=end-start, reference_T=aend-astart,
                       changed_cells=sorted(changed), related_operations=related,
                       reference_scope_fits=len(changed) <= 12 and related <= 32,
                       reference_scope_note='参照側の接触履歴による範囲分類。自動候補の可否・逐次再挿入の表現可能性とは別',
                       nonempty_exit=any(before['states'][end]), entry=list(before['states'][start]), exit=list(before['states'][end]))
    return description, before, after


def completed_trials(path):
    return {(r['candidate'], r['ids'], r['order']): r for r in csv.DictReader(path.open()) if r['status'] != 'state_cap'}


def run(ctx):
    audit, analysis = ctx.audit('v063'), ctx.analysis('v063')
    inspector = load_module(ctx.source('adhoc/scripts/analyze_v047_coordination.py'), 'v063_independent_replay')
    support = load_module(ctx.source('adhoc/scripts/analyze_v061_boundaries.py'), 'v063_boundary_helpers')
    inspect = inspector.inspect
    rows = eligible_rows(ctx.saved(INTERVALS))
    if len(rows) != 316:
        raise RuntimeError('v063の固定316更新と一致しない')
    selected, missing = select_examples(rows)
    selected_paths = {r['plan'] for r in selected}
    classified_paths = {r['plan'] for r in rows}
    witnesses = list(csv.DictReader(ctx.saved(WITNESSES).open()))
    witness_paths = {r['plan'] for r in witnesses}
    unique = {r['plan']: r for r in rows + witnesses}
    classification, contracts = [], []
    started = time.monotonic()
    for number, row in enumerate(unique.values()):
        description, before, after = describe(ctx, inspect, row)
        description.update(in_316=row['plan'] in classified_paths, B81_witness=row['plan'] in witness_paths)
        classification.append(description)
        if row['plan'] in selected_paths:
            folder = audit / 'contracts' / description['id']
            folder.mkdir(parents=True)
            reference = support.actions(after, description['reference_start'], description['reference_end'])
            support.save_plan(folder / 'reference_interval.txt', reference)
            support.forced_contract(inspect, Path(description['input']), Path(description['plan']), reference,
                                    description['start'], description['end'], folder)
            write_json(folder / 'contract.json', description)
            contracts.append(description)
        if number % 50 == 0:
            print(f'v063 保存区間の照合 {number+1}/{len(unique)}', flush=True)
    write_json(audit / 'selected_examples.json', dict(selected=selected, missing=missing))
    write_json(audit / 'contracts.json', contracts)
    write_json(analysis / 'scope_classification.json', classification)
    write_csv(analysis / 'scope_classification.csv', [dict(id=r['id'], case=r['case'], span=r['original_T'],
              cells=len(r['changed_cells']), related=r['related_operations'], reference_scope_fits=r['reference_scope_fits'],
              B81_witness=r['B81_witness'], in_316=r['in_316']) for r in classification])
    analysis_seconds = time.monotonic()-started
    binary = ctx.compile('adhoc/bin/v063_sparse_lns_probe.cpp', 'v063_sparse_lns_probe')
    fixture = audit / 'self_test_input.txt'
    fixture.write_text('12 4\nABCD........\nabcd........\n' + '............\n'*10)
    ctx.command([binary, '--self-test'], input_path=fixture, output=audit / 'self_test.json', timeout=30)
    if json.loads((audit / 'self_test.json').read_text()) != dict(self_tests=8, status='passed'):
        raise RuntimeError('v063の自己検証に失敗')
    reports, comparisons, coverage = [], [], []
    improved = set()
    for number, contract in enumerate(contracts):
        folder = analysis / contract['id']
        folder.mkdir()
        print(f'v063 {number+1}/{len(contracts)}: {contract["id"]}、区間{contract["original_T"]}手', flush=True)
        ctx.command([binary, contract['plan'], contract['start'], contract['end'], folder],
                    input_path=contract['input'], output=folder / 'probe.log', timeout=40)
        original = inspect(Path(contract['input']), Path(contract['plan']))
        changed = {tuple(p) for p in contract['changed_cells']}
        scopes = list(csv.DictReader((folder / 'scopes.csv').open()))
        for scope in scopes:
            allowed = {original['floor'][int(p)] for p in scope['allowed_cells'].split()}
            coverage.append(dict(example=contract['id'], case=contract['case'], **scope,
                                 covers_changed_contacts=changed <= allowed))
        comparable = []
        for mode in ('dense', 'sparse'):
            data = json.loads((folder / mode / 'summary.json').read_text())
            if ((data['saved'] > 0) != (folder / mode / 'best.txt').exists() or
                    data['same_length_alternative'] != (folder / mode / 'equal.txt').exists()):
                raise RuntimeError('v063の集計と出力ファイルが不一致')
            verified = []
            for name in ('best', 'equal'):
                path = folder / mode / (name + '.txt')
                if not path.exists():
                    continue
                lines = path.read_text().splitlines()
                full = folder / mode / (name + '_full.txt')
                support.save_plan(full, support.actions(original, 0, contract['start']) + lines + support.actions(original, contract['end']))
                checked = inspect(Path(contract['input']), full)
                if checked['states'][contract['start']+len(lines)] != original['states'][contract['end']]:
                    raise RuntimeError('v063の探索結果が区間外へ接続しない')
                expected = data['best_T'] if name == 'best' else contract['original_T']
                if len(lines) != expected or (name == 'equal' and lines == support.actions(original, contract['start'], contract['end'])):
                    raise RuntimeError('v063の手数・同手数解が記録と不一致')
                verified.append(dict(kind=name, T=len(lines), E=checked['metrics']['E'], sha256=digest(full)))
                if mode == 'sparse' and name == 'best' and data['skipped_layers'] > 0:
                    improved.add(contract['case'])
            write_json(folder / mode / 'independent_verification.json', verified)
            reports.append(dict(example=contract['id'], case=contract['case'], mode=mode, **data))
            comparable.append(completed_trials(folder / mode / 'trials.csv'))
        matches = set(comparable[0]) & set(comparable[1])
        mismatches = []
        for key in sorted(matches):
            a, b = (x[key] for x in comparable)
            equal = (a['status'], a['base_T'], a['final_T']) == (b['status'], b['base_T'], b['final_T'])
            comparisons.append(dict(example=contract['id'], candidate=key[0], ids=key[1], order=key[2], equal=equal,
                                    status=a['status'], dense_T=a['final_T'], sparse_T=b['final_T'],
                                    dense_ms=a['elapsed_ms'], sparse_ms=b['elapsed_ms']))
            if not equal:
                mismatches.append(key)
        write_json(folder / 'comparison.json', dict(completed_common=len(matches), mismatches=mismatches,
                                                  limitation='期限・リンク上限の未完了条件を除き、共通の候補と順序だけ照合'))
        if mismatches:
            raise RuntimeError('v063の密な時間層と省略版の結果が不一致。自動修正・再試行せず停止する')
    write_csv(analysis / 'probes.csv', reports, fields=list(reports[0]) if reports else ['example'])
    write_csv(analysis / 'control_comparison.csv', comparisons, fields=list(comparisons[0]) if comparisons else ['example'])
    write_csv(analysis / 'candidate_coverage.csv', coverage, fields=list(coverage[0]) if coverage else ['example'])
    result = dict(status='diagnosed', integration_evidence=len(improved) >= 2 and bool(comparisons),
                  improved_long_cases=sorted(improved), classified_updates=316, contracts=len(contracts),
                  missing_slots=len(missing), matched_trials=len(comparisons), analysis_seconds=analysis_seconds,
                  ordinary_evaluation=False, limitation='既知の全盤面境界を指定した診断。対象領域外の経路や共同状態は探索しない')
    write_json(analysis / 'summary.json', result)
    return result
