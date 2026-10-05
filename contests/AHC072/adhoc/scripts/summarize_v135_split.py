#!/usr/bin/env python3
"""保存したv135の比較と機構を集計する。solverは実行しない。"""
from pathlib import Path
import csv,json,statistics
import numpy as np
from build_v135_split import ROOT,RUN,BASE,CURRENT,SOURCE
from v089_data import save,sha

def load(p):return json.loads(p.read_text())

def main():
    data=load(RUN/'result.json');assert load(RUN/'pipeline/exit.json')['exit_code']==0
    cases=data['validation'];summary=dict(assessment=data['assessment'],completed_at=data['completed_at'],
        validation={k:v['metrics'] for k,v in cases.items()},source_sha256=sha(SOURCE),parent_sha256=sha(BASE),current_sha256=sha(CURRENT))
    base={r['case']:r for r in cases['v113']['rows']};diagnostics={}
    for label,result in cases.items():
        rows=result['rows'];d=dict(
            nn_first_hash_matches=sum(r['trace'].get('nn_initial_hash')==base[r['case']]['trace'].get('nn_initial_hash') for r in rows),
            all_pool_returned=all(r['trace'].get('state_pool_free_at_end')==4 for r in rows),
            errors={k:sum(r['trace'].get(k,0) for r in rows) for k in ('lns_errors','construction_errors','lns_invalid_candidates','baseline_recovery','final_recovery')},
            mean_lns_attempts=statistics.mean(r['trace'].get('lns_attempts',0) for r in rows))
        if label!='v113':
            keys=sorted({k for r in rows for k in r['construction'] if k.startswith(('v322_','v135_'))})
            d['rewrite']={k:(max(r['construction'].get(k,0) for r in rows) if k=='v322_max_span' else sum(r['construction'].get(k,0) for r in rows)) for k in keys}
            d['rewrite_completed_inputs']=sum(r['trace'].get('sparse_rewrite_completed',0)>0 for r in rows)
            d['split_completed_inputs']=sum(r['trace'].get('split_rewrite_completed',0)>0 for r in rows)
            d['mean_rewrite_ms']=1000*sum(r['construction'].get('v322_seconds',0) for r in rows)/len(rows)
        diagnostics[label]=d
    summary['diagnostics']=diagnostics
    rng=np.random.default_rng(135006);intervals={}
    target=cases['v135']['rows']
    for label in ('v113','v322'):
        old={r['case']:r for r in cases[label]['rows']}
        differences=np.array([r['S']-old[r['case']]['S'] for r in target])
        means=rng.choice(differences,size=(4096,len(differences)),replace=True).mean(axis=1)
        intervals[label]=np.quantile(means,[.025,.975]).tolist()
    summary['input_paired_bootstrap_95']=intervals
    summary['uncertainty_note']='Input variation only; same-condition solver runs were not repeated.'
    mechanism=load(RUN/'mechanism/result.json')
    comparisons=[]
    for record in mechanism['rows']:
        i=record['case'];old={r['first']:r for r in [json.loads(l) for l in (RUN/'mechanism'/f'{i}_base.jsonl').read_text().splitlines()]}
        new=[json.loads(l) for l in (RUN/'mechanism'/f'{i}_split.jsonl').read_text().splitlines()]
        comparisons.extend((old[r['first']],r) for r in new)
    summary['mechanism']=dict(all_legal=mechanism['all_legal'],preprocessing_checked=mechanism['preprocessing_checked'],
        fixture_paths=len(mechanism['fixture_rows']),fixture_max_span=max(r['span'] for r in mechanism['fixture_rows']),
        atomic_root_checks=mechanism['atomic_root_checks'],sequence_step_checks=mechanism['sequence_step_checks'],
        saved_plan_rows=mechanism['rows'],gained=sum(not a['ok'] and b['ok'] for a,b in comparisons),
        lost=sum(a['ok'] and not b['ok'] for a,b in comparisons))
    if 'final' in data:
        f=data['final'];summary['final']={k:v for k,v in f.items() if k!='tools_in'}
        summary['final']['tools_in']=f['tools_in']['metrics']
    else:
        summary['retained']=data['retained']
        previous=ROOT/'results/nn_rank/v113/20261004_integrated_studio/final/result.json';p=load(previous)
        assert p['candidate']['source_sha256']==sha(CURRENT)
        summary['retained_final']=dict(reused=True,source=str(previous.relative_to(ROOT)),tools_in=p['tools_in']['metrics'],final_valid=p['final_valid'])
    shared=ROOT/'adhoc/v135';shared.mkdir(exist_ok=True)
    save(shared/'result_summary.json',summary);save(RUN/'summary.json',summary)
    parent={r['case']:r for r in cases['v322']['rows']}
    with (shared/'validation_paired.csv').open('w',newline='') as f:
        writer=csv.writer(f,lineterminator='\n');writer.writerow(['case','v113_T','v322_T','v135_T','delta_v113','delta_v322','E','elapsed_ms','split_completed','split_saved'])
        for r in target:
            i=r['case'];writer.writerow([i,base[i]['T'],parent[i]['T'],r['T'],r['T']-base[i]['T'],r['T']-parent[i]['T'],r['E'],r['elapsed_ms'],r['trace'].get('split_rewrite_completed',0),r['trace'].get('split_rewrite_saved',0)])
    print(json.dumps(summary,ensure_ascii=False,indent=2))

if __name__=='__main__':main()
