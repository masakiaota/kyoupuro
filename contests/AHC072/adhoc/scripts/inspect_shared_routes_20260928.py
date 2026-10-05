#!/usr/bin/env python3
"""Replay saved answers and count route overlap; no search or answer modification."""
from collections import defaultdict
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'adhoc/play_global_20260928'
D={'U':(-1,0),'D':(1,0),'L':(0,-1),'R':(0,1)}
results=[]
for source in sorted((ROOT/'tools/in').glob('*.txt')):
    lines=source.read_text().splitlines();n,k=map(int,lines[0].split());cells=''.join(lines[1:])
    towers=[[p] if c.islower() else [] for p,c in enumerate(cells)]
    last_move={};last_path={};windows=defaultdict(list);encounters=[]
    for t,line in enumerate((ROOT/'results/out/v037_pair_transfer_compression'/source.name).read_text().splitlines(),1):
        i,j,keep,dr,length=line.split();i,j,keep,length=map(int,(i,j,keep,length));di,dj=D[dr]
        p=i*n+j;q=(i+di*length)*n+j+dj*length
        ids=tuple(sorted(towers[p][keep:]));assert ids
        previous=last_path.get(ids)
        if previous and previous[0]==p and all(last_move.get(x)==previous[1] for x in ids):
            path=previous[2]+[(p,q,t)]
        else:path=[(p,q,t)]
        if len(path)>=3:
            tail=path[-3:];key=tuple((a,b) for a,b,_ in tail)
            windows[key].append((ids,tail[0][2],t))
        last_path[ids]=(q,t,path)
        for x in ids:last_move[x]=t
        moving=towers[p][keep:];towers[p]=towers[p][:keep];towers[q].extend(reversed(moving))
        if len(towers[q])>1:encounters.append(frozenset(towers[q]))
        for at in (p,q):
            while towers[at] and cells[towers[at][-1]].upper()==cells[at]:towers[at].pop()
    assert not any(towers)
    pairs=[];seen=set()
    for route,visits in windows.items():
        for i,(left,t1,t2) in enumerate(visits):
            for right,u1,u2 in visits[i+1:]:
                a,b=frozenset(left),frozenset(right);union=a|b
                if a&b or len(union)>8 or len({cells[x] for x in union})<2:continue
                key=(tuple(sorted(a)),tuple(sorted(b)),route)
                if key in seen:continue
                seen.add(key)
                pairs.append({'route':[[divmod(p,n),divmod(q,n)] for p,q in route],
                              'left_ids':[divmod(p,n) for p in left], 'right_ids':[divmod(p,n) for p in right],
                              'turns':[[t1,t2],[u1,u2]],'union_size':len(union),
                              'ever_shared_tower':any(union<=s for s in encounters)})
    results.append({'case':source.name,'overlap_pairs':len(pairs),'never_shared_tower':sum(not p['ever_shared_tower'] for p in pairs),'pairs':pairs})
summary={'cases':len(results),'overlap_pairs':sum(r['overlap_pairs'] for r in results),
         'overlap_cases':sum(r['overlap_pairs']>0 for r in results),
         'never_shared_tower':sum(r['never_shared_tower'] for r in results),
         'never_shared_tower_cases':sum(r['never_shared_tower']>0 for r in results),
         'note':'Three consecutive parcel flights over identical directed endpoints; different disjoint parcels with combined size <=8. These counts do not establish legal or beneficial batching.'}
(OUT/'shared_routes.json').write_text(json.dumps({**summary,'details':results},ensure_ascii=False,indent=2)+'\n')
print(json.dumps(summary,ensure_ascii=False,indent=2))
for r in results:
    for p in r['pairs']:
        if not p['ever_shared_tower']:
            print(json.dumps({'case':r['case'],**p},ensure_ascii=False));break
    if r['never_shared_tower']:break
