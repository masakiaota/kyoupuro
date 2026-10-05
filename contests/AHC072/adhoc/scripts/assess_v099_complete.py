#!/usr/bin/env python3
"""v096の検証手順を使い、固定した1候補・1入力集合を1回だけ測る。"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import numpy as np
import torch
from build_v090_board import build, quantize
from check_v089_board import compile_binary, transformed_input, trace
from train_v090_board import Model, tensors
from v090_data import Dataset, Geometry, save, sha, now
from run_v089_evaluation import validated_output, statistics
from v091_env import load
from v099_complete import ROOT, BC_RUN

def numerical(root, source):
    directory = root / 'numerical'; directory.mkdir(parents=True, exist_ok=True)
    marker = directory / 'result.json'; model_path = root / 'training/model.json'
    if marker.exists():
        result = load(marker); assert result['model_sha256'] == sha(model_path); return result
    storage = build(model_path, source)
    source.write_text(source.read_text().replace('// epoch=', '// PPO iteration=', 1))
    diagnostic = ROOT / 'adhoc/bin' / f'check_{source.stem}.cpp'
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


def evaluate(root, role, cases, source):
    condition = 'greedy'
    directory = root / 'evaluation' / role / condition; directory.mkdir(parents=True, exist_ok=True)
    marker = directory / 'result.json'
    if marker.exists():
        result = load(marker); assert result['solver_sha256'] == sha(source); return result
    name = f'{source.stem}_{role}_{condition}'
    wrapper = ROOT / 'adhoc/bin' / f'{name}.cpp'
    wrapper.write_text(f'// {name}.cpp\n' + ('#define V090_STOCHASTIC\n' if condition == 'stochastic' else '')
                       + f'#include "../../{source.relative_to(ROOT)}"\n')
    if role == 'final_in':
        inputs = ROOT / 'tools/in'
    else:
        inputs = directory / 'inputs'; inputs.mkdir(exist_ok=True)
        for case in cases:
            original = BC_RUN / case['path']; assert sha(original) == case['sha256']
            shutil.copy2(original, inputs / case['filename'])
    assert len(list(inputs.glob('*.txt'))) == len(cases)
    save(directory / 'input_manifest.json', cases)
    label = f'{root.parent.name}_{name}'; started = directory / 'started.json'
    identity = dict(label=label, name=name, solver_sha256=sha(source), model_sha256=sha(root / 'training/model.json'),
                    input_sha256=sha(directory / 'input_manifest.json'), local=True, jobs=20)
    scratch = ROOT / 'results/out' / name
    if not started.exists():
        assert not scratch.exists(), f'refuse to overwrite prior evaluation {scratch}'
        for local in (False, True): compile_binary(name, directory / ('binary_local' if local else 'binary_judge'), local)
        save(started, identity)
        with (directory / 'eval.log').open('w') as stream:
            process = subprocess.run([sys.executable, ROOT / 'scripts/eval.py', name, inputs, '-j', '20', '--wait-lock', '--label', label],
                                     cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
        save(directory / 'exit.json', dict(exit_code=process.returncode, finished_at=now()))
        assert process.returncode == 0, f'evaluation failed: {directory}'
    else: assert load(started) == identity
    records = []
    with (ROOT / 'results/eval_records.jsonl').open() as stream:
        for line in stream:
            row = json.loads(line)
            if row['label'] == label: records.append(row)
    assert len(records) == len({r['case_name'] for r in records}) == len(cases), 'partial evaluation requires inspection'
    assert len({r['run_id'] for r in records}) == 1
    save(directory / 'official_records.json', records)
    archive = directory / 'outputs'
    if not archive.exists(): shutil.copytree(scratch, archive)
    indexed = {r['case_name']: r for r in records}; rows = []
    for case in cases:
        filename = case['filename']; record = indexed[filename]
        assert record['status'] == 'ok' and record['local']
        replay = validated_output(inputs / filename, (archive / filename).read_text())
        counts = trace((archive / (filename + '.err')).read_text())
        assert replay['S'] == record['score'] and counts['E'] == replay['E'] and counts['T'] == replay['T']
        rows.append(dict(case=case['index'], elapsed_ms=record['elapsed'], trace=counts, **replay))
    result = dict(metrics=statistics(rows), rows=rows, all_legal=True, solver_sha256=sha(source), completed_at=now())
    save(marker, result); shutil.copy2(source, directory / source.name)
    return result


def compare(current, baseline):
    old = {r['case']: r for r in baseline['rows']}; pairs = [(old[r['case']], r) for r in current['rows']]
    assert len(pairs) == len(old) == 256
    common = [(a,b) for a,b in pairs if a['E'] == b['E'] == 0]
    return dict(baseline=baseline['metrics'], current=current['metrics'], common_completed=len(common),
                mean_T_difference_on_common=float(np.mean([b['T']-a['T'] for a,b in common])) if common else None,
                mean_S_difference_all=current['metrics']['mean_S_all']-baseline['metrics']['mean_S_all'],
                completion_difference=current['metrics']['complete']-baseline['metrics']['complete'],
                wins_on_common=sum(b['T']<a['T'] for a,b in common), ties_on_common=sum(b['T']==a['T'] for a,b in common),
                losses_on_common=sum(b['T']>a['T'] for a,b in common))


if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);p.add_argument('--source',type=Path,required=True)
    p.add_argument('--stage',choices=('numerical','validation','test','final_in'),required=True);a=p.parse_args()
    torch.set_num_threads(2);torch.set_num_interop_threads(2)
    if a.stage=='numerical':numerical(a.run.resolve(),a.source.resolve())
    else:
        assert load(a.run/'numerical/result.json')['passed']
        if a.stage=='final_in':
            cases=[dict(index=i,filename=x.name,sha256=sha(x)) for i,x in enumerate(sorted((ROOT/'tools/in').glob('*.txt')))]
            assert len(cases)==100
        else:
            cases=[dict(c,filename=f"{c['index']:06d}.txt") for c in load(BC_RUN/'input_manifest.json') if c['role']==a.stage]
            assert len(cases)==256
        print(json.dumps(evaluate(a.run.resolve(),a.stage,cases,a.source.resolve())),flush=True)
