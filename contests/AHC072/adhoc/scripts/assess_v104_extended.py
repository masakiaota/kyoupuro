#!/usr/bin/env python3
"""固定した追加学習量ごとに、同じ未学習入力の完成手数を測る。"""
import argparse
from pathlib import Path
import torch
from v089_data import ROOT,save,now
from v091_env import BC_RUN,load
from v104_extended import PARENT
from assess_v099_complete import compare
from assess_v102_ablation import numerical,evaluate,statistics


def assess(root,point):
    where=root/'points'/f'{point:04d}';marker=where/'assessment.json'
    if marker.exists():return load(marker)
    source=ROOT/'adhoc/bin'/f'v104_point_{point:04d}.cpp';numerical(where,source)
    cases=[dict(c,filename=f"{c['index']:06d}.txt") for c in load(BC_RUN/'input_manifest.json') if c['role']=='validation']
    assert len(cases)==256
    greedy=evaluate(where,'validation',cases,source)
    baseline=load(PARENT/'assessment.json');result=dict(point=point,greedy=greedy,comparison=compare(greedy,baseline['greedy']))
    if point==320:
        records=[]
        for seed in (102041,102042,102043,102044):
            condition=f'stochastic_{seed}';variant=ROOT/'adhoc/bin'/f'{source.stem}_{condition}.cpp';code=source.read_text()
            assert code.count('uint64_t rng=90003;')==code.count('if(!rng)rng=90003;')==1
            code=code.replace(f'// {source.name}',f'// {variant.name}',1).replace('uint64_t rng=90003;',f'uint64_t rng={seed};').replace('if(!rng)rng=90003;',f'if(!rng)rng={seed};')
            if variant.exists():assert variant.read_text()==code
            else:variant.write_text(code)
            records.append(evaluate(where,'validation',cases,variant,condition))
        rows=[dict(row,case=f'{rep}:{row["case"]}') for rep,record in enumerate(records) for row in record['rows']]
        previous={r['case']:r for r in baseline['stochastic']['rows']};common=[(previous[r['case']],r) for r in rows if previous[r['case']]['E']==r['E']==0]
        result['stochastic']=dict(metrics=statistics(rows),rows=rows,all_legal=all(r['all_legal'] for r in records),
             common_completed=len(common),mean_T_difference_on_common=sum(b['T']-a['T'] for a,b in common)/len(common) if common else None,
             mean_S_difference_all=statistics(rows)['mean_S_all']-baseline['stochastic']['metrics']['mean_S_all'])
    save(marker,dict(result,completed_at=now()));return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',required=True,type=Path);p.add_argument('--point',required=True,type=int,choices=(80,160,320));a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2);result=assess(a.run.resolve(),a.point)
    print(dict(point=a.point,metrics=result['greedy']['metrics'],comparison=result['comparison']))
