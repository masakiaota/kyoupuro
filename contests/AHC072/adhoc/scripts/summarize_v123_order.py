#!/usr/bin/env python3
"""保存済みの学習・C++診断・公式評価を集計し、入力別比較を共有する。"""
import csv
import gzip
import json
import re
import shutil
import statistics
import numpy as np
from build_v123_order import ROOT, RUN, BASE
from v089_data import save, sha, now


def load(path):return json.loads(path.read_text())


def main():
    trained=load(RUN/'training/result.json')
    choices=np.load(RUN/'training/development_selection.npz')
    selection={int(i):int(s) for i,s in zip(choices['indices'],choices['selected'])}
    per_case=[];index=0;common=0;common_delta=0
    for case in load(RUN/'inputs.json'):
        totals=dict(case=case['index'],groups=0,baseline_complete=0,model_complete=0,
                    mean_difference=0.,sum_difference=0,common_complete=0,common_difference=0,
                    wins=0,ties=0,losses=0,oracle_difference=0)
        if case['order_role']!='development':
            index+=load(RUN/'data'/f"{case['index']:06d}"/'result.json')['groups']
            continue
        with gzip.open(RUN/'data'/f"{case['index']:06d}"/'rows.jsonl.gz','rt') as stream:
            for line in stream:
                row=json.loads(line)
                if not row['extracted']:continue
                at=index;index+=1
                if at not in selection:continue
                base=min(row['costs'][0],row['costs'][row['alternate']])
                selected=min(row['costs'][0],row['costs'][selection[at]])
                difference=selected-base
                totals['groups']+=1;totals['sum_difference']+=difference
                totals['baseline_complete']+=base<=row['allowance']
                totals['model_complete']+=selected<=row['allowance']
                totals['wins']+=difference<0;totals['ties']+=difference==0;totals['losses']+=difference>0
                totals['oracle_difference']+=min(row['costs'])-base
                if max(base,selected)<=row['allowance']:
                    common+=1;common_delta+=difference
                    totals['common_complete']+=1;totals['common_difference']+=difference
        if totals['groups']:
            totals['mean_difference']=totals['sum_difference']/totals['groups'];per_case.append(totals)
    assert len(per_case)==64 and sum(row['groups'] for row in per_case)==trained['development']['groups']
    assert sum(row['sum_difference'] for row in per_case)==trained['development']['sum_difference']
    result=load(RUN/'result.json');final=result['final'];assessment=result['assessment']
    mechanism=load(RUN/'mechanism/result.json')
    source=ROOT/final['candidate']['source'];assert sha(source)==final['candidate']['source_sha256']
    history=load(RUN/'training/history.json')
    summary=dict(selected=assessment['selected'],candidate=final['candidate'],
                 collection=load(RUN/'full/result.json'),training=trained,
                 scaling_seconds=load(RUN/'scaling/exit.json')['seconds'],
                 training_loss_first=history[0]['loss'],training_loss_last=history[-1]['loss'],
                 common_completed_development=dict(groups=common,sum_difference=common_delta,mean_difference=common_delta/common),
                 development_case_wins=sum(r['sum_difference']<0 for r in per_case),
                 development_case_ties=sum(r['sum_difference']==0 for r in per_case),
                 development_case_losses=sum(r['sum_difference']>0 for r in per_case),
                 completion_cost_is_proxy=True,
                 mechanism={k:v for k,v in mechanism.items() if k!='runtime'},
                 validation={k:v['metrics'] for k,v in assessment['validation'].items()},
                 differences=assessment['differences'],activation=assessment['activation'],
                 final={k:final[k]['metrics'] for k in ['test','tools_in']},
                 final_differences=final.get('differences'),
                 final_reused=final.get('reused_control',False),final_accepted=final['accepted'],
                 goal_190=final['tools_in']['metrics']['mean_T_completed']<190,
                 model_sha256=sha(RUN/'training/model.json'),
                 official_source=str(RUN.relative_to(ROOT)/'result.json'),completed_at=now())
    current=assessment['validation']['learned'];before=assessment['validation']['base']
    rows=current['rows'];previous={r['case']:r for r in before['rows']}
    times=[]
    for path in sorted((RUN/'learned/evaluation/validation/outputs').glob('*.err')):
        times.append({k:float(v) for k,v in re.findall(r'\[summary.time_ms\] (\w+)=([\d.]+)',path.read_text())})
    assert len(times)==len(rows)==256
    summary['observations']=dict(
        mean_lns_attempts=statistics.mean(r['trace']['lns_attempts'] for r in rows),
        baseline_mean_lns_attempts=statistics.mean(r['trace']['lns_attempts'] for r in before['rows']),
        initial_nn_same_hash=sum(r['trace']['nn_initial_hash']==previous[r['case']]['trace']['nn_initial_hash'] for r in rows),
        active_cases=sum(r['trace'].get('order_nn_calls',0)>0 for r in rows),
        mean_order_nn_ms=statistics.mean(t.get('order_nn',0) for t in times),
        mean_order_nn_calls=statistics.mean(r['trace'].get('order_nn_calls',0) for r in rows),
        pool_returned_cases=sum(r['trace']['state_pool_free_at_end']==4 for r in rows),
        errors={k:sum(r['trace'].get(k,0) for r in rows) for k in
                ['construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery']})
    shared=ROOT/'adhoc/v123';shared.mkdir(parents=True,exist_ok=True)
    save(shared/'result_summary.json',summary);save(RUN/'summary.json',summary)
    shutil.copy2(RUN/'training/model.json',shared/'model.json')
    with (shared/'development_by_input.csv').open('w',newline='') as stream:
        writer=csv.DictWriter(stream,fieldnames=list(per_case[0]),lineterminator='\n')
        writer.writeheader();writer.writerows(per_case)
    with (shared/'validation_paired.csv').open('w',newline='') as stream:
        writer=csv.writer(stream,lineterminator='\n')
        writer.writerow(['case','v115_T','v123_T','difference','v123_E','elapsed_ms','v115_lns_attempts','v123_lns_attempts','order_nn_calls','order_nn_changed'])
        for r in rows:
            p=previous[r['case']];writer.writerow([r['case'],p['T'],r['T'],r['T']-p['T'],r['E'],r['elapsed_ms'],
                p['trace']['lns_attempts'],r['trace']['lns_attempts'],r['trace'].get('order_nn_calls',0),r['trace'].get('order_nn_changed',0)])
    print(json.dumps({k:summary[k] for k in ['selected','differences','observations','common_completed_development','final','goal_190']},ensure_ascii=False,indent=2))


if __name__=='__main__':main()
