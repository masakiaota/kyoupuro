#!/usr/bin/env python3
"""Independently verify v055 saved plans and the single ordinary evaluation."""
import csv
import hashlib
import json
from pathlib import Path
import sys

from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v055_audit'
BIN = 'v055_final_reductions'
PARENT = 'v050_joint_towers'
PARENT_RUN = '20260929T104058+0900_v050_joint_towers_71ae82'
KINDS = ('route', 'pair', 'joint', 'mono', 'strict', 'slack', 'finite')
MODES = ('repeat', 'mono', 'orientation', 'finite', 'combined')


def sources():
    audit = json.loads((AUDIT / 'static_verification.json').read_text())
    for name, key in ((BIN, 'solver_sha256'), (PARENT, 'parent_sha256')):
        assert hashlib.sha256((ROOT / f'src/bin/{name}.cpp').read_bytes()).hexdigest() == audit[key]
    for folder, key in ((ROOT / 'tools/in', 'input_sha256'),
                        (ROOT / 'results/out' / PARENT, 'parent_output_sha256')):
        assert len(audit[key]) == 100
        for name, digest in audit[key].items():
            assert hashlib.sha256((folder / name).read_bytes()).hexdigest() == digest
    return audit


def input_board(case):
    lines = (ROOT / 'tools/in' / case).read_text().splitlines()
    N, K = map(int, lines[0].split())
    C = lines[1:]
    towers = {(i,j): ([ord(c)-ord('a')] if c.islower() else [])
              for i, row in enumerate(C) for j,c in enumerate(row) if c != '#'}
    nests = {(i,j):ord(c)-ord('A') for i,row in enumerate(C) for j,c in enumerate(row) if c.isupper()}
    return N,K,towers,nests


def apply(towers,nests,move):
    i,j,k,d,length = move
    di,dj = {'U':(-1,0),'D':(1,0),'L':(0,-1),'R':(0,1)}[d]
    p = (i,j)
    assert p in towers and 0 <= k < len(towers[p]) and 1 <= length <= k+1
    for step in range(1,length+1):
        assert (i+di*step,j+dj*step) in towers
    q = (i+di*length,j+dj*length)
    assert len(towers[q])+len(towers[p])-k <= 8
    towers[q].extend(reversed(towers[p][k:]))
    del towers[p][k:]
    for v in (p,q):
        while towers[v] and towers[v][-1] == nests.get(v,-1):
            towers[v].pop()


def verify_pass(row):
    _,_,before,nests = input_board(row['case'])
    original,reduced = row['before'],row['after']
    first,old_end,new_end = row['first'],row['old_end'],row['new_end']
    assert original[:first] == reduced[:first] and original[old_end:] == reduced[new_end:]
    assert len(reduced) < len(original)
    for move in original[:first]:
        apply(before,nests,move)
    after = {p:list(w) for p,w in before.items()}
    for move in original[first:old_end]:
        apply(before,nests,move)
    for move in reduced[first:new_end]:
        apply(after,nests,move)
    assert before == after
    for move in reduced[new_end:]:
        apply(after,nests,move)
    assert all(not w for w in after.values())


