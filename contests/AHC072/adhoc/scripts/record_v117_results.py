#!/usr/bin/env python3
"""v117の保存結果を整理する。solverの再評価は行わない。"""
import csv
import json
import re
from build_v117_pickup import ROOT, RUN
from v089_data import save, sha

def run():
    result=json.loads((RUN/'result.json').read_text());a=result['assessment'];v=a['validation']['pickup']
    shared=ROOT/'adhoc/v117';shared.mkdir(exist_ok=True)
    parent={r['filename']:r for r in a['validation']['parent']['rows']}
    baseline={r['filename']:r for r in a['validation']['base']['rows']}
    with (shared/'validation_paired.csv').open('w') as stream:
        writer=csv.writer(stream,lineterminator='\n');writer.writerow(['case','v113_T','v115_T','v117_T','vs_v115','S','E','elapsed_ms','pickup_selected'])
        for row in v['rows']:
            writer.writerow([row['filename'],parent[row['filename']]['T'],baseline[row['filename']]['T'],row['T'],
                             row['T']-baseline[row['filename']]['T'],row['S'],row['E'],row['elapsed_ms'],row['trace']['pickup_selected']])
    times=[float(re.search(r'\[summary.time_ms\] pickup=([\d.]+)',p.read_text()).group(1))
           for p in (RUN/'pickup/evaluation/validation/outputs').glob('*.err')]
    assert len(times)==256
    details=dict(mean_pickup_ms=sum(times)/len(times),max_pickup_ms=max(times),
                 mean_lns_attempts=sum(r['trace']['lns_attempts'] for r in v['rows'])/len(v['rows']),
                 state_pool_restored=all(r['trace']['state_pool_free_at_end']==4 for r in v['rows']),
                 errors={k:sum(r['trace'].get(k,0) for r in v['rows']) for k in
                         ['construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery']})
    assert details['state_pool_restored'] and not any(details['errors'].values())
    summary=dict(selected=a['selected'],accepted=False,goal_190=False,sources=a['sources'],
                 metrics={k:v['metrics'] for k,v in a['validation'].items()},differences=a['differences'],
                 parent_differences=a['parent_differences'],activation=a['activation'],mechanism=a['mechanism'],
                 details=details,retained_candidate=result['final']['candidate'],
                 tools_in_reused=result['final']['tools_in']['metrics'],result_sha256=sha(RUN/'result.json'),completed_at=result['completed_at'])
    save(shared/'result_summary.json',summary)
    print(json.dumps(dict(accepted=False,details=details),ensure_ascii=False))

if __name__=='__main__':run()
