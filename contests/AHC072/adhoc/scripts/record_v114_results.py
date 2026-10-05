#!/usr/bin/env python3
"""v114の保存結果を共有する。solverは再実行しない。"""
import csv
import json
import re
from build_v114_cooperative import ROOT, RUN
from v089_data import save, sha

def run():
    result=json.loads((RUN/'result.json').read_text());assessment=result['assessment']
    shared=ROOT/'adhoc/v114';shared.mkdir(exist_ok=True)
    metrics={k:v['metrics'] for k,v in assessment['validation'].items()}
    details={}
    for label in ('b08','b20'):
        data=assessment['validation'][label];elapsed=[]
        for p in (RUN/label/'evaluation/validation/outputs').glob('*.err'):
            elapsed.append(float(re.search(r'\[summary.time_ms\] cooperative=([\d.]+)',p.read_text()).group(1)))
        assert len(elapsed)==256
        details[label]=dict(mean_cooperative_ms=sum(elapsed)/len(elapsed),max_cooperative_ms=max(elapsed),
                           cases_with_completion=sum(r['trace']['cooperative_completed']>0 for r in data['rows']),
                           state_pool_restored=all(r['trace']['state_pool_free_at_end']==4 for r in data['rows']),
                           errors={k:sum(r['trace'].get(k,0) for r in data['rows']) for k in
                                   ['construction_errors','lns_errors','baseline_recovery','final_recovery','lns_invalid_candidates']})
        parent={r['filename']:r for r in assessment['validation']['base']['rows']}
        with (shared/(label+'_paired.csv')).open('w') as stream:
            w=csv.writer(stream);w.writerow(['case','v113_T','v114_T','difference','S','E','elapsed_ms','cooperative_completed'])
            for row in data['rows']:
                before=parent[row['filename']]
                w.writerow([row['filename'],before['T'],row['T'],row['T']-before['T'],row['S'],row['E'],
                            row['elapsed_ms'],row['trace']['cooperative_completed']])
    summary=dict(selected=assessment['selected'],accepted=False,goal_190=False,
                 metrics=metrics,differences=assessment['differences'],activation=assessment['activation'],
                 mechanism=assessment['mechanism'],details=details,sources=assessment['sources'],
                 retained_candidate=result['final']['candidate'],
                 tools_in_reused=result['final']['tools_in']['metrics'],
                 result_sha256=sha(RUN/'result.json'),completed_at=result['completed_at'])
    save(shared/'result_summary.json',summary)
    print(json.dumps(details,ensure_ascii=False))

if __name__=='__main__':run()