def diagnostic():
    audit = sources()
    mechanism = json.loads((AUDIT / 'mechanism.json').read_text())
    assert mechanism['rng_consumption'] == 0 and mechanism['strict_equal_cases'] == 100
    assert mechanism['mono_positive'] > 0 and mechanism['mono_negative'] > 0
    assert mechanism['combination_checks'] == 5 and mechanism['witness_checks'] == 8
    rows = list(csv.DictReader((AUDIT / 'diagnostic.csv').open()))
    assert len(rows) == len({(r['case'],r['mode']) for r in rows}) == 500
    for row in rows:
        for key in row:
            if key not in ('case','mode'):
                row[key] = float(row[key]) if key == 'ms' else int(row[key])
        assert row['mode'] in MODES
        actual = verify_output(row['case'], AUDIT / row['mode'] / row['case'])
        assert actual == row['after'] <= row['before']
        assert row['before'] == len((ROOT / 'results/out' / PARENT / row['case']).read_text().splitlines())
        row['saved'] = row['before'] - actual
        assert row['saved'] == sum(row[k+'_saved'] for k in KINDS)
        assert row['stable'] == 1
        assert all(row[k] == 0 for k in ('deadlines','finite_budget_stops','joint_deadlines','finite_deadlines','finite_global_deadlines'))
    witnesses = list(csv.DictReader((AUDIT / 'witnesses.csv').open()))
    assert len(witnesses) == 4 and all(int(w['found']) == 1 for w in witnesses)
    for w in witnesses:
        verify_output(w['case']+'.txt', AUDIT / 'witnesses' / (w['label']+'_auto.txt'))
    passes = [json.loads(line) for line in (AUDIT / 'passes.jsonl').read_text().splitlines()]
    for row in passes:
        verify_pass(row)
    grouped = {mode:[r for r in rows if r['mode'] == mode] for mode in MODES}
    summaries = {mode:{'saved':sum(r['saved'] for r in rs), 'shortened_cases':sum(r['saved']>0 for r in rs),
                       'mean_ms':sum(r['ms'] for r in rs)/100, 'max_ms':max(r['ms'] for r in rs),
                       'by_kind':{k:sum(r[k+'_saved'] for r in rs) for k in KINDS},
                       'joint_caps':sum(r['joint_caps'] for r in rs), 'finite_caps':sum(r['finite_caps'] for r in rs),
                       'finite_beam_pruned':sum(r['finite_beam_pruned'] for r in rs),
                       'route_beam_caps':sum(r['route_beam_caps'] for r in rs)} for mode,rs in grouped.items()}
    by_key = {(r['case'],r['mode']):r for r in rows}
    comparisons = {}
    for mode in MODES[:-1]:
        differences = [by_key[case,'combined']['after'] - by_key[case,mode]['after'] for case in audit['input_sha256']]
        comparisons[mode] = {'delta_T':sum(differences), 'wins':sum(d<0 for d in differences),
                             'draws':sum(d==0 for d in differences), 'losses':sum(d>0 for d in differences)}
    # Best-of-controls is a descriptive comparison, never a solver selector.
    additional = []
    for case in audit['input_sha256']:
        best = min(by_key[case,m]['after'] for m in MODES[:-1])
        additional.append({'case':case,'delta_T':by_key[case,'combined']['after']-best})
    result = {'solver_sha256':audit['solver_sha256'], 'verified_outputs':504,
              'mechanism':mechanism, 'verified_changed_passes':len(passes), 'modes':summaries,
              'combined_vs_controls':comparisons, 'combined_vs_best_control':{
                  'delta_T':sum(r['delta_T'] for r in additional), 'cases':[r for r in additional if r['delta_T']]},
              'proceed_to_evaluation':summaries['combined']['saved']>0}
    (AUDIT / 'diagnostic_verification.json').write_text(json.dumps({**result,'cases':rows},indent=2)+'\n')
    print(json.dumps(result,indent=2))


