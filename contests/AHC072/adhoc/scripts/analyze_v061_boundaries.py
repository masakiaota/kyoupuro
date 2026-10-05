"""v061: 固定した保存例の全盤面境界を照合し、区間内の逐次再挿入を診断する。"""
from __future__ import annotations

from collections import defaultdict
import csv
import json
from pathlib import Path
import resource
import time

from experiment_batch_support import ROOT, LONG_CASES, digest, load_module, write_csv, write_json

EVENTS = 'results/analysis/v800/20260929_through_0047/events.csv'
SPANS = (16, 32, 64)


def select_examples(ctx):
    rows = list(csv.DictReader(ctx.saved(EVENTS).open()))
    selected, missing = [], []
    for case in LONG_CASES:
        eligible = [r for r in rows if r['case'] == case and r['mode'] == 'continuous' and
                    r['type'] == 'improvement' and r['before_T'] == r['previous_best_T'] and
                    r['before_same_as_record'] == '0']
        for slot in range(5):
            choices = [r for r in eligible if slot*60 <= float(r['elapsed_sec']) and
                       (float(r['elapsed_sec']) < (slot+1)*60 or slot == 4 and float(r['elapsed_sec']) <= 300)]
            if not choices:
                missing.append(dict(case=case, slot=slot, reason='該当イベントなし'))
                continue
            row = min(choices, key=lambda r: (float(r['elapsed_sec']), int(r['iteration']), r['plan']))
            plan = ctx.parent(row['plan'])
            search = plan.parent.parent
            events = [json.loads(line) for line in (search / 'events.jsonl').read_text().splitlines() if line.strip()]
            matches = [e for e in events if e.get('plan') and (search / e['plan']).resolve() == plan.resolve() and e['type'] == 'improvement']
            if len(matches) != 1:
                raise RuntimeError('選択した改善イベントを一意に照合できない')
            event = matches[0]
            previous = search / event['previous_best_plan']
            before = ctx.parent(row['before_plan'])
            if before.resolve() != (search / event['before_plan']).resolve():
                raise RuntimeError('改善前の出典が一致しない')
            round_meta = json.loads((search.parent / 'round.json').read_text())
            files = dict(input=ctx.parent(f'cases/{case}/input.txt'), before=before, after=plan, previous=previous)
            if any(not path.resolve().is_relative_to(ctx.data) for path in files.values()):
                raise RuntimeError('選択したイベントが凍結データの外を指している')
            selected.append(dict(id=f'{case}_{slot}', case=case, slot=slot, row=row, event=event,
                                 round_seed=round_meta['seed'], rng_state=event.get('rng_state'),
                                 rng_state_note='イベントに内部状態がなければ復元しない。ラウンドの種のみ保存する',
                                 paths={k: str(v) for k, v in files.items()}, hashes={k: digest(v) for k, v in files.items()}))
    return selected, missing


def boundaries(before, after):
    # Tuple dictionary lookup uses a hash index AND exact tuple equality.
    index = defaultdict(list)
    for j, board in enumerate(after['states']):
        index[board].append(j)
    chosen = {}
    for bound in SPANS:
        best = None
        for start in range(len(before['states']) - 1):
            for astart in index[before['states'][start]]:
                for end in range(start + 1, min(start + bound + 1, len(before['states']))):
                    for aend in index[before['states'][end]]:
                        saved = (end-start) - (aend-astart)
                        if aend < astart or saved <= 0:
                            continue
                        key = (-saved, end-start, start, astart, end, aend)
                        if best is None or key < best[0]:
                            best = (key, (start, end, astart, aend))
        chosen[bound] = best[1] if best else None
    return chosen


def actions(report, start=0, end=None):
    return [op['action'] for op in report['operations'][start:end]]


def save_plan(path, lines):
    path.write_text(''.join(line + '\n' for line in lines))


def contact_history(report):
    cells = defaultdict(list)
    for op in report['operations']:
        for key in ('source', 'destination'):
            cells[tuple(op[key])].append((key, tuple(op['moving']), tuple(op['support']), tuple(op['landing'])))
    return cells


