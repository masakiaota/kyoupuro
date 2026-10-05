#!/usr/bin/env python3
from pathlib import Path
import json,hashlib,re,collections,statistics,sys,csv,shutil
root=Path(__file__).resolve().parents[2]; out=root/'results/analysis/v402';sys.path.insert(0,str(root/'adhoc/scripts'))
from replay_slime_output import replay
label='cloud_v402_gcc15.3_LOCAL_search1.900_core1.870466_j8_20261004'
records=[json.loads(s) for s in (root/'results/eval_records.jsonl').read_text().splitlines()];records=[r for r in records if r['label']==label]
assert len(records)==100 and len({r['case_name'] for r in records})==100
ref=json.loads((root/'results/analysis/cloud_baseline/fixed_reference.json').read_text())['cases'];assert {r['case_name'] for r in records}==set(ref)
parent={r['case_name']:r['score'] for r in json.loads((root/'results/analysis/v401/records.json').read_text())}
traces=[];replays=[];pairs=[];repairs=[]
for r in records:
 p=root/r['stdout_path'];t=Path(str(p)+'.err').read_text();case=r['case_name']; inp=root/'tools/in'/case
 assert hashlib.sha256(inp.read_bytes()).hexdigest()==ref[case]['input_sha256']
 counts={k:int(v) for k,v in re.findall(r'\[summary.count\] ([^=]+)=(-?\d+)',t)}; times={k:float(v) for k,v in re.findall(r'\[summary.time_ms\] ([^=]+)=(-?[\d.]+)',t)}
 traces.append({'case':case,'counts':counts,'times_ms':times,'output_sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
 rep=replay(inp,p)['metrics']; assert rep['S']==r['score'] and rep['E']==0;replays.append({'case':case,**rep})
 pairs.append({'case':case,'v311':ref[case]['score'],'v401':parent[case],'v402':r['score'],'delta':r['score']-parent[case],'index':100*ref[case]['score']/r['score'],'elapsed_ms':r['elapsed']})
 for kind,seed,T,plan in re.findall(r'\[initial_repair\.(raw|accepted)\] (\d+) (\d+)\n(.*?)\[initial_repair.end\]',t,re.S):
  target=out/'repair_outputs'/f'{inp.stem}_{seed}_{kind}.txt';target.parent.mkdir(exist_ok=True);target.write_text(plan)
  metrics=replay(inp,target)['metrics'];assert metrics['E']==0 and metrics['T']==int(T)
  repairs.append({'case':case,'seed':int(seed),'kind':kind,**metrics})
 assert counts.get('initial_repair_calls',0)<=counts['initial_repair_raw_seeds']
 assert counts.get('initial_repair_scored',0)<=32*counts.get('initial_repair_calls',0)
scores=[r['score'] for r in records if r['status']=='ok'];elapsed=[r['elapsed'] for r in records]
result={'label':label,'status_counts':dict(collections.Counter(r['status'] for r in records)),'cases':len(records),'score_sum':sum(scores),'score_mean':statistics.mean(scores),'score_min':min(scores),'score_max':max(scores),'score_delta':sum(p['delta'] for p in pairs),'score_percent_change':100*sum(p['delta'] for p in pairs)/sum(p['v401'] for p in pairs),'fixed_reference_index':statistics.mean(p['index'] for p in pairs),'wins':sum(p['delta']<0 for p in pairs),'ties':sum(p['delta']==0 for p in pairs),'losses':sum(p['delta']>0 for p in pairs),'elapsed_mean_ms':statistics.mean(elapsed),'elapsed_max_ms':max(elapsed),'elapsed_over_1900ms':sum(e>1900 for e in elapsed),'elapsed_over_2000ms':sum(e>2000 for e in elapsed),'E_distribution':dict(collections.Counter(t['counts'].get('E','missing') for t in traces)),'search_limit_distribution_ms':dict(collections.Counter(t['times_ms'].get('search_limit','missing') for t in traces)),'error_recovery_totals':{key:sum(t['counts'].get(key,0) for t in traces) for key in ['construction_errors','lns_errors','baseline_recovery','final_recovery']},'free_slots_distribution':dict(collections.Counter(t['counts'].get('state_pool_free_at_end','missing') for t in traces)),'trace_total_max_ms':max(t['times_ms'].get('total',0) for t in traces),'cnn_counts':{k:sum(t['counts'].get(k,0) for t in traces) for k in traces[0]['counts'] if k.startswith('cnn_')},'independent_replay_ok':len(replays),'memory':json.loads((out/'memory.json').read_text())}
result['initial_repair_counts']={k:sum(t['counts'].get(k,0) for t in traces) for k in ['initial_repair_raw_seeds','initial_repair_calls','initial_repair_scored','initial_repair_completed','initial_repair_accepted','initial_repair_saved','initial_repair_deadlines']}
result['initial_repair_ms_mean']=statistics.mean(t['times_ms'].get('initial_repair',0) for t in traces)
result['initial_repair_ms_max']=max(t['times_ms'].get('initial_repair',0) for t in traces)
result['raw_replay_count']=sum(x['kind']=='raw' for x in repairs)
result['accepted_replay_count']=sum(x['kind']=='accepted' for x in repairs)
result['acceptance_passed']=result['score_sum']<19876 and result['fixed_reference_index']>100.304115 and result['elapsed_max_ms']<2000 and len(replays)==100 and all(v==0 for v in result['error_recovery_totals'].values()) and result['free_slots_distribution']=={4:100}
for name,data in [('repair_replay',repairs),('result',result),('case_traces',traces),('records',records),('replay',replays)]: (out/f'{name}.json').write_text(json.dumps(data,indent=2))
with (out/'paired.csv').open('w') as f:w=csv.DictWriter(f,fieldnames=list(pairs[0]));w.writeheader();w.writerows(pairs)
shutil.copytree(root/'results/out/v402_initial_nn_repair',out/'outputs',dirs_exist_ok=True)
print(json.dumps({k:v for k,v in result.items() if k!='memory'},indent=2));print('memory_max_kib',result['memory']['max_observed_kib'])
