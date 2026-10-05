#!/usr/bin/env python3
"""保存した短縮前後だけから、現在の区間制限との関係を集計する。探索は実行しない。"""
from pathlib import Path
from collections import Counter,defaultdict
import bisect,json
from v116_data import BC,save,load
from run_v119_pairs import RUN


def board(path):
    lines=path.read_text().splitlines();N,K=map(int,lines[0].split());grid=lines[1:1+N]
    positions=[(i,j) for i in range(N) for j in range(N) if grid[i][j]!='#']
    ids={pos:c for c,pos in enumerate(positions)}
    initial=[];nest=[]
    for i,j in positions:
        ch=grid[i][j];initial.append(ord(ch)-96 if ch.islower() else 0);nest.append(ord(ch)-64 if ch.isupper() else 0)
    ray={}
    for p,(i,j) in enumerate(positions):
        for d,(di,dj) in enumerate([(-1,0),(1,0),(0,-1),(0,1)]):
            for length in range(1,9):
                q=ids.get((i+di*length,j+dj*length))
                if q is None:break
                ray[p,d,length]=q
    return tuple(initial),nest,ray


def height(word):return (word.bit_length()+3)//4

def replay(initial,nest,ray,path):
    a=list(initial);states=[initial]
    for p,k,d,length in path:
        q=ray[p,d,length];assert 0<=k<height(a[p]) and length<=k+1
        packet=a[p]>>(4*k);a[p]&=(1<<(4*k))-1
        assert height(a[q])+height(packet)<=8
        reverse=0
        while packet:reverse=(reverse<<4)|(packet&15);packet>>=4
        a[q]|=reverse<<(4*height(a[q]))
        for c in [p,q]:
            while a[c] and (a[c]>>(4*(height(a[c])-1)))==nest[c]:a[c]&=(1<<(4*(height(a[c])-1)))-1
        states.append(tuple(a))
    assert not any(a)
    return states


def pairs(path):
    it=iter(map(int,path.read_text().split()));n=next(it);next(it);next(it)
    for record in range(n):
        kind=next(it);na=next(it);nb=next(it)
        a=[tuple(next(it) for _ in range(4)) for _ in range(na)]
        b=[tuple(next(it) for _ in range(4)) for _ in range(nb)]
        yield record,kind,a,b


def analyze():
    counts=Counter();examples={};teacher=[]
    for case in sorted((RUN/'cases').glob('*')):
        meta=load(case/'complete.json');initial,nest,ray=board(BC/meta['input']['path'])
        for row in (case/'teachers.jsonl').read_text().splitlines():teacher.append(dict(json.loads(row),case=case.name))
        for record,kind,a,b in pairs(case/'pairs.txt'):
            sa=replay(initial,nest,ray,a);sb=replay(initial,nest,ray,b);index=defaultdict(list)
            for j,state in enumerate(sb):index[state].append(j)
            boundaries=[(0,0)];last=0
            for i,state in enumerate(sa[1:],1):
                js=index.get(state,[]);at=bisect.bisect_right(js,last)
                if at<len(js):last=js[at];boundaries.append((i,last))
            assert boundaries[-1]==(len(a),len(b))
            for (ai,bi),(az,bz) in zip(boundaries,boundaries[1:]):
                parents=list(range(len(initial)))
                def root(c):
                    while parents[c]!=c:parents[c]=parents[parents[c]];c=parents[c]
                    return c
                for path,start,end in [(a,ai,az),(b,bi,bz)]:
                    for p,k,d,l in path[start:end]:parents[root(p)]=root(ray[p,d,l])
                left=defaultdict(list);right=defaultdict(list)
                for t in range(ai,az):left[root(a[t][0])].append(t)
                for t in range(bi,bz):right[root(b[t][0])].append(t)
                for group,times in left.items():
                    new=right[group]
                    if len(times)<=len(new):continue
                    counts['shorter_components']+=1
                    cells={c for t in times for c in (a[t][0],ray[a[t][0],a[t][2],a[t][3]])}
                    state=sa[times[0]];pieces=sum(height(state[c]) for c in cells);towers=sum(state[c]!=0 for c in cells)
                    constraints={'fewer_than_three_moves':len(times)<3,'over_twelve_moves':len(times)>12,
                                 'over_eight_cells':len(cells)>8,'over_sixteen_pieces':pieces>16,'over_six_towers':towers>6,
                                 'over_sixty_four_span':times[-1]-times[0]>=64,
                                 'at_most_four_cells_without_nest':len(cells)<=4 and not any(nest[c] for c in cells)}
                    excluded=False
                    for name,bad in constraints.items():
                        if bad:
                            counts[name]+=1;excluded=True
                            examples.setdefault(name,dict(case=case.name,pair=record,kind=kind,before=len(times),after=len(new),cells=len(cells),pieces=pieces,towers=towers,span=times[-1]-times[0]+1))
                    counts['violates_at_least_one_simple_constraint']+=excluded
    assert counts['shorter_components']==load(RUN/'result.json')['statistics']['shorter_components']
    comparison=Counter('teacher_better' if x['after']<x['baseline'] else 'baseline_better' if x['after']>x['baseline'] else 'equal' for x in teacher)
    result=dict(counts=counts,constraint_counts_overlap=True,examples=examples,teacher_comparison=comparison,
                teacher_saved=sum(x['before']-x['after'] for x in teacher),
                baseline_saved=sum(x['before']-x['baseline'] for x in teacher),
                decisive_teachers=[x for x in teacher if x['after']<x['baseline']],
                interpretation='単純な容量条件の違反を重複込みで数えた。全ての不採取理由を一意に分類した値ではない。solver実行なし。')
    save(RUN/'saved_analysis.json',result)
    print(json.dumps({k:v for k,v in result.items() if k not in ['decisive_teachers']},ensure_ascii=False,indent=2))

if __name__=='__main__':analyze()
