#!/usr/bin/env python3
"""現在のPPO完了後、承認済みの学習をGPUで順に実行する。"""
import fcntl
import os
from pathlib import Path
import subprocess
import sys
import time
import traceback
from v096_selected import ROOT,RUN as AUX,PARENT
from v097_policy import RUN as FACTOR
from v091_env import load
from v090_data import save,now,status


def execute():
    directory=AUX/'followups';directory.mkdir(parents=True,exist_ok=True);lock=(directory/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    status(directory,'waiting_for_v092',pid=os.getpid())
    while not (PARENT/'pipeline/exit.json').exists():
        launch=load(PARENT/'launch.json');guard=Path(launch['guard_directory'])
        if (guard/'exit.json').exists():assert load(guard/'exit.json')['exit_code']==0,'v092 guard ended before pipeline completion'
        time.sleep(10)
    assert load(PARENT/'pipeline/exit.json')['exit_code']==0,'v092 needs repair'
    for version,root,script,seconds in [('v096',AUX,'run_v096_pipeline.py',16800),('v097',FACTOR,'run_v097_pipeline.py',3600)]:
        if (root/'pipeline/exit.json').exists() and load(root/'pipeline/exit.json')['exit_code']==0:continue
        gpu=root/'gpu_state.json'
        if gpu.exists():assert not load(gpu)['active'],'previous GPU stage still active'
        else:save(gpu,dict(active=False,pid=0,updated_unix=time.time(),driver_bytes=0))
        index=1
        while (root/f'guard_compare{index}').exists():index+=1
        guard=root/f'guard_compare{index}'
        command=[sys.executable,ROOT/'adhoc/scripts/memory_guard.py','--log-dir',guard,'--stop-gb','56','--limit-gb','64','--seconds',str(seconds),'--gpu-state',gpu,'--',sys.executable,ROOT/'adhoc/scripts'/script,'--run',root]
        status(directory,version,pid=os.getpid(),guard_directory=str(guard))
        with (root/f'guard_compare{index}.log').open('w') as stream:
            process=subprocess.Popen(command,stdin=subprocess.DEVNULL,stdout=stream,stderr=subprocess.STDOUT)
            save(root/'launch.json',dict(guard_pid=process.pid,guard_directory=str(guard),command=list(map(str,command)),started_at=now(),managed_by=os.getpid()))
            code=process.wait()
        assert code==0 and load(root/'pipeline/exit.json')['exit_code']==0,version+' failed'
    status(directory,'final_evaluation',pid=os.getpid())
    with (directory/'final.log').open('a') as stream:
        process=subprocess.run([sys.executable,ROOT/'adhoc/scripts/finalize_v096_candidate.py'],stdout=stream,stderr=subprocess.STDOUT,cwd=ROOT)
    assert process.returncode==0,'final evaluation failed'
    status(directory,'completed');save(directory/'exit.json',dict(exit_code=0,finished_at=now()))


if __name__=='__main__':
    try:execute()
    except BlockingIOError:print('followups already running',file=sys.stderr);sys.exit(76)
    except BaseException as e:
        save(AUX/'followups/exit.json',dict(exit_code=1,error=repr(e),finished_at=now()));traceback.print_exc();sys.exit(1)
