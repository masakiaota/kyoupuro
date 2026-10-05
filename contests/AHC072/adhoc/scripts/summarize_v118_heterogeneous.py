#!/usr/bin/env python3
"""保存結果だけから入力別差分と構築・育成の発動記録を共有用にまとめる。"""
import collections
import csv
import json
import re
import statistics
from build_v118_heterogeneous import ROOT, RUN
from v089_data import save, sha

def main():
    result=json.loads((RUN/'result.json').read_text());assessment=result['assessment']
    current=assessment['validation']['mixed'];baseline=assessment['validation']['base'];rows=current['rows']
    observed=dict(classical_initial_rank=collections.Counter(),classical_pilot_wins=0,classical_final_wins=0,
                  classical_retained_cases=0,candidate_count=collections.Counter(),best_classical_kind=collections.Counter())
    for row in rows:
        c=row['construction'];t=row['trace']
        idx=next((i for i in range(4) if c.get(f'v311_seed_{i}_origin')==-118),None)
        if idx is not None:
            observed['classical_retained_cases']+=1;observed['classical_initial_rank'][idx]+=1
            observed['classical_final_wins']+=c.get('v311_winner')==idx
            observed['classical_pilot_wins']+=min(range(t['race_seed_count']),key=lambda i:(c[f'v311_seed_{i}_pilot'],i))==idx
        observed['candidate_count'][t.get('race_seed_count')]+=1
        observed['best_classical_kind'][t['heterogeneous_best_kind']]+=1
    previous={r['case']:r for r in baseline['rows']}
    observed.update(
        mean_classical_minus_first_nn_ops=statistics.mean(x['trace']['heterogeneous_best_ops']-x['trace']['nn_initial_ops'] for x in rows),
        mean_lns_attempts=statistics.mean(x['trace']['lns_attempts'] for x in rows),
        baseline_mean_lns_attempts=statistics.mean(x['trace']['lns_attempts'] for x in baseline['rows']),
        errors={k:sum(x['trace'].get(k,0) for x in rows) for k in ['construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery']},
        initial_nn_same_hash=sum(x['trace']['nn_initial_hash']==previous[x['case']]['trace']['nn_initial_hash'] for x in rows))
    ms=[]
    for path in (RUN/'mixed/evaluation/validation/outputs').glob('*.err'):
        times={k:float(v) for k,v in re.findall(r'\[summary.time_ms\] (\w+)=([\d.]+)',path.read_text())}
        ms.append(times['heterogeneous'])
    observed.update(classical_ms_mean=statistics.mean(ms),classical_ms_max=max(ms))
    save(RUN/'observations.json',observed)
    final=result['final'];candidate=ROOT/final['candidate']['source']
    shared=ROOT/'adhoc/v118';shared.mkdir(parents=True,exist_ok=True)
    summary=dict(selected=assessment['selected'],retained=final.get('retained'),candidate=final['candidate'],
                 validation={k:v['metrics'] for k,v in assessment['validation'].items()},
                 differences=assessment['differences'],activation=assessment['activation'],observations=observed,
                 mechanism={k:v for k,v in assessment['mechanism'].items() if k!='reports'},
                 retained_final=dict(test=final['test']['metrics'],tools_in=final['tools_in']['metrics']),
                 reused_final=final.get('reused_control',False),goal_190=final['tools_in']['metrics']['mean_T_completed']<190,
                 source_hash_verified=sha(candidate)==final['candidate']['source_sha256'],
                 official_source=str(RUN.relative_to(ROOT)/'result.json'),completed_at=result['completed_at'])
    save(shared/'result_summary.json',summary)
    with (shared/'validation_paired.csv').open('w',newline='') as stream:
        writer=csv.writer(stream,lineterminator='\n')
        writer.writerow(['case','v115_T','v118_T','difference','v118_E','elapsed_ms','classical_raw_T','first_nn_T','classical_won'])
        for row in rows:
            c=row['construction'];winner=int(c['v311_winner'])
            writer.writerow([row['case'],previous[row['case']]['T'],row['T'],row['T']-previous[row['case']]['T'],
                             row['E'],row['elapsed_ms'],row['trace']['heterogeneous_best_ops'],row['trace']['nn_initial_ops'],
                             int(c.get(f'v311_seed_{winner}_origin')==-118)])
    print(json.dumps(summary,ensure_ascii=False,indent=2))

if __name__=='__main__':main()
