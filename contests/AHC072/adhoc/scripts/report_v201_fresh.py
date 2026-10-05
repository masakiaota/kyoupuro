#!/usr/bin/env python3
"""v201の独立200件と既知100件を保存出力だけで判定する。"""
from pathlib import Path
import csv,json,random,statistics,urllib.request
from v201_fresh import ROOT,WORK,PARENTS,BIN,read,save,sha,unchanged
from analyze_v201_fresh import predict,features,relative,FEATURES,LABELS

def interval(values):
 values=sorted(values);return [values[int(.025*(len(values)-1))],values[int(.975*(len(values)-1))]]

def main():
 unchanged(); selection=read(WORK/'selection.json'); frozen=read(WORK/'solver_frozen.json')
 assert sha(ROOT/frozen['source'])==frozen['source_sha256'] and sha(WORK/'selection.json')==frozen['selection_sha256']
 manifest=[r for r in read(WORK/'manifest.json') if r['role']=='holdout200']
 datasets={b:{r['case_name']:r for r in read(WORK/'runs'/f'v201_fresh_holdout200_{b.split("_")[0]}'/'cases.json')} for b in [*PARENTS,BIN]}
 rows=[]
 for row in manifest:
  f=features(ROOT/row['path']); enabled=predict(selection['model'],f); current=datasets[BIN][row['case']]
  assert current['counts']['selector_nn_enabled']==enabled
  assert (current['counts']['nn_calls']>0)==bool(enabled)
  assert current['counts']['selector_M']==f['M']
  scores={b:datasets[b][row['case']]['score'] for b in datasets};base=min(scores.values())
  rows.append(dict(**row,features=f,scores=scores,relative={b:relative(base,t) for b,t in scores.items()},use_nn=enabled))
 summary={b:dict(total=sum(r['scores'][b] for r in rows),mean=statistics.mean(r['scores'][b] for r in rows),
                relative_100=statistics.mean(r['relative'][b] for r in rows),max_elapsed_ms=max(r['elapsed'] for r in datasets[b].values())) for b in datasets}
 comparisons={}
 for b in PARENTS:
  delta=[r['scores'][BIN]-r['scores'][b] for r in rows]; rel=[r['relative'][BIN]-r['relative'][b] for r in rows]
  rng=random.Random(20120261002); boot_T=[];boot_R=[]
  for _ in range(10000):
   sample=[rng.randrange(200) for _ in range(200)]
   boot_T.append(sum(delta[i] for i in sample)/200);boot_R.append(sum(rel[i] for i in sample)/200)
  comparisons[b]=dict(total_delta=sum(delta),mean_delta=statistics.mean(delta),relative_delta=statistics.mean(rel),
   wins=sum(t<0 for t in delta),ties=sum(t==0 for t in delta),losses=sum(t>0 for t in delta),
   mean_delta_ci95=interval(boot_T),relative_delta_ci95=interval(boot_R))
 policy_scores=[r['scores'][PARENTS[r['use_nn']]] for r in rows]
 policy_relative=[relative(min(r['scores'].values()),t) for r,t in zip(rows,policy_scores)]
 policy=dict(total=sum(policy_scores),mean=statistics.mean(policy_scores),relative_100=statistics.mean(policy_relative),
  actual_minus_selected_parent=sum(r['scores'][BIN]-t for r,t in zip(rows,policy_scores)))
 groups=[]
 for enabled in [0,1]:
  group=[r for r in rows if r['use_nn']==enabled]
  groups.append(dict(use_nn=enabled,cases=len(group),M_range=[min(r['features']['M'] for r in group),max(r['features']['M'] for r in group)] if group else [],
   deltas={b:sum(r['scores'][BIN]-r['scores'][b] for r in group) for b in PARENTS},
   nn_calls=sum(datasets[BIN][r['case']]['counts']['nn_calls'] for r in group)))
 with urllib.request.urlopen('http://127.0.0.1:5173/api/eval-view-data') as f:viewer=json.load(f)
 for b,data in datasets.items():
  rid=next(iter(data.values()))['run_id'];match=[r for runs in viewer['runsByEvalSet'].values() for r in runs if r['id']==rid]
  assert len(match)==1 and match[0]['caseScores']=={case:r['score'] for case,r in data.items()}
  assert abs(match[0]['relativeAvg']/10**7-summary[b]['relative_100'])<1e-7
 current100=read(WORK/'runs'/'v201_fresh_tools_in100_v201'/'cases.json')
 current_run=current100[0]['run_id']; registered=next(r for r in viewer['runsByEvalSet']['tools/in'] if r['id']==current_run)
 assert registered['caseScores']=={r['case_name']:r['score'] for r in current100}
 base_runs=['20260930T202705+0900_v076_relative_tuned_e688d5','20261002T190030+0900_v079_nn_immediate_d63947']
 hundred={BIN:dict(mean=registered['totalAvg'],relative_100=registered['relativeAvg']/10**7,max_elapsed_ms=registered['maxElapsed'],run_id=current_run)}
 for b,rid in zip(PARENTS,base_runs):
  run=next(r for r in viewer['runsByEvalSet']['tools/in'] if r['id']==rid)
  hundred[b]=dict(mean=run['totalAvg'],relative_100=run['relativeAvg']/10**7,max_elapsed_ms=run['maxElapsed'],run_id=rid)
 # 登録用100件でも全分岐が固定規則と一致することを照合する。
 for r in current100:
  expected=predict(selection['model'],features(ROOT/'tools/in'/r['case_name']))
  assert r['counts']['selector_nn_enabled']==expected and (r['counts']['nn_calls']>0)==bool(expected)
 adopted=all(summary[BIN]['total']<summary[b]['total'] and summary[BIN]['relative_100']>=summary[b]['relative_100'] for b in PARENTS)
 adopted=adopted and summary[BIN]['max_elapsed_ms']<=2000
 result=dict(adopted=adopted,source_sha256=frozen['source_sha256'],holdout_cases=200,summary=summary,comparisons=comparisons,
  saved_parent_selection=policy,groups=groups,tools_in100=hundred,viewer_verified=True,mechanism_verified=True,
  all_legal_home=True,holdout_run_ids={b:next(iter(data.values()))['run_id'] for b,data in datasets.items()})
 save(WORK/'final_comparison.json',result);save(WORK/'holdout_cases.json',rows)
 with (WORK/'holdout_comparison.csv').open('w',newline='') as f:
  w=csv.writer(f);w.writerow(['case',*FEATURES,'v076','v079','v201','selected_parent'])
  for r in rows:w.writerow([r['case'],*[r['features'][n] for n in FEATURES],*[r['scores'][b] for b in [*PARENTS,BIN]],PARENTS[r['use_nn']]])
 print(json.dumps(result,ensure_ascii=False,indent=2))
if __name__=='__main__':main()
