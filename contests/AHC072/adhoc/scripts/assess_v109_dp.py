#!/usr/bin/env python3
"""v108と同じ推論・補完・LNSへ学習済み重みだけを組み込む。"""
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


def function_span(text, name):
    start = text.index('void '+name+'(')
    left = text.index('{', start); depth = 1; end = left+1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}'); end += 1
    return start, end


def without_model(text):
    # 除外するのは容量・重み・デコードだけ。推論、方策、DP、LNSを照合に残す。
    if text.startswith('// '): text = text.split('\n', 1)[1]
    a, b = function_span(text, 'load_nn')
    text = text[:a]+'void load_nn() {}'+text[b:]
    a = text.index('constexpr int NN_WIDTH=')
    b = text.index('alignas(64) static array', a)
    text = text[:a]+'/* fixed model region */\n'+text[b:]
    text = re.sub(r'"[^"\n]*\.cpp"', '"source.cpp"', text)
    text = re.sub(r'("source.cpp",\s*)\d+', r'\g<1>0', text)
    return text


def with_model(root, label, model_path):
    parent = (root/'frozen/v312.cpp').read_text()
    out = ROOT/'adhoc/bin'/f'v109_assisted_{label}.cpp'
    temporary = root/label/'unoptimized_nn.cpp'; temporary.parent.mkdir(parents=True, exist_ok=True)
    storage = build(model_path, temporary); generated = temporary.read_text()
    declarations = generated[generated.index('constexpr int NN_WIDTH='):generated.index('void load_nn()')]
    a, b = function_span(generated, 'load_nn')
    decoder = generated[a:b]
    decoder = decoder[:-1]+'    prepare_fast_weights();\n}'
    a, b = function_span(parent, 'load_nn')
    text = parent[:a]+decoder+parent[b:]
    a = text.index('constexpr int NN_WIDTH='); b = text.index('// The trained row-major tensors', a)
    text = text[:a]+declarations+'\n'+text[b:]
    text = '// '+out.name+'\n'+text.split('\n', 1)[1]
    assert without_model(text) == without_model(parent), 'unexpected change outside model'
    # 派生元の「重み不変」という説明は今回の学習済み候補には当てはまらない。
    head = text.index('#include <bits/stdc++.h>')
    text = ('// '+out.name+'\n// v109: v312の推論・配送DP・LNSへ、実測補完費用で学習した重みを組み込む。\n'
            '// 学習条件と入力分離はnotes/experiments/v109.mdに記録。\n'+text[head:])
    assert len(text.encode()) <= 512000
    if out.exists(): assert out.read_text() == text
    else: out.write_text(text)
    save(root/label/'storage.json', storage)
    return out


