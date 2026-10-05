#!/usr/bin/env python3
"""事前固定した特徴・5分割比較からv201の選択規則を一度だけ決める。"""
from collections import deque
from pathlib import Path
import csv,json,math,random,statistics,sys
from v201_fresh import ROOT,WORK,PARENTS,BIN,read,save,sha
FEATURES=['M','N','K','floors','wall_fraction','density','mean_per_color','max_color_fraction',
 'color_concentration','rare_fraction','home_mean','home_max','detour_mean','same_nearest_mean',
 'any_nearest_mean','dead_end_fraction','straight4_fraction','stepping2_fraction']
LABELS=['スライム数','盤面辺長','色数','床数','壁率','スライム密度','1色あたり匹数','最多色割合',
 '色分布の集中度','少数色の匹数割合','巣までの平均距離','巣までの最大距離','平均遠回り率',
 '同色最近傍距離','色不問最近傍距離','行き止まり率','4マス直線の割合','2マス先の踏み台候補率']

def features(path):
 lines=path.read_text().splitlines(); N,K=map(int,lines[0].split()); grid=''.join(lines[1:]); assert len(grid)==N*N
 floors=[p for p,c in enumerate(grid) if c!='#']; slimes=[p for p,c in enumerate(grid) if 'a'<=c<='l']; M=len(slimes)
 nests={ord(c)-ord('A'):p for p,c in enumerate(grid) if 'A'<=c<='L'}
 colors={p:ord(grid[p])-ord('a') for p in slimes}; counts=[sum(c==k for c in colors.values()) for k in range(K)]
 def ray(p,d,length):
  i,j=divmod(p,N); di,dj=[(-1,0),(1,0),(0,-1),(0,1)][d]
  for l in range(1,length+1):
   r,s=i+di*l,j+dj*l
   if not(0<=r<N and 0<=s<N) or grid[r*N+s]=='#': return -1
  return r*N+s
 adj={p:[q for d in range(4) if (q:=ray(p,d,1))>=0] for p in floors}
 home=[]; detours=[]; near=[]; same=[]
 for p in slimes:
  dist={p:0}; queue=deque([p])
  while queue:
   v=queue.popleft()
   for q in adj[v]:
    if q not in dist: dist[q]=dist[v]+1; queue.append(q)
  q=nests[colors[p]]; home.append(dist[q]); detours.append(dist[q]/(abs(p//N-q//N)+abs(p%N-q%N)))
  near.append(min(dist[q] for q in slimes if q!=p))
  compatible=[dist[q] for q in slimes if q!=p and colors[q]==colors[p]]
  if compatible: same.append(min(compatible))
 F=len(floors)
 values=[M,N,K,F,1-F/(N*N),M/F,M/K,max(counts)/M,sum(c*c for c in counts)/(M*M),
  sum(c for c in counts if c<=3)/M,statistics.mean(home),max(home),statistics.mean(detours),
  statistics.mean(same) if same else 0.,statistics.mean(near),
  sum(len(adj[p])<=1 for p in floors)/F,
  sum(any(ray(p,d,4)>=0 for d in range(4)) for p in floors)/F,
  sum(any(ray(p,d,2) in colors for d in range(4)) for p in slimes)/M]
 return dict(zip(FEATURES,values))

def relative(base,score): return ((2*10**9*base+score)//(2*score))/10**7

def fit(rows,names,depth,min_leaf):
 total=sum(r['gain'] for r in rows); leaf={'use_nn':int(total>0),'n':len(rows)}
 if depth==0 or len(rows)<2*min_leaf: return leaf
 best_gain=max(total,0.); best=None
 for name in names:
  order=sorted(rows,key=lambda r:r['features'][name]); left=0.
  for i in range(1,len(order)):
   left+=order[i-1]['gain']
   if i<min_leaf or len(order)-i<min_leaf: continue
   lo,hi=order[i-1]['features'][name],order[i]['features'][name]
   if lo==hi: continue
   gain=max(left,0.)+max(total-left,0.)
   if gain>best_gain+1e-10:
    best_gain=gain; best=(name,(lo+hi)/2,order[:i],order[i:])
 if best is None: return leaf
 name,threshold,left,right=best
 return {'feature':name,'threshold':threshold,'n':len(rows),
  'left':fit(left,names,depth-1,min_leaf),'right':fit(right,names,depth-1,min_leaf)}

def predict(tree,feature):
 while 'feature' in tree: tree=tree['left' if feature[tree['feature']]<=tree['threshold'] else 'right']
 return tree['use_nn']

def rules(tree,prefix=''):
 if 'feature' not in tree: return [prefix+('v079' if tree['use_nn'] else 'v076')+f"（学習{tree['n']}件）"]
 f=LABELS[FEATURES.index(tree['feature'])]; t=tree['threshold']
 return rules(tree['left'],prefix+f'{f} <= {t:.8g} → ')+rules(tree['right'],prefix+f'{f} > {t:.8g} → ')

def measure(rows,choices):
 scores=[r['scores'][c] for r,c in zip(rows,choices)]
 return dict(total=sum(scores),mean=statistics.mean(scores),relative_100=statistics.mean(relative(min(r['scores']),s) for r,s in zip(rows,scores)),
             nn_cases=sum(choices),cases=len(rows))

def ranks(values):
 order=sorted(range(len(values)),key=values.__getitem__); answer=[0.]*len(values); i=0
 while i<len(order):
  j=i+1
  while j<len(order) and values[order[j]]==values[order[i]]: j+=1
  for k in range(i,j): answer[order[k]]=(i+j-1)/2
  i=j
 return answer

def selection():
 assert not (WORK/'selection.json').exists(),'the rule is already frozen'
 manifest=[r for r in read(WORK/'manifest.json') if r['role']=='selection500']
 datasets=[]
 for b in PARENTS:
  dataset=read(WORK/'runs'/f'v201_fresh_selection500_{b.split("_")[0]}'/'cases.json')
  assert len(dataset)==500; datasets.append({r['case_name']:r for r in dataset})
 rows=[]
 for r in manifest:
  scores=[dataset[r['case']]['score'] for dataset in datasets]
  rows.append(dict(**r,features=features(ROOT/r['path']),scores=scores,
                   gain=relative(min(scores),scores[1])-relative(min(scores),scores[0])))
 save(WORK/'selection_cases.json',rows)
 families=[('M_stump',['M'],1),('all_stump',FEATURES,1),('depth2',FEATURES,2)]
 candidates=[]
 for name,names,depth in families:
  predictions={}; fold_models=[]
  for fold in range(5):
   train=[r for r in rows if r['fold']!=fold]; test=[r for r in rows if r['fold']==fold]
   tree=fit(train,names,depth,40); fold_models.append(tree)
   predictions.update({r['case']:predict(tree,r['features']) for r in test})
  choices=[predictions[r['case']] for r in rows]
  candidates.append(dict(name=name,features=names,depth=depth,cv=measure(rows,choices),fold_models=fold_models,choices=choices))
 top=max(c['cv']['relative_100'] for c in candidates)
 selected=next(c for c in candidates if c['cv']['relative_100']>=top-.05-1e-12)
 model=fit(rows,selected['features'],selected['depth'],50)
 baselines={b:measure(rows,[i]*len(rows)) for i,b in enumerate(PARENTS)}
 # 単一指標の比較も5分割の外側予測で集計し、当てはまりとの差を残す。
 feature_analysis=[]
 gain_rank=ranks([r['gain'] for r in rows])
 for name,label in zip(FEATURES,LABELS):
  cv={}
  for fold in range(5):
   tree=fit([r for r in rows if r['fold']!=fold],[name],1,40)
   cv.update({r['case']:predict(tree,r['features']) for r in rows if r['fold']==fold})
  tree=fit(rows,[name],1,50)
  order=sorted(rows,key=lambda r:r['features'][name]); groups=[]
  for start in range(0,500,100):
   group=order[start:start+100]
   groups.append(dict(min=min(r['features'][name] for r in group),max=max(r['features'][name] for r in group),cases=100,
    mean_delta_T=statistics.mean(r['scores'][1]-r['scores'][0] for r in group),mean_relative_gain=statistics.mean(r['gain'] for r in group)))
  feature_rank=ranks([r['features'][name] for r in rows])
  feature_analysis.append(dict(feature=name,label=label,rank_correlation=statistics.correlation(feature_rank,gain_rank),
   cv=measure(rows,[cv[r['case']] for r in rows]),fit_rule=tree,groups=groups))
 result=dict(selected_family=selected['name'],model=model,rules=rules(model),cv_improves_both=selected['cv']['relative_100']>max(x['relative_100'] for x in baselines.values()),
  candidates=candidates,baselines=baselines,fit=measure(rows,[predict(model,r['features']) for r in rows]),feature_analysis=feature_analysis,
  legacy80=measure(rows,[int(r['features']['M']>=80) for r in rows]),selection_cases_sha256=sha(WORK/'selection_cases.json'),holdout_seen=False)
 save(WORK/'selection.json',result)
 with (WORK/'feature_analysis.csv').open('w',newline='') as f:
  w=csv.writer(f); w.writerow(['feature','label','rank_correlation','cv_relative_100','cv_total','fit_rule'])
  for r in feature_analysis:w.writerow([r['feature'],r['label'],r['rank_correlation'],r['cv']['relative_100'],r['cv']['total'],'; '.join(rules(r['fit_rule']))])
 print(json.dumps({k:v for k,v in result.items() if k not in ('feature_analysis','candidates')},ensure_ascii=False,indent=2))
 print('CV',json.dumps([{k:c[k] for k in ('name','cv')} for c in candidates],ensure_ascii=False))
 print('FEATURE_CV',json.dumps(sorted([dict(feature=r['feature'],label=r['label'],cv=r['cv']) for r in feature_analysis],key=lambda r:-r['cv']['relative_100']),ensure_ascii=False))
if __name__=='__main__': selection()
