#!/usr/bin/env python3
"""保存済みの時刻切替比較を集計する。solverを再実行しない。"""
import csv
import json
import statistics
from build_v125_late import ROOT, RUN
from v089_data import save, sha

def main():
    result=json.loads((RUN/'result.json').read_text());a=result['assessment']
    sets=a['validation'];base={r['case']:r for r in sets['base']['rows']};full={r['case']:r for r in sets['full']['rows']}
    observations={}
    for label,item in sets.items():
        rows=item['rows'];observations[label]=dict(
            mean_lns_attempts=statistics.mean(r['trace']['lns_attempts'] for r in rows),
            nn_same_hash=sum(r['trace']['nn_initial_hash']==base[r['case']]['trace']['nn_initial_hash'] for r in rows),
            pool_returned=sum(r['trace']['state_pool_free_at_end']==4 for r in rows),
            errors={k:sum(r['trace'].get(k,0) for r in rows) for k in ['lns_errors','construction_errors','lns_invalid_candidates','baseline_recovery','final_recovery']})
    rows=sets['late']['rows']
    observations['late']['phase_active_cases']={k:sum(r['trace'].get(k,0)>0 for r in rows) for k in a['activation']}
    summary=dict(accepted=a['accepted'],validation={k:v['metrics'] for k,v in sets.items()},differences=a['differences'],
                 activation=a['activation'],observations=observations,mechanism={k:v for k,v in a['mechanism'].items() if k!='reports'},
                 official_source=str(RUN.relative_to(ROOT)/'result.json'),completed_at=result['completed_at'])
    if 'final' in result:
        f=result['final'];assert sha(ROOT/f['candidate']['source'])==f['candidate']['source_sha256']
        summary['final']={k:v for k,v in f.items() if k!='tools_in'};summary['final']['tools_in']=f['tools_in']['metrics']
    else:
        summary['retained']=result['retained'];summary['tools_in_evaluated']=False
        # 採用条件を満たさない場合、同じ固定ソースの保存済み最終評価を使う。
        previous=ROOT/'results/nn_rank/v113/20261004_integrated_studio/final/result.json'
        stored=json.loads(previous.read_text());saved=stored['tools_in']
        assert stored['candidate']['source_sha256']==saved['source_sha256']==result['retained']['sha256']
        assert sha(ROOT/result['retained']['source'])==result['retained']['sha256']
        reference=json.loads((RUN/'in_reference.json').read_text())
        assert {r['filename'] for r in saved['rows']}==set(reference['values'])
        summary['retained_final']=dict(
            reused=True,source=str(previous.relative_to(ROOT)),completed_at=stored['completed_at'],
            tools_in=saved['metrics'],final_valid=stored['final_valid'],
            fixed_reference_relative=100*statistics.mean(reference['values'][r['filename']]/r['T'] for r in saved['rows']),
            reference_sha256=sha(RUN/'in_reference.json'),goal_190=saved['metrics']['mean_T_completed']<190)
    shared=ROOT/'adhoc/v125';shared.mkdir(parents=True,exist_ok=True);save(shared/'result_summary.json',summary);save(RUN/'summary.json',summary)
    with (shared/'validation_paired.csv').open('w',newline='') as stream:
        writer=csv.writer(stream,lineterminator='\n');writer.writerow(['case','v113_T','v115_T','v125_T','difference_v113','difference_v115','E','elapsed_ms','early_full','late_full','late_limited'])
        for r in rows:
            writer.writerow([r['case'],base[r['case']]['T'],full[r['case']]['T'],r['T'],r['T']-base[r['case']]['T'],r['T']-full[r['case']]['T'],r['E'],r['elapsed_ms']]+[r['trace'].get(k,0) for k in ['corridor_early_full','corridor_late_full','corridor_late_limited']])
    print(json.dumps(summary,ensure_ascii=False,indent=2))


if __name__=='__main__':main()
