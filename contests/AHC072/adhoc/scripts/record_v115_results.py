#!/usr/bin/env python3
"""v115の公式評価・独立再生の保存結果を共有用に整理する。再実行はしない。"""
import csv
import json
from build_v115_corridor import ROOT, RUN
from v089_data import save, sha

def run():
    result=json.loads((RUN/'result.json').read_text());assessment=result['assessment'];final=result['final']
    shared=ROOT/'adhoc/v115';shared.mkdir(exist_ok=True)
    datasets=dict(assessment['validation'],test=final['test'],tools_in=final['tools_in'])
    errors=['construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery']
    details={name:dict(state_pool_restored=all(r['trace']['state_pool_free_at_end']==4 for r in data['rows']),
                       errors={k:sum(r['trace'].get(k,0) for r in data['rows']) for k in errors},
                       mean_lns_attempts=sum(r['trace']['lns_attempts'] for r in data['rows'])/len(data['rows']))
             for name,data in datasets.items()}
    for name,data in datasets.items():
        if name=='base':continue
        old=assessment['validation']['base'] if name in ('r0','r1') else json.loads(
            (ROOT/'results/nn_rank/v113/20261004_integrated_studio/final/result.json').read_text())[name]
        indexed={r['filename']:r for r in old['rows']}
        with (shared/(name+'_paired.csv')).open('w') as stream:
            writer=csv.writer(stream,lineterminator='\n');writer.writerow(['case','v113_T','v115_T','difference','S','E','elapsed_ms','lns_attempts'])
            for row in data['rows']:
                before=indexed[row['filename']]
                writer.writerow([row['filename'],before['T'],row['T'],row['T']-before['T'],row['S'],row['E'],
                                 row['elapsed_ms'],row['trace']['lns_attempts']])
    assert all(d['state_pool_restored'] and not any(d['errors'].values()) for d in details.values())
    summary=dict(selected=assessment['selected'],accepted=final['accepted'],goal_190=final['goal_190'],
                 metrics={k:v['metrics'] for k,v in datasets.items()},differences=assessment['differences'],
                 final_differences=final['differences'],activation=assessment['activation'],details=details,
                 mechanism={k:v for k,v in assessment['mechanism'].items() if k!='rows'},
                 candidate=final['candidate'],result_sha256=sha(RUN/'result.json'),completed_at=result['completed_at'])
    save(shared/'result_summary.json',summary)
    print(json.dumps(dict(candidate=summary['candidate'],details=details),ensure_ascii=False))

if __name__=='__main__':run()
