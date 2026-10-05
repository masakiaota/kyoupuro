#!/usr/bin/env python3
"""v122の凍結生成器と既存320入力を再利用して2,112入力へ増やす。"""
import fcntl
import json
import os
import random
import shutil
import time
import traceback

from build_v123_order import ROOT, RUN
from v090_data import RUN as BC_RUN
from v089_data import save, sha, now, status
import collect_v122_orders as collector
import train_v123_order as trainer

PREVIOUS = ROOT/'results/nn_rank/v122/20261004_order_studio'
def load(path):return json.loads(path.read_text())


def prepare():
    old=load(PREVIOUS/'inputs.json')
    excluded=set(load(PREVIOUS/'input_exclusion.json')['excluded_hashes'])
    available=[c for c in load(BC_RUN/'input_manifest.json') if c['role']=='train' and c['sha256'] not in excluded]
    random.Random(122001).shuffle(available)
    assert [c['sha256'] for c in available[:320]]==[c['sha256'] for c in old]
    cases=old[:256]+[dict(c,order_role='train') for c in available[320:2112]]+old[256:]
    assert len(cases)==2112 and sum(c['order_role']=='train' for c in cases)==2048
    assert len({c['sha256'] for c in cases})==2112
    for name,value in [('inputs.json',cases),('input_exclusion.json',load(PREVIOUS/'input_exclusion.json')),
                       ('pilot/result.json',dict(load(PREVIOUS/'pilot/result.json'),reused_from=str(PREVIOUS/'pilot/result.json')))]:
        path=RUN/name
        if path.exists():assert load(path)==value
        else:save(path,value)
    binary=RUN/'collector'
    if binary.exists():assert sha(binary)==sha(PREVIOUS/'collector')
    else:shutil.copy2(PREVIOUS/'collector',binary)
    for case in old:
        name=f"{case['index']:06d}"
        dest=RUN/'data'/name;dest.mkdir(parents=True,exist_ok=True)
        for filename in ('rows.jsonl.gz','result.json','stderr.log'):
            source=PREVIOUS/'data'/name/filename;target=dest/filename
            if target.exists():assert sha(source)==sha(target)
            else:os.link(source,target)
    identity=dict(previous=str(PREVIOUS.relative_to(ROOT)),reused_cases=len(old),new_cases=1792,
                  collector_sha256=sha(binary),input_manifest_sha256=sha(RUN/'inputs.json'),
                  scripts={name:sha(ROOT/'adhoc/scripts'/name) for name in
                           ['build_v123_order.py','train_v123_order.py','run_v123_scaling.py','collect_v122_orders.py']})
    if (RUN/'config.json').exists():assert load(RUN/'config.json')==identity
    else:
        save(RUN/'config.json',identity)
        frozen=RUN/'frozen';frozen.mkdir(exist_ok=True)
        for name in identity['scripts']:shutil.copy2(ROOT/'adhoc/scripts'/name,frozen/name)
        for name in ['collect_v122_orders.cpp','v122_order_core.cpp']:
            shutil.copy2(PREVIOUS/'frozen'/name,frozen/name)


def run():
    RUN.mkdir(parents=True,exist_ok=True)
    lock=(RUN/'scaling.lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    save(RUN/'launch.json',dict(pid=os.getpid(),started_at=now(),stage='data_and_training'))
    started=time.monotonic()
    try:
        prepare()
        if not (RUN/'full/result.json').exists():
            collector.RUN=RUN
            collector.collect(full=True)
        if not (RUN/'training/result.json').exists():trainer.train()
        result=load(RUN/'training/result.json')
        status(RUN/'scaling','completed',passed=result['development']['passed'],seconds=time.monotonic()-started)
        save(RUN/'scaling/exit.json',dict(exit_code=0,completed_at=now(),seconds=time.monotonic()-started))
    except Exception as error:
        status(RUN/'scaling','failed',error=str(error))
        save(RUN/'scaling/exit.json',dict(exit_code=1,error=str(error),completed_at=now()))
        traceback.print_exc();raise


if __name__=='__main__':run()
