#!/usr/bin/env python3
"""完走方策の全操作を、四つのLNS結果の平均費用で更新する。"""
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
import time
import numpy as np
from v089_data import ROOT, Geometry, remaining
from v090_data import RUN as BC_RUN
from v091_env import load
from v099_complete import configuration as parent_configuration
from v102_learning import collect as collect_complete
from v109_dp import INITIAL_SHA, make_rngs, blocked_inputs as parent_blocked
from run_v112_labels import invoke, verify


def configuration():
    return dict(parent_configuration('group'), seed=112103, seed_base=1121000000000,
                iterations=32, seconds=2700, initial_lr=3e-6, final_lr=1e-6,
                value_coef=0., bc_coef=0., max_episode_steps=2048, completion_workers=20)


def blocked_inputs(root, completed_iterations=0):
    blocked=parent_blocked(root,completed_iterations)
    blocked.update(load(root/'diagnostic/excluded_sha256.json'))
    blocked.update(c['sha256'] for c in load(root/'diagnostic/input_manifest.json'))
    return blocked


def build_completion(root):
    assert load(root/'diagnostic/result.json')['promising_teacher']
    return root/'diagnostic/helper_local'


class CompletionPool:
    def __init__(self,binary,workers):
        self.binary=binary;self.workers=workers
        self.executor=ThreadPoolExecutor(max_workers=workers)
    def close(self):self.executor.shutdown(wait=True)
    def one(self,text,row,seed,geo):
        payload=text+f"{len(row['actions'])} {seed}\n"+' '.join(map(str,row['actions']))+'\n'
        result,elapsed=invoke(self.binary,'grow',payload)
        assert result['seed']==seed and result['initial_T']==row['T']
        assert result['state_pool_free']==4 and result['invalid_candidates']==0 and result['attempts']>0
        for sample in result['samples']:
            verify(geo,sample);assert sample['E']==0 and sample['T']<=row['T']
        return dict(seed=seed,T=result['samples'][-1]['T'],actions=result['samples'][-1]['actions'],
                    attempts=result['attempts'],seconds=elapsed,verified=True)
    def complete(self,rows,items,cohort):
        seeds=[1121001+4*cohort+i for i in range(4)]
        # Geometryは読み取り専用。作業ディレクトリも採取ごとに再利用する。
        directory=self.binary.parent.parent/'lns_inputs';directory.mkdir(exist_ok=True)
        geometries={};by_id={x['id']:x for x in items}
        for item in items:
            path=directory/f"{item['id']}.txt";path.write_text(item['text'])
            geometries[item['id']]=Geometry(path)
        jobs=[(i,seed) for i,row in enumerate(rows) if row['E']==0 for seed in seeds]
        pending={};next_job=0;done=0
        for row in rows:row['lns_results']=[]
        while done<len(jobs):
            while len(pending)<self.workers and next_job<len(jobs):
                i,seed=jobs[next_job];next_job+=1;row=rows[i]
                future=self.executor.submit(self.one,by_id[row['case']]['text'],row,seed,geometries[row['case']])
                pending[future]=i
            future=next(as_completed(pending));i=pending.pop(future)
            rows[i]['lns_results'].append(future.result());done+=1
        for row in rows:
            row['lns_results'].sort(key=lambda x:x['seed'])
            assert len(row['lns_results'])==(4 if row['E']==0 else 0)
            row['lns_mean_T']=float(np.mean([x['T'] for x in row['lns_results']])) if not row['E'] else float(row['T'])
        for item in items:(directory/f"{item['id']}.txt").unlink()


def targets(rows, buffer, repetitions):
    costs=np.array([r['lns_mean_T']/100.+(30.+r['E']/256. if r['E'] else 0.) for r in rows])
    grouped=costs.reshape(-1,repetitions)
    relative=((grouped.sum(1,keepdims=True)-grouped)/(repetitions-1)-grouped).reshape(-1)
    return (-costs[buffer['episode']]).astype(np.float32), relative[buffer['episode']].astype(np.float32), costs, relative


def collect(model,engine,queue,pool,rngs,device,config,report=None):
    tick=time.monotonic()
    buffer,rows,states,items=collect_complete(model,engine,queue,rngs['action'],rngs['transform'],device,config,report)
    nn_seconds=time.monotonic()-tick;tick=time.monotonic()
    pool.complete(rows,items,config.get('cohort_index',0));lns_seconds=time.monotonic()-tick
    returns,advantages,costs,relative=targets(rows,buffer,config['repetitions'])
    # LNSが作った操作は勾配の対象にしない。価値予測による補填も使わない。
    buffer['returns']=returns
    buffer['advantages']=advantages
    for i,row in enumerate(rows):row.update(cost=float(costs[i]),group_advantage=float(relative[i]))
    return buffer,rows,states,items,dict(nn_collect_seconds=nn_seconds,lns_seconds=lns_seconds,
            lns_tasks=sum(len(r['lns_results']) for r in rows))


def summary(rows):
    good=[r for r in rows if r['E']==0]
    return dict(episodes=len(rows),successes=len(good),mean_T_completed=float(np.mean([r['T'] for r in good])) if good else None,
        mean_lns_T_completed=float(np.mean([r['lns_mean_T'] for r in good])) if good else None,
        mean_E_finished=float(np.mean([r['E'] for r in rows])),mean_cost_all=float(np.mean([r['cost'] for r in rows])),
        mean_lns_saved_completed=float(np.mean([r['T']-r['lns_mean_T'] for r in good])) if good else None)