def evaluation():
    audit = sources()
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').read_text().splitlines()]
    current = sorted([r for r in records if r['bin'] == BIN],key=lambda r:r['case_name'])
    parent = {r['case_name']:r for r in records if r['run_id'] == PARENT_RUN}
    assert len(current) == len(parent) == 100 and len({r['run_id'] for r in current}) == 1
    assert {r['case_name'] for r in current} == parent.keys()
    cases = []
    for r in current:
        assert r['status'] == 'ok' and r['local'] and r['input_dir'] == 'tools/in'
        path = ROOT / r['stdout_path']
        T = verify_output(r['case_name'],path)
        c,t,diagnostics = log_values(path.with_suffix('.txt.err'))
        assert T == r['score'] == c['T'] == c['final_ops'] == c['validated_moves']
        assert c['E'] == 0 and T <= 100000 and c['board_pool_free_at_end'] == c['board_pool_slots']
        assert c['pre_joint_ops']-c['pre_final_reductions_ops'] == c['joint_window_saved']
        assert c['pre_final_reductions_ops']-T == c['final_reductions_saved']
        assert c['final_reductions_saved'] == sum(c['final_reductions_'+k+'_saved'] for k in KINDS)
        assert c['pre_pair_ops']-c['pre_joint_ops'] == c['pair_transfer_saved']-c['final_reductions_pair_saved']
        assert c['pre_lns_ops']-c['pre_pair_ops'] == c['lns_saved']
        assert t['search_limit'] == 1544.0
        p = parent[r['case_name']]
        old,_,_ = log_values((ROOT / p['stdout_path']).with_suffix('.txt.err'))
        cases.append({**r, 'parent_T':p['score'], 'delta_T':T-p['score'],
                      'pre_lns_delta':c['pre_lns_ops']-old['pre_lns_ops'],
                      'parent_attempts':old['lns_attempts'],'counts':c,'times_ms':t,'diagnostics':diagnostics})
    total = sum(r['score'] for r in cases)
    baseline = sum(r['score'] for r in parent.values())
    count_keys = set.union(*(set(r['counts']) for r in cases))
    counts = {key:sum(r['counts'].get(key,0) for r in cases) for key in sorted(count_keys)
              if key.startswith(('final_reductions_','joint_window_','mono_dispatch_','coupled_routes_','finite_'))}
    errors = {key:sum(r['counts'][key] for r in cases) for key in ERRORS}
    attempts = sum(r['counts']['lns_attempts'] for r in cases)
    old_attempts = sum(r['parent_attempts'] for r in cases)
    result = {'run_id':current[0]['run_id'],'parent_run_id':PARENT_RUN,'solver_sha256':audit['solver_sha256'],
              'verified_cases':100,'total_T':total,'parent_T':baseline,'delta_T':total-baseline,
              'delta_percent':100*(total/baseline-1),'average_T':total/100,'normal99_average_T':(total-cases[0]['score'])/99,
              'wins':sum(r['delta_T']<0 for r in cases),'draws':sum(r['delta_T']==0 for r in cases),'losses':sum(r['delta_T']>0 for r in cases),
              'case0000_T':cases[0]['score'],'max_elapsed_ms':max(r['elapsed'] for r in cases),
              'errors':errors,'diagnostic_count':sum(len(r['diagnostics']) for r in cases),'counts':counts,
              'mean_reductions_ms':sum(r['times_ms']['final_reductions'] for r in cases)/100,
              'max_reductions_ms':max(r['times_ms']['final_reductions'] for r in cases),
              'mean_ms_by_kind':{k:sum(r['times_ms']['final_reductions_'+k] for r in cases)/100 for k in KINDS},
              'shortened_cases':sum(r['counts']['final_reductions_saved']>0 for r in cases),
              'attempts':attempts,'parent_attempts':old_attempts,'attempts_delta_percent':100*(attempts/old_attempts-1),
              'pre_lns_delta_T':sum(r['pre_lns_delta'] for r in cases),
              'pre_lns_changed_cases':sum(r['pre_lns_delta']!=0 for r in cases),
              'pre_reductions_delta_T':sum(r['counts']['pre_final_reductions_ops']-r['parent_T'] for r in cases)}
    result['adopt'] = (not any(errors.values()) and result['diagnostic_count']==0
                       and result['max_elapsed_ms']<=2000 and result['case0000_T']<=43
                       and total<baseline and counts['final_reductions_saved']>0)
    out = ROOT / 'results/analysis/v055'
    out.mkdir(parents=True,exist_ok=True)
    with (out / 'comparison.csv').open('w') as file:
        writer = csv.writer(file)
        writer.writerow(('case','parent_T','T','delta_T','pre_reductions_T',*(k+'_saved' for k in KINDS),'reductions_ms'))
        for r in cases:
            c = r['counts']
            writer.writerow((r['case_name'],r['parent_T'],r['score'],r['delta_T'],c['pre_final_reductions_ops'],
                             *(c['final_reductions_'+k+'_saved'] for k in KINDS),r['times_ms']['final_reductions']))
    (out / 'summary.json').write_text(json.dumps({**result,'cases':cases},indent=2)+'\n')
    print(json.dumps(result,indent=2))


if __name__ == '__main__':
    {'diagnostic':diagnostic,'evaluation':evaluation}[sys.argv[1]]()
