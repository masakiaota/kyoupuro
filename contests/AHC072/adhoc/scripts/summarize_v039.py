#!/usr/bin/env python3
"""Analyze saved v039 measurements and independently validate outputs."""
import csv
import json
import statistics
import sys
from prepare_v039 import ROOT,OUT,NAMES,digest
from run_v039 import unchanged,COUNTS
from check_v028_two_orders import validate


def comparison(rows,metric):
    totals={v:[sum(int(r[metric]) for r in rows if r['variant']==v and int(r['round'])==i) for i in range(1,6)] for v in ('v037','v039')}
    median={v:statistics.median(xs) for v,xs in totals.items()}
    changes=[100*(b/a-1) for a,b in zip(totals['v037'],totals['v039'])]
    delta=100*(median['v039']/median['v037']-1)
    return {'round_ns':totals,'median_ns':median,'change_percent':delta,'round_change_percent':changes,
        'faster_rounds':sum(x<0 for x in changes),'noninferior':delta<=1 and all(x<=1 for x in changes),
        'faster':delta<=-1 and sum(x<0 for x in changes)>=4}


def work():
    unchanged()
    rows=list(csv.DictReader((OUT/'fixed_work_samples.csv').open()))
    assert len(rows)==1200 and len({(r['case'],r['round'],r['variant']) for r in rows})==1200
    for case in {r['case'] for r in rows}:
        selected=[r for r in rows if r['case']==case]
        assert len({(r['candidates'],r['completed'],r['checksum']) for r in selected})==1
    normal=[r for r in rows if r['case']!='0000.txt']
    metrics=('build_ns','reconstruct_ns','smooth_ns','compress_ns','total_ns','total_wall_ns')
    result={'normal_99':{m:comparison(normal,m) for m in metrics},'all_100':{m:comparison(rows,m) for m in metrics},
        'cases':100,'candidate_sets':sum(int(r['candidates']) for r in rows if r['round']=='1' and r['variant']=='v037'),
        'normal_candidate_sets':sum(int(r['candidates']) for r in normal if r['round']=='1' and r['variant']=='v037'),
        'all_signatures_equal':True,'LOCAL':False,'clock':'unchanged steady_clock'}
    cases=[]
    for case in sorted({r['case'] for r in rows}):
        selected=[r for r in rows if r['case']==case]
        item={'case':case,'floors':int(selected[0]['floors'])}
        for metric in metrics:
            c=comparison(selected,metric)
            item.update({metric+'_parent':c['median_ns']['v037'],metric+'_child':c['median_ns']['v039'],metric+'_change_percent':c['change_percent']})
        cases.append(item)
    with (OUT/'case_timing.csv').open('w') as f:
        writer=csv.DictWriter(f,fieldnames=cases[0].keys());writer.writeheader();writer.writerows(cases)
    values=[c['total_ns_change_percent'] for c in cases if c['case']!='0000.txt']
    result['case_counts']={'faster_by_1_percent':sum(x<=-1 for x in values),'within_1_percent':sum(-1<x<=1 for x in values),'slower_over_1_percent':sum(x>1 for x in values)}
    result['floor_bands']={str(cap):comparison([r for r in normal if min(400,(int(r['floors'])+31)//32*32)==cap],'total_ns') for cap in sorted({min(400,(int(r['floors'])+31)//32*32) for r in normal})}
    copies=list(csv.DictReader((OUT/'copy_samples.csv').open()));assert len(copies)==2400
    for case in {r['case'] for r in copies}:
        for task in ('reuse_copy','create_copy_destroy'):
            assert len({r['checksum'] for r in copies if r['case']==case and r['task']==task})==1
    result['copies']={t:comparison([r for r in copies if r['task']==t and r['case']!='0000.txt'],'cpu_ns') for t in ('reuse_copy','create_copy_destroy')}
    sizes=list(csv.DictReader((OUT/'sizes.csv').open()));assert len(sizes)==100
    result['memory']={k:statistics.mean(int(r[k]) for r in sizes if r['case']!='0000.txt') for k in ('floors','parent_payload','child_payload','child_handle','child_pool_payload','child_pool_metadata')}
    result['memory']['payload_change_percent']=100*(result['memory']['child_payload']/1600-1)
    result['memory']['four_live_with_metadata_change_percent']=100*((result['memory']['child_pool_payload']+4*result['memory']['child_handle']+result['memory']['child_pool_metadata'])/6400-1)
    (OUT/'fixed_work_summary.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:result[k] for k in ('normal_99','memory','copies','candidate_sets','case_counts')},indent=2))


def logs(p):
    text=p.read_text()
    return {k:int(v) for k,v in COUNTS.findall(text)}, {k:float(v) for k,v in __import__('re').findall(r'\[summary.time_ms\] ([^=]+)=([\d.]+)',text)}, [x for x in text.splitlines() if 'diagnostic:' in x]


def evaluation():
    unchanged()
    records=[json.loads(x) for x in (ROOT/'results/eval_records.jsonl').read_text().splitlines()]
    rows=[r for r in records if r['bin']==NAMES['child']]
    parent={r['case_name']:r for r in records if r['run_id']=='20260928T020458+0900_v037_pair_transfer_compression_6086dd'}
    assert len(rows)==len(parent)==100 and len({r['run_id'] for r in rows})==1
    errors=('baseline_recovery','final_recovery','construction_errors','lns_errors','lns_invalid_candidates')
    cases=[]
    for r in sorted(rows,key=lambda r:r['case_name']):
        assert r['status']=='ok' and r['local'] and r['input_dir']=='tools/in'
        path=ROOT/r['stdout_path'];moves=[]
        for line in path.read_text().splitlines():
            a,b,c,d,e=line.split();moves.append((int(a),int(b),int(c),d,int(e)))
        validate(r['case_name'],moves)
        c,t,diag=logs(path.with_suffix('.txt.err'))
        pc,_,_=logs((ROOT/parent[r['case_name']]['stdout_path']).with_suffix('.txt.err'))
        text=(ROOT/'tools/in'/r['case_name']).read_text().splitlines();n=int(text[0].split()[0]);floors=sum(ch!='#' for line in text[1:n+1] for ch in line)
        assert c['board_payload_bytes']==4*floors and c['floor_cells']==floors
        assert c['board_handle_bytes']==8 and c['board_pool_slots']==c['board_pool_free_at_end']==4
        assert c['board_pool_bytes']==16*floors
        assert c['E']==0 and len(moves)==r['score']==c['T']==c['final_ops']
        assert t['search_limit']==1544.0 and c['pre_pair_ops']-c['pair_transfer_saved']==r['score']
        cases.append({'case':r['case_name'],'parent_T':parent[r['case_name']]['score'],'T':r['score'],
            'delta_T':r['score']-parent[r['case_name']]['score'],'elapsed_ms':r['elapsed'],'counts':c,'parent_counts':pc,'diagnostics':diag})
    total=sum(c['T'] for c in cases);base=sum(c['parent_T'] for c in cases)
    result={'run_id':rows[0]['run_id'],'parent_run_id':next(iter(parent.values()))['run_id'],'verified_cases':100,
        'total_T':total,'parent_T':base,'delta_T':total-base,'delta_percent':100*(total/base-1),
        'wins':sum(c['delta_T']<0 for c in cases),'ties':sum(c['delta_T']==0 for c in cases),'losses':sum(c['delta_T']>0 for c in cases),
        'case0000_T':cases[0]['T'],'max_elapsed_ms':max(c['elapsed_ms'] for c in cases),'average_elapsed_ms':statistics.mean(c['elapsed_ms'] for c in cases),
        'errors':{k:sum(c['counts'][k] for c in cases) for k in errors},'diagnostics_count':sum(len(c['diagnostics']) for c in cases),
        'pre_lns_delta':sum(c['counts']['pre_lns_ops']-c['parent_counts']['pre_lns_ops'] for c in cases),
        'pre_lns_different_cases':sum(c['counts']['pre_lns_ops']!=c['parent_counts']['pre_lns_ops'] for c in cases),
        'counts':{k:sum(c['counts'].get(k,0) for c in cases) for k in cases[0]['counts']},'behavior_change_percent':{}}
    for key in ('lns_attempts','single_insert_calls','packet_insert_calls','event_layers'):
        before=sum(c['parent_counts'].get(key,0) for c in cases[1:]);after=sum(c['counts'].get(key,0) for c in cases[1:])
        result['behavior_change_percent'][key]=100*(after/before-1)
    result['required_passed']=result['max_elapsed_ms']<2000 and result['case0000_T']<=43 and not any(result['errors'].values()) and result['diagnostics_count']==0
    result['score_noninferior']=result['delta_percent']<=0.3
    work=json.loads((OUT/'fixed_work_summary.json').read_text())
    result['speed_noninferior']=work['normal_99']['total_ns']['noninferior']
    result['adopt']=result['required_passed'] and result['score_noninferior'] and result['speed_noninferior']
    result['local_binary_matches_frozen']=digest(ROOT/'target/release'/NAMES['child'])==digest(OUT/'child_local')
    (OUT/'evaluation_summary.json').write_text(json.dumps({**result,'cases':cases},indent=2)+'\n')
    with (OUT/'evaluation_cases.csv').open('w') as f:
        writer=csv.writer(f);writer.writerow(('case','parent_T','T','delta_T','elapsed_ms'))
        for c in cases:writer.writerow(c[k] for k in ('case','parent_T','T','delta_T','elapsed_ms'))
    print(json.dumps({k:v for k,v in result.items() if k!='counts'},indent=2))


if __name__=='__main__':
    {'work':work,'eval':evaluation}[sys.argv[1]]()
