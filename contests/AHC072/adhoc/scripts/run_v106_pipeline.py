#!/usr/bin/env python3
"""CPU教師採取と、一つのGPUを共有する登録済みの段階的学習。"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback
from v089_data import ROOT, save, sha, status, now
from v106_data import RUN


def load(path):return json.loads(path.read_text())


def execute(root):
    pipe=root/'pipeline';pipe.mkdir(parents=True,exist_ok=True)
    lock=(pipe/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (root/'result.json').exists():return
    sources=[ROOT/'adhoc/scripts'/p for p in ('run_v106_pipeline.py','v106_data.py','train_v106_selector.py',
        'train_v087_value.py','train_v086_policy.py','v087_data.py','v086_data.py','v085_data.py','build_v106_collector.py')]
    sources.extend(ROOT/'adhoc/bin'/p for p in ('collect_v106_rewards.cpp','v106_probe_base.cpp','extract_v086_graph.cpp'))
    identity=dict(sources={str(p.relative_to(ROOT)):sha(p) for p in sources},collector_sha256=sha(ROOT/'target/release/collect_v106_rewards'),
                  extractor_sha256=sha(ROOT/'target/release/extract_v086_graph'))
    if (pipe/'config.json').exists():assert load(pipe/'config.json')==identity
    else:
        save(pipe/'config.json',identity)
        for p in sources+[ROOT/'notes/experiments/v106.md']:
            dest=pipe/'frozen'/p.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,dest)
    def stage(name,command,marker):
        if marker.exists():return
        for path,digest in identity['sources'].items():assert sha(ROOT/path)==digest,path
        status(pipe,name,pid=os.getpid())
        with (pipe/(name+'.log')).open('a') as f:
            child=subprocess.run([sys.executable]+list(map(str,command)),cwd=ROOT,stdout=f,stderr=subprocess.STDOUT)
        save(pipe/(name+'_exit.json'),dict(exit_code=child.returncode,finished_at=now()))
        assert child.returncode==0 and marker.exists(),name
    started=time.monotonic()
    try:
        results={}
        for phase,seconds in (('diagnostic',1800),('main',5400)):
            where=root/phase
            stage(phase+'_collect',[ROOT/'adhoc/scripts/v106_data.py','--run',where,'--phase',phase,'--workers',16],where/'dataset.json')
            stage(phase+'_quality',[ROOT/'adhoc/scripts/v106_data.py','--run',where,'--quality'],where/'teacher_quality.json')
            if not load(where/'teacher_quality.json')['gate_passed']:
                results[phase]=dict(gate_passed=False,reason='teacher_headroom');break
            # GPUは容量診断の2本が終了してから使う。待機中にGPUを確保しない。
            capacity=ROOT/'results/nn_rank/v107/20261004_capacity_studio'
            status(pipe,'waiting_for_capacity_gpu',next_phase=phase,pid=os.getpid())
            while True:
                completed=(capacity/'diagnostic/wide/training/result.json').exists()
                state=load(capacity/'gpu_state.json')
                if completed and not state['active']:break
                failed=capacity/'pipeline/exit.json'
                if failed.exists() and load(failed)['exit_code']!=0:raise RuntimeError('capacity pipeline failed before GPU release')
                time.sleep(10)
            stage(phase+'_train',[ROOT/'adhoc/scripts/train_v106_selector.py','--run',where,'--seconds',seconds,
                                 '--gpu-state',root/'gpu_state.json'],where/'result.json')
            results[phase]=load(where/'result.json')
            if not results[phase]['gate_passed']:break
        ready='main' in results and results['main']['gate_passed']
        save(root/'result.json',dict(phases=results,ready_for_cpp=ready,completed_at=now()))
        save(pipe/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,finished_at=now()))
        status(pipe,'ready_for_cpp' if ready else 'diagnostic_stopped',next_stage='record registered decision; continue qualifying v107 main')
    except BaseException as e:
        save(pipe/'exit.json',dict(exit_code=1,error=repr(e),traceback=traceback.format_exc(),finished_at=now()))
        status(pipe,'failed',error=repr(e));raise


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args();execute(a.run)
