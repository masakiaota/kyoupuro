#!/usr/bin/env python3
"""保存済みv057/v059/v062の評価を照合・集計する。solverを実行しない。"""
from __future__ import annotations
from collections import Counter
import csv
import hashlib
import json
from pathlib import Path
import re
import shutil
import statistics

from replay_slime_output import replay

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'results/analysis/v059_v062/20260930'
BATCH = '20260929T235018_0cd76222'
BINS = dict(v057='v057_search_reductions', v059='v059_repair_priority', v062='v062_deferred_passenger', v800='v800')
RUNS = dict(v057='20260929T145653+0900_v057_search_reductions_af817c',
            v059='20260929T235217+0900_v059_repair_priority_6879ca',
            v062='20260930T000654+0900_v062_deferred_passenger_6821e0')
LONG6 = {'0014', '0015', '0030', '0034', '0042', '0047'}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n')


def save_csv(path, rows):
    with path.open('w', newline='') as f:
        w=csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)


def counters(path):
    s=path.read_text()
    c={k:int(v) for k,v in re.findall(r'\[summary.count\] (\S+)=(-?\d+)',s)}
    t={k:float(v) for k,v in re.findall(r'\[summary.time_ms\] (\S+)=([\d.]+)',s)}
    return c,t,s


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    all_records=[json.loads(line) for line in (ROOT/'results/eval_records.jsonl').read_text().splitlines() if line.strip()]
    records={}
    for v,b in BINS.items():
        matches=[r for r in all_records if r['bin']==b and (v not in RUNS or r['run_id']==RUNS[v])]
        assert len(matches)==100 and len({r['run_id'] for r in matches})==1,(v,len(matches))
        assert {r['case_name'] for r in matches}=={f'{i:04d}.txt' for i in range(100)}
        assert all(r['status']=='ok' for r in matches)
        records[v]={Path(r['case_name']).stem:r for r in matches}
    original_summary=list(csv.DictReader((ROOT/'results/score_summary.csv').open()))
    original_detail=list(csv.DictReader((ROOT/'results/score_detail.csv').open()))
    for v in BINS:
        r0=records[v]['0000']
        matches=[r for r in original_summary if r['bin']==BINS[v] and r['executed_at']==r0['executed_at']]
        assert len(matches)==1 and int(matches[0]['total_sum'])==sum(r['score'] for r in records[v].values())
        details=[r for r in original_detail if r['bin']==BINS[v] and r['executed_at']==r0['executed_at']]
        assert len(details)==1
        assert all(int(details[0][r['case_name']])==r['score'] for r in records[v].values())
    v800_manifest=json.loads((ROOT/'results/long_search/v800/20260929T150407_c945d3d4/manifest.json').read_text())
    inputs={c['case']:c['features'] | dict(cohort='reused_45min' if c['reused'] else 'handcrafted' if c['case']=='0000' else 'fresh_5min',
                                         initial_T=c['reference_T']) for c in v800_manifest['cases']}
    assert len(inputs)==100
    # The previous case table contains only the 90 newly searched cases. Join
    # all 100 explicitly from the run manifest, including the 10 reused cases.
    for r in csv.DictReader((ROOT/'results/analysis/v800/20260929_all_100/cases.csv').open()):
        f=inputs[r['case']]
        assert int(r['initial_T'])==f['initial_T'] and int(r['M'])==f['M'] and r['cohort']==f['cohort']
    batch=ROOT/'results/experiment_batches/v059_v063'/BATCH
    manifest=json.loads((batch/'manifest.json').read_text())
    data={}; hashes={}; errors=[]
    for v in ('v057','v059','v062'):
        data[v]={}
        for case,r in records[v].items():
            inp=ROOT/'tools/in'/r['case_name']; path=ROOT/r['stdout_path']; err=path.with_suffix(path.suffix+'.err')
            assert r['local'] and r['input_dir']=='tools/in'
            assert digest(inp)==manifest['data_files']['tools/in/'+r['case_name']]
            if v=='v057': assert digest(path)==manifest['data_files'][r['stdout_path']]
            if v=='v059':
                saved=ROOT/'results/analysis/v059'/BATCH/'outputs'/path.name
                assert digest(path)==digest(saved) and digest(err)==digest(saved.with_suffix(saved.suffix+'.err'))
            if v=='v062':
                saved=ROOT/'results/analysis/v062/20260930_manual_review/outputs'/path.name
                saved.parent.mkdir(parents=True, exist_ok=True)
                for source,target in ((path,saved),(err,saved.with_suffix(saved.suffix+'.err'))):
                    if target.exists(): assert digest(source)==digest(target)
                    else: shutil.copy2(source,target)
                path=saved;err=saved.with_suffix(saved.suffix+'.err')
            checked=replay(inp,path)['metrics']
            assert checked['T']==r['score'] and checked['E']==0 and checked['T']<=100000
            c,t,text=counters(err)
            for key in ('construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery'):
                if c.get(key)!=0: errors.append(dict(version=v,case=case,key=key,value=c.get(key)))
            assert c['board_pool_free_at_end']==c['board_pool_slots'] and 'diagnostic:' not in text
            assert c['final_ops']==r['score']
            data[v][case]=dict(counts=c,times=t)
            hashes[f'{v}/{case}']=dict(input=digest(inp),stdout=digest(path),stderr=digest(err),stdout_path=str(path))
    assert not errors,errors
    rows=[]
    for case in sorted(records['v057']):
        f=inputs[case]
        row=dict(case=case,M=int(f['M']),K=int(f['K']),wall_fraction=float(f['wall_fraction']),cohort=f['cohort'],
                 initial_v800_T=int(f['initial_T']),long6=case in LONG6)
        for v in BINS: row[v]=records[v][case]['score']
        for v in ('v059','v062'): row[v+'_saved']=row['v057']-row[v]
        row['v059_gap_to_v800']=row['v059']-row['v800'];row['v062_minus_v059']=row['v062']-row['v059']
        rows.append(row)
    save_csv(OUT/'cases.csv',rows)
    def summarise(selected):
        result={}
        for v in ('v057','v059','v062'):
            cases=[r['case'] for r in selected]
            c=Counter();t=Counter()
            for case in cases: c.update(data[v][case]['counts']);t.update(data[v][case]['times'])
            n=len(cases)
            result[v]=dict(n=n,total=sum(r[v] for r in selected),mean=sum(r[v] for r in selected)/n,
                           saved=sum(r['v057']-r[v] for r in selected),wins=sum(r[v]<r['v057'] for r in selected),
                           ties=sum(r[v]==r['v057'] for r in selected),losses=sum(r[v]>r['v057'] for r in selected),
                           median_saved=statistics.median(r['v057']-r[v] for r in selected),counts=dict(c),
                           mean_times_ms={k:z/n for k,z in t.items()},max_elapsed_ms=max(records[v][p]['elapsed'] for p in cases),
                           dependency_completion_rate=c['lns_dependency_completed']/c['lns_dependency_attempts'] if c['lns_dependency_attempts'] else None,
                           dependency_repair_fraction=1-c['lns_dependency_net_removed']/c['lns_dependency_gross_removed'] if c['lns_dependency_gross_removed'] else None)
        return result
    groups=dict(all=rows,ordinary=[r for r in rows if r['case']!='0000'],registered_long6=[r for r in rows if r['long6']],
                v800_fresh_long14=[r for r in rows if r['cohort']=='fresh_5min' and r['initial_v800_T']>=300],
                v057_under150=[r for r in rows if r['case']!='0000' and r['v057']<150],
                v057_150_to299=[r for r in rows if 150<=r['v057']<300],v057_300plus=[r for r in rows if r['v057']>=300],
                M_under50=[r for r in rows if r['case']!='0000' and r['M']<50],M_50_to99=[r for r in rows if 50<=r['M']<100],M_100plus=[r for r in rows if r['M']>=100],
                wall_under10=[r for r in rows if r['case']!='0000' and r['wall_fraction']<.10],
                wall_10_to25=[r for r in rows if r['case']!='0000' and .10<=r['wall_fraction']<.25],
                wall_25plus=[r for r in rows if r['case']!='0000' and r['wall_fraction']>=.25])
    sums={name:summarise(rr) for name,rr in groups.items() if rr}
    save_json(OUT/'groups.json',sums)
    save_json(OUT/'mechanism_by_case.json',data)
    diag=json.loads((ROOT/'adhoc/v059_audit'/BATCH/'diagnostic_verification.json').read_text())
    diagnostic={}
    for label,plans in (('all',diag['plans']),('long6',[r for r in diag['plans'] if r['case'] in LONG6])):
        result=dict(plans=len(plans),gate_plans=sum(r['gate'] for r in plans))
        for mode,rank in [('old','old_rank'),('new','new_rank')]:
            top=[]
            for plan in plans:
                file=ROOT/'adhoc/v059_audit'/BATCH/plan['case']/f'plan_{plan["plan_index"]:02d}'/'candidates.csv'
                top += [r for r in csv.DictReader(file.open()) if int(r[rank])<32]
            extracted=[r for r in top if r['extract_status']=='ok']
            gross=sum(int(r['gross_removed']) for r in extracted);net=sum(int(r['net_removed']) for r in extracted)
            result[mode]=dict(candidates=len(top),extracted=len(extracted),inserted=sum(r['insert_status']=='ok' for r in top),
                             extraction_rate=len(extracted)/len(top),completion_rate=sum(r['insert_status']=='ok' for r in top)/len(top),
                             mean_net_removed=net/len(extracted),gross=gross,net=net,repair_fraction=(gross-net)/gross,
                             mean_broken=sum(int(r['broken_support_jumps']) for r in top)/len(top))
        diagnostic[label]=result
    diagnostic['eligible_long_cases']=diag['eligible_long_cases']
    diagnostic['deadlines']=sum(r['diagnostic']['deadlines'] for r in diag['plans'])
    diagnostic['rng_calls']=sum(r['diagnostic']['additional_rng_calls'] for r in diag['plans'])
    save_json(OUT/'v059_diagnostic.json',diagnostic)
    deferred=[]
    for row in rows:
        d=data['v062'][row['case']]
        deferred.append(row | {k.removeprefix('deferred_passenger_'):v for k,v in d['counts'].items() if k.startswith('deferred_passenger_')} |
                        dict(ms=d['times']['deferred_passenger']))
    save_csv(OUT/'v062_deferred.csv',deferred)
    summary=dict(runs={v:records[v]['0000']['run_id'] for v in BINS},solver_executions=0,replayed_outputs=300,errors=errors,
                 totals={v:sum(r[v] for r in rows) for v in BINS},
                 v059_top_gains=sorted(rows,key=lambda r:(-r['v059_saved'],r['case']))[:8],v059_top_losses=sorted(rows,key=lambda r:(r['v059_saved'],r['case']))[:8],
                 v062_top_gains=sorted(rows,key=lambda r:(-r['v062_saved'],r['case']))[:8],v062_top_losses=sorted(rows,key=lambda r:(r['v062_saved'],r['case']))[:8],
                 v062_direct=dict(active_cases=sum(r['saved']>0 for r in deferred),accepted_cases=sum(r['accepted_saved']>0 for r in deferred),
                     best_created_cases=sum(r['best_created']>0 for r in deferred),max_span=max(r['max_span'] for r in deferred),
                     spans_over64_cases=sum(r['max_span']>64 for r in deferred),spans_over64_ids=[r['case'] for r in deferred if r['max_span']>64],
                     saved=sum(r['saved'] for r in deferred),accepted_saved=sum(r['accepted_saved'] for r in deferred),best_created=sum(r['best_created'] for r in deferred),
                     mean_ms=statistics.mean(r['ms'] for r in deferred),max_ms=max(r['ms'] for r in deferred)),
                 pre_lns_changed={v:sum(data[v][r['case']]['counts']['pre_lns_ops']!=data['v057'][r['case']]['counts']['pre_lns_ops'] for r in rows) for v in ('v059','v062')},
                 v059_vs_v062=dict(wins=sum(r['v059']<r['v062'] for r in rows),ties=sum(r['v059']==r['v062'] for r in rows),losses=sum(r['v059']>r['v062'] for r in rows)),
                 remaining_to_mean190=sum(r['v059'] for r in rows)-19000,
                 v059_top_reference_gaps=sorted(rows,key=lambda r:(-r['v059_gap_to_v800'],r['case']))[:10])
    save_json(OUT/'summary.json',summary)
    save_json(OUT/'input_output_hashes.json',hashes)
    save_json(ROOT/'results/analysis/v062/20260930_manual_review/verification.json',dict(run_id=RUNS['v062'],valid_cases=100,E=0,errors=errors,analysis=str(OUT),solver_executions=0))
    for v,z in sums['all'].items():
        print(v,{k:z[k] for k in ('mean','saved','wins','ties','losses','median_saved','max_elapsed_ms','dependency_completion_rate','dependency_repair_fraction')})
        print('mechanism', {k:z['counts'][k] for k in z['counts'] if k.startswith(('deferred_passenger_','lns_repair_priority_'))})
        print('time', {k:z['mean_times_ms'][k] for k in ('lns_candidate_build','temporal_lns','lns_repair_priority','deferred_passenger') if k in z['mean_times_ms']})
        print('attempts',z['counts']['lns_attempts'])
    print('v059 diagnostic',json.dumps(diagnostic))
    print('v062 direct',json.dumps(summary['v062_direct']))
    for name,z in sums.items():
        print(name,[(v,zz['n'],zz['saved'],zz['dependency_completion_rate'],zz['dependency_repair_fraction']) for v,zz in z.items()])
    print('SUMMARY',OUT/'summary.json')


if __name__=='__main__':main()