def forced_contract(inspect, input_path, before_path, reference_lines, start, end, folder):
    before = inspect(input_path, before_path)
    path = folder / 'forced_reference_full.txt'
    save_plan(path, actions(before, 0, start) + reference_lines + actions(before, end))
    replayed = inspect(input_path, path)
    if (before['states'][start] != replayed['states'][start] or
            before['states'][end] != replayed['states'][start+len(reference_lines)]):
        raise RuntimeError('参照区間を接続した全盤面が一致しない')
    return before, replayed


def verify_witnesses(ctx, inspect):
    audit = ctx.audit('v061') / 'known_witnesses'
    audit.mkdir()
    rows = []
    for line in ctx.saved('adhoc/v057_audit/witness_manifest.tsv').read_text().splitlines():
        name, case, start, end, before, reference = line.split('\t')
        before = ctx.saved(Path(before).relative_to(ROOT))
        reference = ctx.saved(Path(reference).relative_to(ROOT))
        folder = audit / name
        folder.mkdir()
        report, after = forced_contract(inspect, ctx.saved(f'tools/in/{case}.txt'), before,
                                       reference.read_text().splitlines(), int(start), int(end), folder)
        rows.append(dict(name=name, case=case, verified=True, original_T=int(end)-int(start),
                         reference_T=len(reference.read_text().splitlines()),
                         nonempty_exit=any(report['states'][int(end)])))
    if len(rows) != 4:
        raise RuntimeError('既存の境界検証例が4件そろっていない')
    write_json(audit / 'verification.json', rows)


