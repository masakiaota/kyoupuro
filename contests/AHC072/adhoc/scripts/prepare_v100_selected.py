#!/usr/bin/env python3
"""既存の選別逆教師を、今回の固定初期方策に対して再検証する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
import subprocess
import time

import numpy as np

from check_v089_board import compile_binary
from check_v098_complete import action_line
from v089_data import Geometry, remaining, save, sha, now, status
from v091_env import load
from v092_stream import excluded_inputs, digest, normalized
from v096_selected import ReverseData
from v099_complete import ROOT, REVERSE_RUN, REVERSE_SOURCE


def execute(root, reference_source, reference_model):
    directory=root/'teachers'; directory.mkdir(parents=True,exist_ok=True)
    identity=dict(reference_source_sha256=sha(reference_source), reference_model_sha256=sha(reference_model),
                  source_dataset_sha256=sha(REVERSE_SOURCE/'teachers/dataset.json'))
    marker=directory/'selection.json'
    if marker.exists():
        result=load(marker); assert result['identity']==identity; return result
    if (directory/'config.json').exists(): assert load(directory/'config.json')==identity
    else: save(directory/'config.json',identity)
    wrapper=ROOT/'adhoc/bin/select_v100_fixed.cpp'
    wrapper.write_text('// select_v100_fixed.cpp\n#define V100_POLICY_SOURCE "'+reference_source.name+'"\n#include "select_v100_teachers.cpp"\n')
    binary=directory/'selector'
    if not binary.exists(): compile_binary(wrapper.stem,binary,True)
    data=ReverseData(REVERSE_SOURCE); blocked=excluded_inputs()
    cases=[c for c in data.metadata['cases'] if c['selected_frames']]
    for case in cases: assert digest(normalized((REVERSE_SOURCE/case['path']).read_text())) not in blocked
    started=time.monotonic()
    def one(case):
        cid=case['index']; where=directory/'cases'/f'{cid:04d}'; where.mkdir(parents=True,exist_ok=True)
        if (where/'result.json').exists(): return load(where/'result.json')
        indexes=np.arange(case['frame_start'],case['frame_start']+case['selected_frames']); states=np.asarray(data.states[indexes])
        state_path=where/'states.raw'; states.astype('<u4').tofile(state_path)
        input_path=REVERSE_SOURCE/case['path']; assert sha(input_path)==case['sha256']
        if not (where/'exit.json').exists():
            # 途中出力を黙って上書きしない。異常終了なら記録を調べてから修復する。
            assert not (where/'started.json').exists(), f'partial selection needs inspection: {where}'
            save(where/'started.json',dict(started_at=now(),input_sha256=sha(input_path)))
            process=subprocess.run([binary,input_path,state_path,where],capture_output=True,text=True,timeout=180.)
            save(where/'exit.json',dict(exit_code=process.returncode,stderr=process.stderr,finished_at=now()))
        assert load(where/'exit.json')['exit_code']==0
        values=np.loadtxt(where/'policy_rows.txt',ndmin=2)
        codes=np.fromfile(where/'policy_actions.raw',dtype='<u4'); offsets=np.fromfile(where/'policy_offsets.raw',dtype='<u8')
        assert len(values)==len(indexes) and len(offsets)==len(indexes)+1 and offsets[-1]==len(codes)
        geo=Geometry(input_path); chosen=[]; gains=[]
        source=REVERSE_SOURCE/'teachers/cases'/f'{cid:04d}'
        teacher_codes=np.fromfile(source/'actions.raw',dtype='<u4')
        for j,index in enumerate(indexes):
            row=values[j]; assert int(row[0])==j
            state=states[j].copy(); actions=codes[offsets[j]:offsets[j+1]]
            assert len(actions)==int(row[1])
            for code in actions: geo.apply(state,action_line(int(code)))
            E=remaining(state); assert E==int(row[2])
            known=int(data.known_steps[index]); state=states[j].copy()
            suffix=teacher_codes[-known:]; assert len(suffix)==known and suffix[0]==data.actions[index]
            for code in suffix: geo.apply(state,action_line(int(code)))
            assert remaining(state)==0
            if E or known<len(actions):
                chosen.append(int(index)); gains.append(20 if E else int(len(actions)-known))
        result=dict(case=cid,checked=len(indexes),selected=chosen,gains=gains,all_legal=True)
        save(where/'result.json',result);return result
    results=[]
    with ThreadPoolExecutor(max_workers=8) as pool:
        futures=[pool.submit(one,c) for c in cases]
        for future in as_completed(futures):
            results.append(future.result())
            if len(results)%64==0:
                status(directory,'rechecking_reverse_teachers',completed=len(results),total=len(cases),seconds=time.monotonic()-started)
    results.sort(key=lambda r:r['case']); selected=[i for r in results for i in r['selected']]; gains=[g for r in results for g in r['gains']]
    clipped=np.minimum(np.asarray(gains,np.float64),20.)
    weights=(clipped/clipped.mean()).tolist() if len(clipped) else []
    geometries=sum(bool(r['selected']) for r in results)
    result=dict(identity=identity,checked=sum(r['checked'] for r in results),selected_indexes=selected,weights=weights,
                gains=gains,selected_states=len(selected),selected_geometries=geometries,
                mean_gain=float(np.mean(gains)) if gains else None,all_legal=True,
                training_eligible=len(selected)>=4096 and geometries>=512,seconds=time.monotonic()-started,completed_at=now())
    assert result['checked']==len(data.actions)
    save(marker,result); status(directory,'completed',selected_states=len(selected),selected_geometries=geometries,
                              training_eligible=result['training_eligible']);return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=REVERSE_RUN)
    p.add_argument('--reference-source',type=Path,required=True);p.add_argument('--reference-model',type=Path,required=True)
    a=p.parse_args(); execute(a.run.resolve(),a.reference_source.resolve(),a.reference_model.resolve())
