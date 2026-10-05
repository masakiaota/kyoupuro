#!/usr/bin/env python3
"""LNS平均費用で学習した最終重みをv401へ載せ、固定対照と比較する。"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time
import traceback

import numpy as np
import torch
from build_v090_board import build, quantize
from check_v089_board import compile_binary, transformed_input, trace
from run_v089_evaluation import validated_output, statistics
from run_v105_hybrid import preprocess
from train_v090_board import Model, tensors
from v089_data import ROOT, save, sha, now, status, Geometry
from v090_data import Dataset, RUN as BC_RUN
from v091_env import load


from v111_portfolio import without_model, packed_span, comparison, valid

CONTROL=ROOT/'results/nn_rank/v111/20261004_weight_portfolio_studio'


def with_model(root,model_path):
    parent=(CONTROL/'frozen/v401.cpp').read_text()
    source=ROOT/'adhoc/bin/v112_lns_learned.cpp'
    temporary=root/'learned/unoptimized_nn.cpp';temporary.parent.mkdir(parents=True,exist_ok=True)
    storage=build(model_path,temporary);generated=temporary.read_text()
    assert re.search(r'constexpr int NN_WIDTH=.*?;',generated).group()==re.search(r'constexpr int NN_WIDTH=.*?;',parent).group()
    a,b=packed_span(generated);packed=generated[a:b]
    a,b=packed_span(parent);content=parent[:a]+packed+parent[b:]
    content='// '+source.name+'\n'+content.split('\n',1)[1]
    assert without_model(content)==without_model(parent)
    assert len(content.encode())<=512000
    if source.exists():assert source.read_text()==content
    else:source.write_text(content)
    save(root/'learned/storage.json',dict(storage,source_sha256=sha(source)))
    return source

def numerical(root, label, source, model_path):
    where = root/label/'mechanism'; where.mkdir(parents=True, exist_ok=True)
    marker = where/'result.json'
    if marker.exists():
        result = load(marker); assert result['source_sha256'] == sha(source)
        return result
    checker = ROOT/'adhoc/bin'/f'check_v112_{label}.cpp'
    checker.write_text(f'''// {checker.name}
#define V090_DIAGNOSTIC
#define main v112_submission_main
#include "../../{source.relative_to(ROOT)}"
#undef main
int main(int argc,char** argv) {{
    if(argc!=2)return 2;
    nn::load_nn();nn::Input input;input.read();const nn::Board board(input);
    nn::BoardState state(input,board);std::ifstream snapshot(argv[1]);
    nn::read_snapshot(snapshot,board,state);nn::dump_inference(input,board,state);
}}
''')
    binaries = []
    for local in (True, False):
        mode = 'local' if local else 'judge'; binary = where/('checker_'+mode)
        compile_binary(checker.stem, binary, local); binaries.append(binary)
        compile_binary(source.stem, where/('solver_'+mode), local)
        expanded = preprocess(source, where/(mode+'.ii'), local)
        parent=(CONTROL/'base/mechanism'/(mode+'.ii')).read_text()
        assert without_model(expanded) == without_model(parent), (label, mode, 'preprocessed control mismatch')
    data = Dataset(BC_RUN/'data'); spec = load(model_path); model = Model(spec['spec'])
    restored, _, _ = quantize(spec['parameters']); model.load_state_dict({k: torch.from_numpy(v) for k, v in restored.items()})
    model.eval(); errors = dict(features=0., actions=0., logits=0., value=0.); checks = 0
    for case in [c for c in data.cases if c['role'] == 'train'][:4]:
        frame = case['frame_start']+case['frames']//2; geo = Geometry(BC_RUN/case['path'])
        for group in range(8):
            mapping = data.maps[geo.N][group]; inp = transformed_input(geo, mapping)
            state = np.zeros(400, np.uint32); state[mapping] = data.states[frame]
            snapshot = where/'snapshot.txt'; snapshot.write_text(' '.join(map(str, state))+'\n')
            raw = data.batch([frame], groups=[group])
            with torch.no_grad(): logits, value = model(tensors(raw, 'cpu'))
            codes = data.codes[data.offsets[frame]:data.offsets[frame+1]].astype(np.int64)
            changed = mapping[codes & 511] | (codes & (7 << 9)) | (data.dirs[geo.N][group][(codes >> 12) & 3] << 12) | (codes & (7 << 14))
            for binary in binaries if group == 0 else binaries[:1]:
                out = json.loads(subprocess.run([binary, snapshot], input=inp, text=True, capture_output=True, check=True, timeout=10).stdout)
                index = {code: i for i, code in enumerate(out['codes'])}; assert set(index) == set(changed.tolist())
                order = [index[int(code)] for code in changed]
                values = dict(features=np.max(np.abs(np.array(out['x']).reshape(400, 40)-raw['x'][0].reshape(40, 400).T)),
                              actions=np.max(np.abs(np.array(out['features']).reshape(-1, 16)[order]-raw['features'][0])),
                              logits=np.max(np.abs(np.array(out['logits'])[order]-logits[0].numpy())), value=abs(out['value']-float(value[0])))
                for k, v in values.items(): errors[k] = max(errors[k], float(v))
                checks += 1
    assert errors['features'] < 2e-6 and errors['actions'] < 2e-6 and errors['logits'] < .003 and errors['value'] < .03, errors
    result = dict(passed=True, checks=checks, errors=errors, source_sha256=sha(source), model_sha256=sha(model_path),
                  preprocessing_outside_model_identical=True, completed_at=now())
    save(marker, result); return result


def evaluate(root, label, source, role='validation'):
    where = root/label/'evaluation'/role; where.mkdir(parents=True, exist_ok=True)
    marker = where/'result.json'
    if marker.exists():
        result = load(marker); assert result['source_sha256'] == sha(source); return result
    if role == 'final_in':
        cases = [dict(index=i, filename=p.name, path=str(p), sha256=sha(p)) for i, p in enumerate(sorted((ROOT/'tools/in').glob('*.txt')))]
    else:
        cases = [dict(c, filename=f"{c['index']:06d}.txt") for c in load(BC_RUN/'input_manifest.json') if c['role'] == role]
    assert len(cases) == (100 if role == 'final_in' else 256)
    inputs = where/'inputs'; inputs.mkdir(exist_ok=True)
    for case in cases:
        original = Path(case['path']) if role == 'final_in' else BC_RUN/case['path']
        assert sha(original) == case['sha256']; shutil.copy2(original, inputs/case['filename'])
    save(where/'input_manifest.json', cases)
    name = f'v112_{label}_{role}'; wrapper = ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag = f'v112_assisted_{label}_{role}_studio_j12'; scratch = ROOT/'results/out'/name
    started = where/'started.json'; identity = dict(source_sha256=sha(source), label=tag, jobs=12, manifest_sha256=sha(where/'input_manifest.json'))
    if not started.exists():
        assert not scratch.exists(), scratch
        save(started, identity); status(root/'pipeline', 'evaluating', condition=label, role=role, pid=os.getpid())
        with (where/'eval.log').open('w') as stream:
            proc = subprocess.run([sys.executable, ROOT/'scripts/eval.py', name, inputs, '-j', '12', '--wait-lock', '--label', tag],
                                  cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
        save(where/'exit.json', dict(exit_code=proc.returncode, finished_at=now()))
    else: assert load(started) == identity
    assert load(where/'exit.json')['exit_code'] == 0, 'inspect partial evaluation before resuming'
    records = []
    with (ROOT/'results/eval_records.jsonl').open() as stream:
        for line in stream:
            row = json.loads(line)
            if row['label'] == tag: records.append(row)
    assert len(records) == len({r['case_name'] for r in records}) == len(cases)
    assert len({r['run_id'] for r in records}) == 1
    save(where/'official_records.json', records)
    outputs = where/'outputs'
    if not outputs.exists(): shutil.copytree(scratch, outputs)
    indexed = {r['case_name']: r for r in records}; rows = []
    for case in cases:
        name = case['filename']; record = indexed[name]; assert record['status'] == 'ok' and record['local']
        result = validated_output(inputs/name, (outputs/name).read_text()); assert result['S'] == record['score']
        stderr = (outputs/(name+'.err')).read_text(); counts = trace(stderr)
        custom = {k: float(v) for k, v in re.findall(r'\b(v31[12]_\w+|pre_lns|lns_attempts)=(-?\d+(?:\.\d*)?(?:e[+-]?\d+)?)', stderr)}
        # 共通集計の3項目はNN構築の診断。DPで完成した出力の成否とは分ける。
        for key, diagnostic in (('inferences', 'v312_inferences'), ('depth', 'v312_nn_steps'), ('deadline', 'v312_nn_deadline')):
            counts.setdefault(key, int(custom.get(diagnostic, 0)))
        rows.append(dict(case=case['index'], filename=name, elapsed_ms=record['elapsed'], trace=counts, construction=custom, **result))
    result = dict(metrics=statistics(rows), rows=rows, source_sha256=sha(source), all_legal=True, completed_at=now())
    save(marker, result); return result



def execute(root):
    lock=(root/'assessment.lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (root/'final/result.json').exists():return
    training=load(root/'training/result.json')
    controls=load(CONTROL/'result.json')
    model_path=root/'training/model.json'
    source=with_model(root,model_path)
    numerical_result=numerical(root,'learned',source,model_path)
    current=evaluate(root,'learned',source)
    differences={name:comparison(current,controls['validation'][name]) for name in {'base',controls['selected']}}
    accepted=(training['iterations']==32 and valid(current) and
              all(row['mean_T_difference']<0 and row['mean_S_difference']<0 for row in differences.values()))
    result=dict(accepted=accepted,required_iterations=32,actual_iterations=training['iterations'],
                validation=current,differences=differences,numerical=numerical_result,
                control_selection=controls['selected'],model_sha256=sha(model_path),
                source=str(source.relative_to(ROOT)),source_sha256=sha(source),fixed_at=now())
    save(root/'assessment.json',result)
    final=root/'final';final.mkdir(exist_ok=True)
    if accepted:
        target=ROOT/'src/bin/v112_lns_mean.cpp'
        content='// '+target.name+'\n'+source.read_text().split('\n',1)[1]
        if target.exists():assert target.read_text()==content
        else:target.write_text(content)
        candidate=dict(source=str(target.relative_to(ROOT)),source_sha256=sha(target),
                       model_sha256=sha(model_path),fixed_at=result['fixed_at'])
        save(final/'candidate.json',candidate)
        test=evaluate(root,'final',target,'test');official=evaluate(root,'final',target,'final_in')
        output=dict(accepted=True,candidate=candidate,test=test,tools_in=official,completed_at=now())
    else:
        output=dict(accepted=False,retained=controls['final']['candidate'],heldout_evaluated=False,
                    reason='registered training budget or validation criteria not satisfied',completed_at=now())
    save(final/'result.json',output)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);args=p.parse_args()
    torch.set_num_threads(2);execute(args.run.resolve())