def run(ctx):
    audit, analysis = ctx.audit('v061'), ctx.analysis('v061')
    inspector = load_module(ctx.source('adhoc/scripts/analyze_v047_coordination.py'), 'v061_independent_replay')
    inspect = inspector.inspect
    started = time.monotonic()
    started_cpu = time.process_time()
    examples, missing = select_examples(ctx)
    write_json(audit / 'selected_examples.json', dict(examples=examples, missing=missing))
    verify_witnesses(ctx, inspect)
    contracts, scope_rows, neutral = [], [], []
    for example in examples:
        paths = {k: Path(v) for k, v in example['paths'].items()}
        before, after, previous = [inspect(paths['input'], paths[k]) for k in ('before', 'after', 'previous')]
        if len(before['operations']) != len(previous['operations']) or actions(before) == actions(previous):
            raise RuntimeError('同手数の別手順という選択条件が実データと一致しない')
        old_contacts, new_contacts = contact_history(previous), contact_history(before)
        neutral.append(dict(example=example['id'], T=len(before['operations']),
                            changed_contact_cells=sum(old_contacts.get(p) != new_contacts.get(p) for p in set(old_contacts) | set(new_contacts)),
                            accepted_history_available=False))
        unique = {}
        for span, bounds in boundaries(before, after).items():
            scope_rows.append(dict(example=example['id'], case=example['case'], span=span, eligible=bounds is not None,
                                   bounds=list(bounds) if bounds else None))
            if bounds is not None:
                unique.setdefault(bounds, []).append(span)
        for sequence, (bounds, spans) in enumerate(unique.items()):
            start, end, astart, aend = bounds
            identifier = f'{example["id"]}_{sequence}'
            folder = audit / 'contracts' / identifier
            folder.mkdir(parents=True)
            reference = actions(after, astart, aend)
            save_plan(folder / 'reference_interval.txt', reference)
            _, replayed = forced_contract(inspect, paths['input'], paths['before'], reference, start, end, folder)
            ops = before['operations'][start:end]
            involved = {id for op in ops for key in ('moving', 'support', 'landing') for id in op[key]}
            cells = {tuple(op[k]) for op in ops for k in ('source', 'destination')}
            outside = before['operations'][:start] + before['operations'][end:]
            touched_outside = cells & {tuple(op[k]) for op in outside for k in ('source', 'destination')}
            contract = dict(id=identifier, case=example['case'], spans=spans, input=str(paths['input']), plan=str(paths['before']),
                            start=start, end=end, reference_start=astart, reference_end=aend, original_T=end-start,
                            reference_T=aend-astart, saved=end-start-(aend-astart), entry=list(before['states'][start]),
                            exit=list(before['states'][end]), nonempty_exit=any(before['states'][end]),
                            cells=len(cells), tokens=len(involved), shared_with_outside=len(touched_outside),
                            returned_tokens=[id for op in ops for k in ('returned_source', 'returned_destination') for id in op[k]],
                            reference_replay_valid=True, original_replay_valid=True,
                            sequential_model_representability='not_established_by_replay', folder=str(folder))
            write_json(folder / 'contract.json', contract)
            contracts.append(contract)
    write_json(audit / 'contracts.json', contracts)
    write_json(analysis / 'scope.json', scope_rows)
    write_json(analysis / 'same_length_changes.json', neutral)
    analysis_seconds = time.monotonic() - started
    analysis_cpu_seconds = time.process_time() - started_cpu
    binary = ctx.compile('adhoc/bin/v061_bounded_lns_probe.cpp', 'v061_bounded_lns_probe')
    fixture = audit / 'self_test_input.txt'
    fixture.write_text('12 4\nABCD........\nabcd........\n' + '............\n' * 10)
    ctx.command([binary, '--self-test'], input_path=fixture, output=audit / 'self_test.json', timeout=30)
    if json.loads((audit / 'self_test.json').read_text()) != dict(self_tests=6, status='passed'):
        raise RuntimeError('任意終点の自己検証に失敗')
    rows, improved = [], set()
    for number, contract in enumerate(contracts):
        folder = analysis / contract['id']
        folder.mkdir()
        print(f'v061 {number+1}/{len(contracts)}: {contract["id"]}, {contract["original_T"]}手区間', flush=True)
        before_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
        ctx.command([binary, contract['plan'], contract['start'], contract['end'], folder],
                    input_path=contract['input'], output=folder / 'probe.log', timeout=40)
        after_cpu = resource.getrusage(resource.RUSAGE_CHILDREN)
        write_json(folder / 'process.json', dict(cpu_sec=after_cpu.ru_utime+after_cpu.ru_stime-before_cpu.ru_utime-before_cpu.ru_stime))
        original = inspect(Path(contract['input']), Path(contract['plan']))
        for mode in ('finite', 'suffix'):
            data = json.loads((folder / mode / 'summary.json').read_text())
            if ((data['saved'] > 0) != (folder / mode / 'best.txt').is_file() or
                    data['same_length_alternative'] != (folder / mode / 'equal.txt').is_file()):
                raise RuntimeError('診断結果と保存された操作列が一致しない')
            end = contract['end'] if mode == 'finite' else len(original['operations'])
            changes = []
            for name in ('best', 'equal'):
                found = folder / mode / (name + '.txt')
                if not found.exists():
                    continue
                lines = found.read_text().splitlines()
                full = folder / mode / (name + '_full.txt')
                save_plan(full, actions(original, 0, contract['start']) + lines + actions(original, end))
                checked = inspect(Path(contract['input']), full)
                if checked['states'][contract['start']+len(lines)] != original['states'][end]:
                    raise RuntimeError('自動探索の出口が全盤面で一致しない')
                if name == 'best' and (len(lines) >= end-contract['start'] or len(lines) != data['best_T']):
                    raise RuntimeError('自動短縮の手数が診断ログと一致しない')
                if name == 'equal' and (len(lines) != end-contract['start'] or lines == actions(original, contract['start'], end)):
                    raise RuntimeError('同手数の別解が診断条件と一致しない')
                changes.append(dict(kind=name, T=len(lines), E=checked['metrics']['E'], full_plan=str(full), sha256=digest(full)))
                if mode == 'finite' and name == 'best':
                    improved.add(contract['case'])
            write_json(folder / mode / 'independent_verification.json', changes)
            rows.append(dict(example=contract['id'], case=contract['case'], mode=mode, **data))
    write_csv(analysis / 'probes.csv', rows, fields=list(rows[0]) if rows else ['example', 'case', 'mode', 'saved'])
    result = dict(status='diagnosed', integration_evidence=len(improved) >= 2, improved_long_cases=sorted(improved),
                  selected_examples=len(examples), missing_slots=len(missing), contracts=len(contracts),
                  boundary_analysis_seconds=analysis_seconds, boundary_analysis_cpu_seconds=analysis_cpu_seconds, ordinary_evaluation=False,
                  limitation='既知の改善境界を指定した診断。逐次再挿入の未発見から表現不能とは判定しない')
    write_json(analysis / 'summary.json', result)
    return result
