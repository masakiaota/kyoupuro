#!/usr/bin/env python3
"""Independently replay and summarize the single v056 evaluation."""
import csv
import json
from pathlib import Path
from audit_v056 import ROOT,NAMES,digest
from run_v056 import unchanged,ADDED
from check_v037_results import ERRORS,log_values,verify_output
AUDIT=ROOT/'adhoc/v056_audit'
BIN=NAMES['child']
PARENT=NAMES['parent']
PARENT_RUN='20260929T134646+0900_v055_final_reductions_63082d'
KINDS=('route','pair','joint','mono','strict','slack','finite')

def sources():
    unchanged()
    report=json.loads((AUDIT/'static_verification.json').read_text())
    for name,value in report['sources'].items():
        assert digest(ROOT/f'src/bin/{name}.cpp')==value
    return report['sources']

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
              if key.startswith(('final_reductions_','joint_window_','mono_dispatch_','coupled_routes_','finite_')) or key in ADDED}
    errors = {key:sum(r['counts'][key] for r in cases) for key in ERRORS}
    attempts = sum(r['counts']['lns_attempts'] for r in cases)
    old_attempts = sum(r['parent_attempts'] for r in cases)
    result = {'run_id':current[0]['run_id'],'parent_run_id':PARENT_RUN,'solver_sha256':audit[NAMES['child']],
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
    work = json.loads((AUDIT / 'fixed_work_summary.json').read_text())
    result['speed_passed'] = work['speed_passed']
    result['fixed_work_change_percent'] = work['normal99']['total_ns']['median_change_percent']
    result['mechanism_cases'] = {k:sum(r['counts'].get(k,0)>0 for r in cases) for k in ADDED}
    assert all(counts[k]>0 for k in ADDED)
    result['adopt'] = (not any(errors.values()) and result['diagnostic_count']==0
                       and result['max_elapsed_ms']<=2000 and result['case0000_T']<=43
                       and total<=baseline and result['speed_passed'])
    out = ROOT / 'results/analysis/v056'
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


if __name__=='__main__':
    evaluation()
