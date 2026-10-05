#!/usr/bin/env python3
"""Verify saved v043 results without running or modifying any solver."""
import csv
import hashlib
import json
from pathlib import Path
import sys

from check_v037_results import ERRORS, log_values, verify_output

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v043_audit'
BIN = 'v043_cooperative_events'
PARENT = 'v039_exact_board_lns'
PARENT_RUN = '20260928T092503+0900_v039_exact_board_lns_3c2389'


def verify_sources():
    audit = json.loads((AUDIT/'static_verification.json').read_text())
    for name, key in [(BIN,'solver_sha256'),(PARENT,'parent_sha256')]:
        assert hashlib.sha256((ROOT/f'src/bin/{name}.cpp').read_bytes()).hexdigest() == audit[key]
    for case, digest in audit['input_sha256'].items():
        assert hashlib.sha256((ROOT/'tools/in'/case).read_bytes()).hexdigest() == digest
    return audit


def diagnostic():
    audit = verify_sources()
    rows = list(csv.DictReader((AUDIT/'diagnostic.csv').open()))
    assert len({r['case'] for r in rows}) == 100
    for r in rows:
        for k in r:
            if k != 'case':
                r[k] = float(r[k]) if k == 'ms' else int(r[k])
    assert len({(r['case'],r['time']) for r in rows}) == len(rows)
    paths = sorted((AUDIT/'plans').glob('*.txt'))
    for p in paths:
        verify_output(p.name[:4]+'.txt',p)
    for r in rows:
        base = AUDIT/'plans'/f"{r['case']}_{r['time']}"
        if r['joint'] >= 0:
            assert verify_output(r['case']+'.txt',base.with_suffix('.txt')) == r['time']+r['joint']
            assert verify_output(r['case']+'.txt',base.with_name(base.name+'_polished.txt')) == r['time']+r['polished']
            assert r['polished'] <= r['joint'] <= r['original_tail']
        if r['sequential'] >= 0:
            assert verify_output(r['case']+'.txt',base.with_name(base.name+'_sequential.txt')) == r['time']+r['sequential']
    eligible = [r for r in rows if r['eligible']]
    completed = [r for r in rows if r['joint']>=0]
    shortened = [r for r in completed if r['joint']<r['original_tail']]
    comparison = {'joint_only':0,'sequential_only':0,'both_failed':0,'joint_shorter':0,'equal':0,'joint_longer':0}
    for r in eligible:
        a,b = r['joint'],r['sequential']
        key = ('both_failed' if b<0 else 'sequential_only') if a<0 else ('joint_only' if b<0 else 'joint_shorter' if a<b else 'equal' if a==b else 'joint_longer')
        comparison[key] += 1
    result = {'solver_sha256':audit['solver_sha256'],'queries':len(rows),'eligible':len(eligible),
              'completed':len(completed),'raw_shortened_queries':len(shortened),
              'raw_shortened_cases':len({r['case'] for r in shortened}),
              'verified_outputs':len(paths),'comparison':comparison,
              'mean_joint_ms':sum(r['ms'] for r in eligible)/max(1,len(eligible)),
              'max_joint_ms':max((r['ms'] for r in eligible),default=0),
              'manual0035':[r for r in rows if r['manual']],
              'mechanism':json.loads((AUDIT/'mechanism.json').read_text())}
    assert len(completed)>0 and result['mechanism']['completed']==len(completed)
    (AUDIT/'diagnostic_verification.json').write_text(json.dumps({**result,'cases':rows},indent=2)+'\n')
    print(json.dumps(result,indent=2))


