#!/usr/bin/env python3
"""完了済みv066を保存v059と比較する。探索・再評価・ソース変更はしない。"""
from pathlib import Path
from collections import Counter
import csv, hashlib, json, re, shutil, statistics
from replay_slime_output import replay

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v066_audit/20260930_submit'
PARENT_RUN = '20260929T235217+0900_v059_repair_priority_6879ca'
PARENT_OUT = ROOT / 'results/analysis/v059/20260929T235018_0cd76222/outputs'
LONG = set('0014 0015 0030 0034 0042 0047 0070 0071 0075 0081 0082 0084 0089 0092'.split())
NAME = 'v066_repair_deferred'

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def save(p, d):
    p.write_text(json.dumps(d, ensure_ascii=False, indent=2) + '\n')

def counters(p):
    text = p.read_text()
    return ({k: int(v) for k, v in re.findall(r'\[summary.count\] (\S+)=(-?\d+)', text)},
            {k: float(v) for k, v in re.findall(r'\[summary.time_ms\] (\S+)=([\d.]+)', text)})

def main():
    records = [json.loads(s) for s in (ROOT / 'results/eval_records.jsonl').read_text().splitlines() if s]
    parent = {r['case_name']: r for r in records if r['run_id'] == PARENT_RUN}
    selected = [r for r in records if r['bin'] == NAME]
    current = {r['case_name']: r for r in selected}
    assert len(selected) == len(current) == len(parent) == 100
    assert len({r['run_id'] for r in selected}) == 1
    run = selected[0]['run_id']
    out = ROOT / 'results/analysis/v066' / run
    (out / 'outputs').mkdir(parents=True, exist_ok=True)
    manifest = json.loads((AUDIT / 'manifest.json').read_text())
    for name, h in manifest['protected_files'].items():
        assert sha(ROOT / name) == h, name
    for name, h in manifest['source_files'].items():
        snapshot = AUDIT / 'snapshot' / Path(name).name
        assert sha(snapshot) == h, name
        if name == 'notes/experiments/v066.md':
            assert (ROOT / name).read_text().split('## 実験後', 1)[0] == snapshot.read_text().split('## 実験後', 1)[0]
        else:
            assert sha(ROOT / name) == h, name
    diagnostic = json.loads((AUDIT / 'mechanism.json').read_text())
    assert diagnostic['status'] == 'passed'
    rows, mechanism, hashes, errors = [], {}, {}, []
    count_sum, rank_sum = Counter(), Counter()
    times, max_span = [], 0
    for case, r in sorted(current.items()):
        p = parent[case]
        assert r['status'] == 'ok' and r['local'] and r['input_dir'] == 'tools/in'
        inp, source = ROOT / 'tools/in' / case, ROOT / r['stdout_path']
        stderr = source.with_suffix('.txt.err')
        assert sha(inp) == manifest['input_hashes'][case]
        assert sha(PARENT_OUT / case) == manifest['parent_output_hashes'][case]
        assert sha(PARENT_OUT / (case + '.err')) == manifest['data_hashes'][str((PARENT_OUT / (case + '.err')).relative_to(ROOT))]
        for f in [source, stderr]:
            dest = out / 'outputs' / f.name
            if dest.exists():
                assert sha(dest) == sha(f)
            else:
                shutil.copy2(f, dest)
        metrics = replay(inp, out / 'outputs' / case)['metrics']
        assert metrics['E'] == 0 and metrics['T'] == r['score'] and metrics['T'] <= 100000
        c, t = counters(stderr)
        pc, pt = counters(PARENT_OUT / (case + '.err'))
        for key in ['construction_errors', 'lns_errors', 'lns_invalid_candidates', 'baseline_recovery', 'final_recovery']:
            if c.get(key) != 0:
                errors.append(dict(case=case, key=key, value=c.get(key)))
        if c['board_pool_free_at_end'] != c['board_pool_slots']:
            errors.append(dict(case=case, key='board_pool'))
        assert c['final_ops'] == r['score']
        small = {k.removeprefix('deferred_passenger_'): v for k, v in c.items() if k.startswith('deferred_passenger_')}
        rank = {k.removeprefix('lns_repair_priority_'): v for k, v in c.items() if k.startswith('lns_repair_priority_')}
        count_sum.update(small)
        rank_sum.update(rank)
        max_span = max(max_span, small['max_span'])
        times.append(t['deferred_passenger'])
        mechanism[case] = dict(deferred=small, ranking=rank, counts=c, times_ms=t)
        # LNS終了後には2マス圧縮と共同短縮もあるため、最後の
        # final_reductions_savedだけを後処理全体の削減量にしない。
        assert pc['pre_lns_ops'] - pc['lns_saved'] == pc['pre_pair_ops']
        assert c['pre_lns_ops'] - c['lns_saved'] == c['pre_pair_ops']
        rows.append(dict(case=case[:-4], v059=p['score'], v066=r['score'], saved=p['score'] - r['score'],
            elapsed_ms=r['elapsed'], long14=case[:-4] in LONG, parent_pre_lns=pc['pre_lns_ops'],
            current_pre_lns=c['pre_lns_ops'], parent_lns_saved=pc['lns_saved'], current_lns_saved=c['lns_saved'],
            parent_final_saved=pc['pre_pair_ops'] - pc['final_ops'], current_final_saved=c['pre_pair_ops'] - c['final_ops'],
            parent_final_reductions_saved=pc['final_reductions_saved'], current_final_reductions_saved=c['final_reductions_saved'],
            parent_attempts=pc['lns_attempts'], current_attempts=c['lns_attempts'],
            parent_ranking_ms=pt['lns_repair_priority'], current_ranking_ms=t['lns_repair_priority'],
            deferred_ms=t['deferred_passenger'], **{'deferred_' + k: v for k, v in small.items()}))
        hashes[case] = dict(input=sha(inp), output=sha(source), stderr=sha(stderr))
    count_sum['max_span'] = max_span
    total, base = sum(r['v066'] for r in rows), sum(r['v059'] for r in rows)
    saved = base - total
    long_saved = sum(r['saved'] for r in rows if r['long14'])
    maximum = max(r['elapsed_ms'] for r in rows)
    active = rank_sum['penalized'] > 0 and rank_sum['selected'] > 0 and count_sum['accepted_saved'] > 0
    valid = maximum <= 2000 and current['0000.txt']['score'] <= 43 and not errors
    adopted = valid and active and saved > 0 and long_saved >= 0
    for filename in ['score_summary.csv', 'score_detail.csv']:
        with (ROOT / 'results' / filename).open() as f:
            matches = [r for r in csv.DictReader(f) if r['bin'] == NAME]
        assert len(matches) == 1
        if filename == 'score_summary.csv':
            assert int(matches[0]['total_sum']) == total
        else:
            assert all(int(matches[0][r['case'] + '.txt']) == r['v066'] for r in rows)
    decomposition = dict(pre_lns_added=sum(r['current_pre_lns'] - r['parent_pre_lns'] for r in rows),
                         lns_saved_added=sum(r['current_lns_saved'] - r['parent_lns_saved'] for r in rows),
                         final_saved_added=sum(r['current_final_saved'] - r['parent_final_saved'] for r in rows))
    assert -saved == decomposition['pre_lns_added'] - decomposition['lns_saved_added'] - decomposition['final_saved_added']
    summary = dict(run_id=run, parent_run_id=PARENT_RUN, status='evaluated', adopted=adopted,
        total=total, parent_total=base, mean=total / 100, parent_mean=base / 100, saved=saved,
        long14_saved=long_saved, wins=sum(r['saved'] > 0 for r in rows), ties=sum(r['saved'] == 0 for r in rows),
        losses=sum(r['saved'] < 0 for r in rows), max_elapsed_ms=maximum, case0000=current['0000.txt']['score'],
        valid_cases=100, E=0, errors=errors, mechanism_active=active, mechanism_counts=dict(count_sum),
        ranking_counts=dict(rank_sum), mean_deferred_ms=statistics.mean(times), max_deferred_ms=max(times),
        direct_shortened_cases=sum(r['deferred_saved'] > 0 for r in rows),
        accepted_shortened_cases=sum(r['deferred_accepted_saved'] > 0 for r in rows),
        best_updated_cases=sum(r['deferred_best_created'] > 0 for r in rows),
        long_span_cases=sum(r['deferred_max_span'] > 64 for r in rows),
        pre_lns_changed_cases=sum(r['current_pre_lns'] != r['parent_pre_lns'] for r in rows),
        decomposition=decomposition, diagnostic=diagnostic,
        top_gains=sorted(rows, key=lambda r: -r['saved'])[:5], top_losses=sorted(rows, key=lambda r: r['saved'])[:5],
        solver_executions_in_analysis=0)
    with (out / 'cases.csv').open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)
    save(out / 'summary.json', summary)
    save(out / 'mechanism.json', mechanism)
    save(out / 'hashes.json', hashes)
    save(ROOT / 'results/analysis/v066/latest.json', dict(run_id=run, path=str(out)))
    print(json.dumps({k: v for k, v in summary.items() if k not in ['top_gains', 'top_losses', 'ranking_counts']}, ensure_ascii=False, indent=2))
    print('saved', out)

if __name__ == '__main__':
    main()
