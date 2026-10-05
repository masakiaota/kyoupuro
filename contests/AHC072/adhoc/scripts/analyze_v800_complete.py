#!/usr/bin/env python3
"""v800の保存記録を全件照合し、短縮区間と探索配分を集計する。探索は実行しない。"""

import argparse
from collections import Counter, defaultdict
import csv
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import statistics

from analyze_v047_coordination import inspect, closed_segments, transcript


ROOT = Path(__file__).resolve().parents[2]


def read_json(path):
    return json.loads(path.read_text())


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')


def write_csv(path, rows):
    if not rows:
        return
    fields = list(dict.fromkeys(k for r in rows for k in r))
    with path.open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def contacts(operations):
    result = defaultdict(list)
    for op in operations:
        for key in ('source', 'destination'):
            result[tuple(op[key])].append(op['action'])
    return dict(result)


def interval_features(before, after, bounds):
    a0, a1, b0, b1 = bounds
    old = before['operations'][a0:a1]
    new = after['operations'][b0:b1]
    old_contacts, new_contacts = contacts(old), contacts(new)
    cells = set(old_contacts) | set(new_contacts)
    changed = {p for p in cells if old_contacts.get(p) != new_contacts.get(p)}
    old_actions, new_actions = Counter(o['action'] for o in old), Counter(o['action'] for o in new)
    # 共通操作の個数は文面の一致であり、同じ役割・同じ個体という主張には使わない。
    return dict(before_start=a0, before_end=a1, after_start=b0, after_end=b1,
                before_moves=a1-a0, after_moves=b1-b0, saved=(a1-a0)-(b1-b0),
                cells=len(cells), changed_contact_cells=len(changed),
                old_unmatched_actions=sum((old_actions-new_actions).values()),
                new_unmatched_actions=sum((new_actions-old_actions).values()),
                returned_before=sum(len(o['returned_source'])+len(o['returned_destination']) for o in old),
                nonempty_exit=any(before['states'][a1]))


def shortest_improving_bounds(before, after):
    """全盤面が厳密に等しい2境界を列挙し、元の手数が最短の短縮区間を返す。"""
    index = defaultdict(list)
    for j, state in enumerate(after['states']):
        index[state].append(j)
    pairs = [(i, j) for i, state in enumerate(before['states']) for j in index[state]]
    best = None
    for k, (a0, b0) in enumerate(pairs):
        for a1, b1 in pairs[k+1:]:
            if best is not None and a1-a0 > best[0][0]:
                break
            saved = a1-a0-(b1-b0)
            if a1 <= a0 or b1 < b0 or saved <= 0:
                continue
            key = (a1-a0, -saved, a0, b0, a1, b1)
            if best is None or key < best[0]:
                best = (key, (a0, a1, b0, b1))
    assert best is not None
    return best[1]


def deferred_passenger(before, after, bounds):
    """保存差分のうち、2跳躍を1跳躍にし、一部を出発点へ残した狭い型を数える。"""
    a0, a1, b0, b1 = bounds
    old, new = before['operations'][a0:a1], after['operations'][b0:b1]
    old_counts, new_counts = Counter(o['action'] for o in old), Counter(o['action'] for o in new)

    def unmatched(ops, remaining):
        selected = []
        for op in ops:
            if remaining[op['action']]:
                selected.append(op)
                remaining[op['action']] -= 1
        return selected

    removed = unmatched(old, old_counts-new_counts)
    added = unmatched(new, new_counts-old_counts)
    if len(removed) != 2 or len(added) != 1:
        return None
    first, second = removed
    direct = added[0]
    if not (first['destination'] == second['source'] and first['source'] == direct['source']
            and second['destination'] == direct['destination']
            and first['action'].split()[3] == second['action'].split()[3] == direct['action'].split()[3]
            and first['length'] + second['length'] == direct['length']
            and first['before_source'] == direct['before_source']
            and first['moving'][::-1] == second['moving']
            and direct['k'] > first['k']
            and not first['returned_source'] and not first['returned_destination']):
        return None
    return dict(retained=direct['k']-first['k'], old_load=len(first['moving']), new_load=len(direct['moving']),
                old_lengths=[first['length'], second['length']], new_length=direct['length'],
                monochrome=len({id[0] for id in first['moving']}) == 1,
                first_turn=first['turn'], second_turn=second['turn'], direct_turn=direct['turn'])


