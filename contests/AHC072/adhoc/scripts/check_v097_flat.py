#!/usr/bin/env python3
"""移行時と終了時の固定重みについて提出用C++の数値を照合する。"""
import argparse,json,subprocess
from pathlib import Path
import numpy as np
import torch
from build_v090_board import build,quantize
from check_v089_board import compile_binary,transformed_input
from train_v090_board import Model,tensors
from v090_data import Dataset,Geometry,save,sha,now
from v097_policy import ROOT,RUN,BC_RUN
from v091_env import load

def numerical(root, phase):
    directory = root / 'numerical' / 'flat'; directory.mkdir(parents=True, exist_ok=True)
    marker = directory / 'result.json'; model_path = root / 'flat/model.json'
    if marker.exists():
        result = load(marker); assert result['model_sha256'] == sha(model_path); return result
    source = ROOT / 'adhoc/bin/v097_flat.cpp'; storage = build(model_path, source)
    source.write_text(source.read_text().replace('// epoch=', '// PPO iteration=', 1))
    diagnostic = ROOT / 'adhoc/bin' / 'check_v097_flat.cpp'
    diagnostic.write_text(f'// {diagnostic.name}\n#define V090_DIAGNOSTIC\n#include "../../{source.relative_to(ROOT)}"\n')
    binaries = [directory / 'checker_local', directory / 'checker_judge']
    for binary, local in zip(binaries, [True, False]): compile_binary(diagnostic.stem, binary, local)
    data = Dataset(BC_RUN / 'data'); saved = load(model_path); model = Model(saved['spec'])
    restored, _, _ = quantize(saved['parameters']); model.load_state_dict({k: torch.from_numpy(v) for k, v in restored.items()})
    model.eval(); errors = dict(board_features=0., action_features=0., logits=0., value=0.); checks = 0
    for case in [c for c in data.cases if c['role'] == 'train'][:4]:
        fid = case['frame_start'] + case['frames'] // 2; geo = Geometry(BC_RUN / case['path'])
        for group in range(8):
            mapping = data.maps[geo.N][group]; inp = transformed_input(geo, mapping)
            state = np.zeros(400, np.uint32); state[mapping] = data.states[fid]
            snapshot = directory / 'snapshot.txt'; snapshot.write_text(' '.join(map(str, state)) + '\n')
            raw = data.batch([fid], groups=[group])
            with torch.no_grad(): logits, value = model(tensors(raw, 'cpu'))
            codes = data.codes[data.offsets[fid]:data.offsets[fid+1]].astype(np.int64)
            transformed = mapping[codes & 511] | (codes & (7 << 9)) | (data.dirs[geo.N][group][(codes >> 12) & 3] << 12) | (codes & (7 << 14))
            for binary in binaries if group == 0 else binaries[:1]:
                proc = subprocess.run([binary, snapshot, 'dump'], input=inp, text=True, capture_output=True, check=True, timeout=10)
                out = json.loads(proc.stdout); index = {code: i for i, code in enumerate(out['codes'])}
                assert set(index) == set(transformed.tolist())
                order = [index[int(code)] for code in transformed]
                values = dict(board_features=np.max(np.abs(np.array(out['x']).reshape(400,40)-raw['x'][0].reshape(40,400).T)),
                              action_features=np.max(np.abs(np.array(out['features']).reshape(-1,16)[order]-raw['features'][0])),
                              logits=np.max(np.abs(np.array(out['logits'])[order]-logits[0].numpy())), value=abs(out['value']-float(value[0])))
                for k,v in values.items(): errors[k]=max(errors[k],float(v))
                checks += 1
    assert errors['board_features']<2e-6 and errors['action_features']<2e-6 and errors['logits']<.003 and errors['value']<.03, errors
    result = dict(passed=True, checks=checks, errors=errors, model_sha256=sha(model_path), solver_sha256=sha(source), storage=storage, completed_at=now())
    save(marker, result); print(json.dumps(result), flush=True); return result


if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args()
 torch.set_num_threads(2);torch.set_num_interop_threads(2);numerical(a.run.resolve(),'flat')