def numerical(root, label, source, model_path):
    where = root/label/'mechanism'; where.mkdir(parents=True, exist_ok=True)
    marker = where/'result.json'
    if marker.exists():
        result = load(marker); assert result['source_sha256'] == sha(source)
        return result
    checker = ROOT/'adhoc/bin'/f'check_v109_{label}.cpp'
    checker.write_text(f'''// {checker.name}
#define V090_DIAGNOSTIC
#define main v109_submission_main
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
        parent_path = root/'base/mechanism'/(mode+'.ii')
        if label == 'base':
            parent = preprocess(root/'frozen/v312.cpp', where/('parent_'+mode+'.ii'), local)
        else: parent = parent_path.read_text()
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
    name = f'v109_{label}_{role}'; wrapper = ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag = f'v109_assisted_{label}_{role}_studio_j12'; scratch = ROOT/'results/out'/name
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


def comparison(current, before):
    old = {r['case']: r for r in before['rows']}; assert set(old) == {r['case'] for r in current['rows']}
    pairs = [(old[r['case']], r) for r in current['rows'] if r['E'] == old[r['case']]['E'] == 0]
    return dict(common_completed=len(pairs), mean_T_difference=float(np.mean([b['T']-a['T'] for a, b in pairs])) if pairs else None,
                mean_S_difference=current['metrics']['mean_S_all']-before['metrics']['mean_S_all'],
                wins=sum(b['T']<a['T'] for a, b in pairs), ties=sum(b['T']==a['T'] for a, b in pairs), losses=sum(b['T']>a['T'] for a, b in pairs))


def valid(result):
    return result['metrics']['complete'] == len(result['rows']) and result['metrics']['max_elapsed_ms'] <= 2000 and all(
        r['trace'].get('state_pool_free_at_end') == 4 and r['trace'].get('lns_attempts', 0) > 0 and
        all(r['trace'].get(k, 0) == 0 for k in ('construction_errors', 'lns_errors', 'baseline_recovery', 'final_recovery', 'lns_invalid_candidates'))
        for r in result['rows'])


def execute(root):
    previous = ROOT/'results/nn_rank/v108/20261004_assisted_weights_studio'
    marker = root/'assessment.json'
    if marker.exists(): return load(marker)
    model_path = root/'training/model.json'
    (root/'frozen').mkdir(exist_ok=True)
    original = previous/'frozen/v312.cpp'
    assert sha(original) == '343ea578a27141f817722255c696b76510b59b59225b3d3619dda7826ab49b08'
    shutil.copy2(original, root/'frozen/v312.cpp')
    baseline = root/'base/mechanism'; baseline.mkdir(parents=True, exist_ok=True)
    for mode in ('local', 'judge'):
        shutil.copy2(previous/'base/mechanism'/(mode+'.ii'), baseline/(mode+'.ii'))
    source = with_model(root, 'learned', model_path)
    numerical_result = numerical(root, 'learned', source, model_path)
    result = evaluate(root, 'learned', source)
    base = load(previous/'base/evaluation/validation/result.json')
    multistart = load(previous/'multistart/evaluation/validation/result.json')
    difference = comparison(result, base); control = comparison(result, multistart)
    complete_budget = load(root/'training/result.json')['iterations'] == 240
    supported = complete_budget and valid(result) and difference['mean_T_difference'] < 0 and difference['mean_S_difference'] < 0
    accepted = supported and control['mean_T_difference'] < 0 and control['mean_S_difference'] < 0
    assessment = dict(accepted=accepted, learning_supported=supported, complete_budget=complete_budget,
                      numerical=numerical_result, validation=result, versus_same_solver=difference,
                      versus_multistart=control, source=str(source.relative_to(ROOT)), source_sha256=sha(source),
                      checkpoint_sha256=sha(root/'training/latest.pt'), model_sha256=sha(model_path), fixed_at=now())
    save(marker, assessment)
    return assessment


def final(root, assessment):
    directory = root/'final'; directory.mkdir(exist_ok=True)
    marker = directory/'result.json'
    if marker.exists(): return load(marker)
    if not assessment['accepted']:
        result = dict(accepted=False, retained='v108 original weights and multistart control',
                      heldout_evaluated=False, reason='preregistered adoption condition not met', completed_at=now())
        save(marker, result); return result
    old_source = ROOT/assessment['source']; assert sha(old_source) == assessment['source_sha256']
    source = ROOT/'src/bin/v109_nn_dp_reward.cpp'
    text = '// '+source.name+'\n'+old_source.read_text().split('\n', 1)[1]
    if source.exists(): assert source.read_text() == text
    else: source.write_text(text)
    candidate = dict(source=str(source.relative_to(ROOT)), source_sha256=sha(source),
                     checkpoint_sha256=assessment['checkpoint_sha256'], model_sha256=assessment['model_sha256'],
                     fixed_at=assessment['fixed_at'])
    if (directory/'candidate.json').exists(): assert load(directory/'candidate.json') == candidate
    else: save(directory/'candidate.json', candidate)
    # 条件選択に使わず、候補を固定してから既存の比較集合を各1回測る。
    test = evaluate(root, 'final', source, 'test')
    tools_in = evaluate(root, 'final', source, 'final_in')
    result = dict(accepted=True, candidate=candidate, test=test, tools_in=tools_in, completed_at=now())
    save(marker, result); return result


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, required=True); a = p.parse_args()
    torch.set_num_threads(2); root = a.run.resolve()
    assessment = execute(root); final(root, assessment)