def summarize_time(analysis, out):
    def read_rows(name):
        with (analysis/(name+'.csv')).open() as f:
            return list(csv.DictReader(f))
    cases = {r['case']: r for r in read_rows('cases') if r['cohort'] == 'fresh_5min'}
    events, progress, rounds, curves = [read_rows(n) for n in ('events', 'progress', 'rounds', 'learning_curve')]
    periods, round_rows, curve_rows, cohort_rows = [], [], [], []
    groups = dict(short=[c for c, r in cases.items() if int(r['initial_T']) < 150],
                  middle=[c for c, r in cases.items() if 150 <= int(r['initial_T']) < 300],
                  long=[c for c, r in cases.items() if int(r['initial_T']) >= 300])
    for label, ids in groups.items():
        for start in range(0, 300, 60):
            es = [e for e in events if e['case'] in ids and e['mode'] == 'continuous' and int(e['worker_saved']) > 0
                  and start <= float(e['elapsed_sec']) < start+60]
            ps = [p for p in progress if p['case'] in ids and p['mode'] == 'continuous'
                  and start <= float(p['elapsed_sec']) < start+60]
            periods.append(dict(group=label, start_sec=start, end_sec=start+60,
                saved=sum(int(e['worker_saved']) for e in es), events=len(es),
                worse_before=sum(int(e['before_T']) > int(e['previous_best_T']) for e in es),
                progress_samples=len(ps), nonbest_samples=sum(int(p['current_T']) > int(p['T']) for p in ps),
                mean_temperature=statistics.mean(float(p['temperature']) for p in ps)))
        for number in range(6):
            rs = [r for r in rounds if r['case'] in ids and r['mode'] == 'multistart' and int(r['round']) == number]
            round_rows.append(dict(group=label, round=number, cases=len(rs),
                worker_saved=sum(int(r['worker_saved']) for r in rs),
                within_round_saved=sum(int(r['initial_T'])-int(r['final_T']) for r in rs)))
        for mark in sorted({int(r['elapsed_sec']) for r in curves}):
            xs = [r for r in curves if r['case'] in ids and int(r['elapsed_sec']) == mark]
            curve_rows.append(dict(group=label, elapsed_sec=mark,
                **{k: sum(int(r[k]) for r in xs) for k in ('continuous_T', 'multistart_T', 'best_T', 'saved')}))
    for label, subset in [('through_0047', [r for c, r in cases.items() if int(c) <= 47]),
                          ('0048_to_0099', [r for c, r in cases.items() if int(c) > 47])]:
        wins = Counter(r['winner'] for r in subset)
        cohort_rows.append(dict(cohort=label, cases=len(subset), initial_T=sum(int(r['initial_T']) for r in subset),
                                saved=sum(int(r['saved']) for r in subset), **wins))
    write_csv(out/'continuous_periods.csv', periods)
    write_csv(out/'group_rounds.csv', round_rows)
    write_csv(out/'group_learning_curve.csv', curve_rows)
    write_csv(out/'cohorts.csv', cohort_rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run', type=Path)
    parser.add_argument('analysis', type=Path, help='analyze_v800_partial.py --through 0099 の出力')
    args = parser.parse_args()
    run, analysis = args.run.resolve(), args.analysis.resolve()
    if run == analysis or run in analysis.parents:
        parser.error('分析先は実行フォルダの外に置く')
    out = analysis / 'complete'
    out.mkdir(parents=True, exist_ok=True)
    summarize_time(analysis, out)
    manifest = read_json(run/'manifest.json')
    publication = read_json(run/'publication.json')
    assert read_json(run/'status.json')['status'] == 'completed'
    assert publication['status'] == 'published' and publication['cases'] == 100
    provenance = {x['case']: x for x in read_json(run/'provenance.json')}
    records = [json.loads(s) for s in (ROOT/'results/eval_records.jsonl').read_text().splitlines()]
    records = [r for r in records if r['run_id'] == publication['run_id']]
    assert len(records) == 100
    registered = {r['case_name'][:-4]: r for r in records}
    assert set(registered) == {f'{i:04}' for i in range(100)}
    case_map = {r['case']: r for r in manifest['cases']}
    metrics, audited = [], []
    # 参照値の公開を、実行記録・出力のハッシュ・独立再生で照合する。
    for case, entry in case_map.items():
        inp = run/entry['input']
        plan = run/'cases'/case/'best.txt'
        status = read_json(plan.parent/'status.json')
        record, prov = registered[case], provenance[case]
        report = inspect(inp, plan)
        T = report['metrics']['T']
        assert digest(inp) == entry['input_sha256'] == digest(ROOT/'tools/in'/f'{case}.txt')
        assert digest(plan) == status['best_sha256'] == prov['best_sha256'] == digest(ROOT/record['stdout_path'])
        assert T == status['T'] == prov['T'] == prov['official_score'] == record['score']
        assert T <= entry['reference_T'] and record['status'] == 'ok' and report['metrics']['E'] == 0
        assert len(status['attempts']) == (0 if entry['reused'] else 1)
        audited.append(dict(case=case, T=T, initial_T=entry['reference_T'], reused=entry['reused'],
                            elapsed_ms=status['elapsed_ms'], new_elapsed_ms=status['new_elapsed_ms'],
                            output_sha256=digest(plan), E=0))
        for stage, data in [('initial', inspect(inp, run/entry['seeds'][0]['path'])), ('best', report)]:
            ops = data['operations']
            metrics.append(dict(case=case, stage=stage, reused=entry['reused'], **data['metrics'],
                                moved_slimes=sum(len(o['moving']) for o in ops),
                                sum_jump_length=sum(o['length'] for o in ops),
                                moving_one=sum(len(o['moving']) == 1 for o in ops),
                                moving_at_least_four=sum(len(o['moving']) >= 4 for o in ops)))
    summary_rows = list(csv.DictReader((ROOT/'results/score_summary.csv').open()))
    published_summary = [r for r in summary_rows if r['bin'] == 'v800' and r['executed_at'] == records[0]['executed_at']]
    assert len(published_summary) == 1 and int(published_summary[0]['total_sum']) == sum(r['T'] for r in audited)
    detail_rows = list(csv.DictReader((ROOT/'results/score_detail.csv').open()))
    published_detail = [r for r in detail_rows if r['bin'] == 'v800' and r['executed_at'] == records[0]['executed_at']]
    assert len(published_detail) == 1
    for case in registered:
        assert int(published_detail[0][case+'.txt']) == registered[case]['score']
    write_csv(out/'publication_cases.csv', audited)
    write_csv(out/'route_metrics.csv', metrics)
    ordinary = {case for case, entry in case_map.items() if not entry['reused'] and case != '0000'}
    groups = {case: ('short' if case_map[case]['reference_T'] < 150 else
                    'middle' if case_map[case]['reference_T'] < 300 else 'long') for case in ordinary}
    events = list(csv.DictReader((analysis/'events.csv').open()))
    updates = [e for e in events if e['case'] in ordinary and e['mode'] == 'continuous' and int(e['worker_saved']) > 0]
    intervals, segments, event_hashes, motifs = [], [], {}, []
    witnesses = out/'witnesses'
    witnesses.mkdir(exist_ok=True)
    for n, event in enumerate(updates):
        inp = run/case_map[event['case']]['input']
        before_path, after_path = run/event['before_plan'], run/event['plan']
        before, after = inspect(inp, before_path), inspect(inp, after_path)
        assert len(before['operations']) == int(event['before_T'])
        assert len(after['operations']) == int(event['T'])
        assert after['metrics']['W'] == int(event['W'])
        for path in (before_path, after_path):
            event_hashes[str(path.relative_to(run))] = digest(path)
        common = {k: event[k] for k in ('case', 'reason', 'elapsed_sec', 'plan', 'before_plan', 'before_T', 'T', 'previous_best_T')}
        common.update(group=groups[event['case']], record_saved=int(event['worker_saved']))
        bounds = shortest_improving_bounds(before, after)
        intervals.append(dict(**common, **interval_features(before, after, bounds)))
        motif = deferred_passenger(before, after, bounds)
        if motif is not None:
            motifs.append(dict(**intervals[-1], **motif))
        for segment in closed_segments(before, after):
            bounds = tuple(segment[k] for k in ('before_start', 'before_end', 'after_start', 'after_end'))
            item = dict(**common, **interval_features(before, after, bounds))
            segments.append(item)
        # 後半ケースで複数手縮んだ保存例を、生成せずそのまま抜き書きする。
        selected_witness = (event['case'], after_path.stem) in {
            ('0081', '000020'), ('0089', '000031'), ('0092', '000053')}
        if selected_witness or (event['case'] in ('0081', '0092') and int(event['worker_saved']) >= 3):
            folder = witnesses/f"{event['case']}_{after_path.stem}"
            folder.mkdir(exist_ok=True)
            a0, a1, b0, b1 = shortest_improving_bounds(before, after)
            (folder/'before_operations.txt').write_text(transcript(before, a0, a1))
            (folder/'after_operations.txt').write_text(transcript(after, b0, b1))
            write_json(folder/'source.json', intervals[-1])
            old_contacts, new_contacts = contacts(before['operations']), contacts(after['operations'])
            changed = {p for p in set(old_contacts) | set(new_contacts) if old_contacts.get(p) != new_contacts.get(p)}
            for name, report in [('before', before), ('after', after)]:
                relevant = ''.join(transcript(report, op['turn'], op['turn']+1) for op in report['operations']
                                   if tuple(op['source']) in changed or tuple(op['destination']) in changed)
                (folder/(name+'_changed_cells.txt')).write_text(relevant)
        if (n+1) % 100 == 0:
            print(f'保存済み最良更新を再生: {n+1}/{len(updates)}', flush=True)
    write_csv(out/'shortest_intervals.csv', intervals)
    write_csv(out/'closed_segments.csv', segments)
    write_csv(out/'deferred_passengers.csv', motifs)
    interval_groups = []
    for group in ('short', 'middle', 'long', 'all'):
        subset = [r for r in intervals if group == 'all' or r['group'] == group]
        interval_groups.append(dict(group=group, events=len(subset),
            shortest_span_median=statistics.median(r['before_moves'] for r in subset),
            **{f'within_{cap}': sum(r['before_moves'] <= cap for r in subset) for cap in (8,16,32,64,128)},
            over_64_with_at_most_8_changed_old_actions=sum(r['before_moves'] > 64 and r['old_unmatched_actions'] <= 8 for r in subset)))
    write_csv(out/'interval_groups.csv', interval_groups)
    write_json(out/'audit.json', dict(run=str(run), analysis=str(analysis), created_at=datetime.now(timezone.utc).isoformat(),
        publication=publication, checked_cases=len(audited), replayed_update_pairs=len(updates),
        total_T=sum(r['T'] for r in audited), initial_total_T=sum(r['initial_T'] for r in audited),
        deferred_passenger_matches=len(motifs),
        manifest_sha256=digest(run/'manifest.json'), script_sha256=digest(Path(__file__)),
        event_files_sha256=event_hashes,
        method='保存済み操作列の独立再生と全盤面の厳密比較。新しい操作列の生成、solver実行、公式採点は行っていない。',
        caveats=['短縮区間は改善が起きた後に特定したもの。未知の短縮を探す費用は測っていない。',
                 '最短区間は1手以上縮む最短の元区間。最良更新全体の変化を説明するとは限らない。',
                 '共通操作の文面が一致しても、同じ個体や役割を持つとは限らない。']))
    print(json.dumps(dict(out=str(out), cases=len(audited), updates=len(updates), intervals=interval_groups), ensure_ascii=False), flush=True)


if __name__ == '__main__':
    main()