def evaluation():
    audit = verify_sources()
    records = [json.loads(line) for line in (ROOT/'results/eval_records.jsonl').read_text().splitlines()]
    current = [r for r in records if r['bin']==BIN]
    parent = {r['case_name']:r for r in records if r['run_id']==PARENT_RUN}
    assert len(current)==len(parent)==100 and len({r['run_id'] for r in current})==1
    cases=[]
    for record in sorted(current,key=lambda r:r['case_name']):
        assert record['status']=='ok' and record['local'] and record['input_dir']=='tools/in'
        path=ROOT/record['stdout_path'];T=verify_output(record['case_name'],path)
        c,t,d=log_values(path.with_suffix('.txt.err'))
        assert T==record['score']==c['T']==c['final_ops']==c['validated_moves'] and c['E']==0
        assert c['pair_transfer_saved']==c['pre_pair_ops']-T
        assert c['lns_saved']==c['pre_lns_ops']-c['pre_pair_ops']
        baseline=parent[record['case_name']]
        pc,_,_=log_values((ROOT/baseline['stdout_path']).with_suffix('.txt.err'))
        cases.append({**record,'parent_T':baseline['score'],'delta_T':T-baseline['score'],
                      'pre_lns_delta':c['pre_lns_ops']-pc['pre_lns_ops'],
                      'parent_attempts':pc['lns_attempts'],'counts':c,'times_ms':t,'diagnostics':d})
    total=sum(r['score'] for r in cases);baseline=sum(r['score'] for r in parent.values())
    counts={key:sum(r['counts'][key] for r in cases) for key in cases[0]['counts'] if key.startswith('cooperative_')}
    errors={key:sum(r['counts'].get(key,0) for r in cases) for key in ERRORS}
    normal=[r for r in cases if r['case_name']!='0000.txt']
    attempts=sum(r['counts']['lns_attempts'] for r in cases);old_attempts=sum(r['parent_attempts'] for r in cases)
    result={'run_id':current[0]['run_id'],'parent_run_id':PARENT_RUN,'solver_sha256':audit['solver_sha256'],
            'verified_cases':100,'total_T':total,'parent_T':baseline,'delta_T':total-baseline,
            'delta_percent':100*(total/baseline-1),'wins':sum(r['delta_T']<0 for r in cases),
            'draws':sum(r['delta_T']==0 for r in cases),'losses':sum(r['delta_T']>0 for r in cases),
            'case0000_T':cases[0]['score'],'max_elapsed_ms':max(r['elapsed'] for r in cases),
            'errors':errors,'diagnostic_count':sum(len(r['diagnostics']) for r in cases),'counts':counts,
            'normal_raw_improved_cases':sum(r['counts']['cooperative_raw_improvements']>0 for r in normal),
            'normal_best_updated_cases':sum(r['counts']['cooperative_best_updates']>0 for r in normal),
            'mean_cooperative_ms':sum(r['times_ms']['cooperative'] for r in cases)/100,
            'max_cooperative_ms':max(r['times_ms']['cooperative'] for r in cases),
            'attempts':attempts,'parent_attempts':old_attempts,'attempts_delta_percent':100*(attempts/old_attempts-1),
            'pre_lns_delta_T':sum(r['pre_lns_delta'] for r in cases),
            'source_added_lines':len((ROOT/f'src/bin/{BIN}.cpp').read_text().splitlines())-len((ROOT/f'src/bin/{PARENT}.cpp').read_text().splitlines())}
    result['adopt']=not any(errors.values()) and result['diagnostic_count']==0 and result['max_elapsed_ms']<=2000 and result['case0000_T']<=43 and total<baseline and result['normal_raw_improved_cases']>0 and result['normal_best_updated_cases']>0
    with (ROOT/'adhoc/v043_comparison.csv').open('w') as file:
        writer=csv.writer(file);writer.writerow(('case','parent_T','T','delta_T','raw_improvements','raw_saved','best_updates','best_saved','cooperative_ms'))
        for r in cases:
            c=r['counts'];writer.writerow((r['case_name'],r['parent_T'],r['score'],r['delta_T'],c['cooperative_raw_improvements'],c['cooperative_raw_saved'],c['cooperative_best_updates'],c['cooperative_best_saved'],r['times_ms']['cooperative']))
    (ROOT/'adhoc/v043_evaluation_summary.json').write_text(json.dumps({**result,'cases':cases},indent=2)+'\n')
    print(json.dumps(result,indent=2))


if __name__=='__main__':
    {'diagnostic':diagnostic,'evaluation':evaluation}[sys.argv[1]]()
