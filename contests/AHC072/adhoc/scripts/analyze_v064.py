#!/usr/bin/env python3
"""完了済みv064評価を保存v059と比較する。探索や再評価は起動しない。"""
from pathlib import Path
from collections import Counter
import csv,hashlib,json,re,shutil,statistics
from replay_slime_output import replay
ROOT=Path(__file__).resolve().parents[2]
AUDIT=ROOT/'adhoc/v064_audit/20260930_submit'
PARENT_RUN='20260929T235217+0900_v059_repair_priority_6879ca'
PARENT_OUT=ROOT/'results/analysis/v059/20260929T235018_0cd76222/outputs'
LONG=set('0014 0015 0030 0034 0042 0047 0070 0071 0075 0081 0082 0084 0089 0092'.split())
NAME='v064_sparse_interval_lns'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,d):p.write_text(json.dumps(d,ensure_ascii=False,indent=2)+'\n')
def counters(p):
 text=p.read_text();return ({k:int(v) for k,v in re.findall(r'\[summary.count\] (\S+)=(-?\d+)',text)},
                          {k:float(v) for k,v in re.findall(r'\[summary.time_ms\] (\S+)=([\d.]+)',text)})

def main():
 records=[json.loads(line) for line in (ROOT/'results/eval_records.jsonl').read_text().splitlines() if line.strip()]
 parent={r['case_name']:r for r in records if r['run_id']==PARENT_RUN}
 current={r['case_name']:r for r in records if r['bin']==NAME}
 assert len(parent)==100 and len(current)==100
 assert len({r['run_id'] for r in current.values()})==1
 run=next(iter(current.values()))['run_id'];out=ROOT/'results/analysis/v064'/run;out.mkdir(parents=True,exist_ok=True)
 (out/'outputs').mkdir(exist_ok=True)
 manifest=json.loads((AUDIT/'manifest.json').read_text())
 for name,h in manifest['source_files'].items():
  snapshot=AUDIT/'snapshot'/Path(name).name
  assert digest(snapshot)==h,name
  if name=='notes/experiments/v064.md':
   # 評価後の記録追記を許し、事前登録の本文は凍結時と照合する。
   assert (ROOT/name).read_text().split('## 実験後',1)[0]==snapshot.read_text().split('## 実験後',1)[0],name
  else:assert digest(ROOT/name)==h,name
 assert json.loads((AUDIT/'mechanism.json').read_text())['status']=='passed'
 rows=[];counts=Counter();times=[];mechanism={};hashes={};errors=[];max_span=0;max_returned=0;parent_pre=0;current_pre=0
 for case,r in sorted(current.items()):
  p=parent[case];assert r['status']=='ok' and r['local'] and r['input_dir']=='tools/in'
  inp=ROOT/'tools/in'/case;source=ROOT/r['stdout_path'];stderr=source.with_suffix(source.suffix+'.err')
  assert digest(inp)==manifest['input_hashes'][case] and digest(PARENT_OUT/case)==manifest['parent_output_hashes'][case]
  for f in [source,stderr]:
   dest=out/'outputs'/f.name
   if dest.exists():assert digest(dest)==digest(f)
   else:shutil.copy2(f,dest)
  metrics=replay(inp,out/'outputs'/case)['metrics'];assert metrics['E']==0 and metrics['T']==r['score'] and metrics['T']<=100000
  c,t=counters(stderr);pc,pt=counters(PARENT_OUT/(case+'.err'))
  for key in ['construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery']:
   if c.get(key)!=0:errors.append(dict(case=case,key=key,value=c.get(key)))
  if c['board_pool_free_at_end']!=c['board_pool_slots']:errors.append(dict(case=case,key='board_pool'))
  assert c['final_ops']==r['score']
  small={k.removeprefix('sparse_interval_'):v for k,v in c.items() if k.startswith('sparse_interval_')}
  max_span=max(max_span,small['max_span']);max_returned=max(max_returned,small['max_returned_span'])
  counts.update(small);times.append(t['sparse_interval']);mechanism[case]=dict(counts=small,times_ms=t)
  parent_pre+=pc['pre_lns_ops'];current_pre+=c['pre_lns_ops']
  rows.append(dict(case=case[:-4],v059=p['score'],v064=r['score'],saved=p['score']-r['score'],elapsed_ms=r['elapsed'],long14=case[:-4] in LONG,
                   parent_pre_lns=pc['pre_lns_ops'],current_pre_lns=c['pre_lns_ops'],sparse_ms=t['sparse_interval'],**small))
  hashes[case]=dict(input=digest(inp),output=digest(source),stderr=digest(stderr))
 assert not errors,errors
 counts['max_span']=max_span;counts['max_returned_span']=max_returned
 total=sum(r['v064'] for r in rows);base=sum(r['v059'] for r in rows);saved=base-total
 long_saved=sum(r['saved'] for r in rows if r['long14']);maximum=max(r['elapsed_ms'] for r in rows)
 usual_gate=counts['accepted']>0 and counts['skipped_layers']>0
 valid_gate=maximum<=2000 and current['0000.txt']['score']<=43 and not errors
 adopted=valid_gate and usual_gate and saved>0 and long_saved>=0
 for filename in ['score_summary.csv','score_detail.csv']:
  matches=[r for r in csv.DictReader((ROOT/'results'/filename).open()) if r['bin']==NAME]
  assert len(matches)==1
  if filename=='score_summary.csv':assert int(matches[0]['total_sum'])==total
  else:assert all(int(matches[0][r['case']+'.txt'])==r['v064'] for r in rows)
 summary=dict(run_id=run,parent_run_id=PARENT_RUN,status='evaluated',adopted=adopted,
  total=total,parent_total=base,mean=total/100,parent_mean=base/100,saved=saved,long14_saved=long_saved,
  wins=sum(r['saved']>0 for r in rows),ties=sum(r['saved']==0 for r in rows),losses=sum(r['saved']<0 for r in rows),
  max_elapsed_ms=maximum,case0000=current['0000.txt']['score'],valid_cases=100,E=0,errors=errors,
  mechanism_active=usual_gate,mechanism_counts=dict(counts),mean_sparse_ms=statistics.mean(times),max_sparse_ms=max(times),
  completed_cases=sum(r['completed']>0 for r in rows),returned_cases=sum(r['returned']>0 for r in rows),
  direct_shortened_cases=sum(r['returned_saved']>0 for r in rows),accepted_cases=sum(r['accepted']>0 for r in rows),
  best_updated_cases=sum(r['best_updates']>0 for r in rows),long_completed_cases=sum(r['long_completed']>0 for r in rows),
  long_shortened_cases=sum(r['long_shorter']>0 for r in rows),
  pre_lns_added=current_pre-parent_pre,pre_lns_changed_cases=sum(r['current_pre_lns']!=r['parent_pre_lns'] for r in rows),
  top_gains=sorted(rows,key=lambda r:-r['saved'])[:5],top_losses=sorted(rows,key=lambda r:r['saved'])[:5],
  solver_executions_in_analysis=0)
 with (out/'cases.csv').open('w',newline='') as f:
  w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
 save(out/'summary.json',summary);save(out/'mechanism.json',mechanism);save(out/'hashes.json',hashes)
 save(ROOT/'results/analysis/v064/latest.json',dict(run_id=run,path=str(out)))
 print(json.dumps({k:v for k,v in summary.items() if k not in ['top_gains','top_losses']},ensure_ascii=False,indent=2))
 print('saved',out)
if __name__=='__main__':main()
