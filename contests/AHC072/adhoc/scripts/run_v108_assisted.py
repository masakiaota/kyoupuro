#!/usr/bin/env python3
"""保存した方策の重みだけを替え、同じ配送DPとLNSで比較する。"""
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

RUN = ROOT/'results/nn_rank/v108/20261004_assisted_weights_studio'
REVISION = '19878a5'
PARENT_SHA = '343ea578a27141f817722255c696b76510b59b59225b3d3619dda7826ab49b08'
MODELS = {
    'base': ROOT/'results/nn_rank/v105/20261004_nn_lns_studio/training/model.json',
    'small': ROOT/'results/nn_rank/v107/20261004_capacity_studio/diagnostic/small/training/model.json',
    'wide': ROOT/'results/nn_rank/v107/20261004_capacity_studio/diagnostic/wide/training/model.json',
}


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
    out = ROOT/'adhoc/bin'/f'v108_assisted_{label}.cpp'
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
    assert len(text.encode()) <= 512000
    if out.exists(): assert out.read_text() == text
    else: out.write_text(text)
    save(root/label/'storage.json', storage)
    return out


def prepare(root):
    marker = root/'config.json'
    if marker.exists():
        config = load(marker)
        for item in config['conditions'].values():
            assert sha(ROOT/item['source']) == item['source_sha256']
            if 'model' in item: assert sha(ROOT/item['model']) == item['model_sha256']
        return config
    frozen = root/'frozen'; frozen.mkdir(parents=True, exist_ok=True)
    for version, name in (('v312', 'v312_certified_neural_constructor'), ('v311', 'v311_neural_multistart')):
        raw = subprocess.check_output(['git', 'show', f'{REVISION}:src/bin/{name}.cpp'], cwd=ROOT)
        (frozen/(version+'.cpp')).write_bytes(raw)
        note = ROOT/'notes/experiments'/f'{version}.md'
        if not note.exists():
            note.write_bytes(subprocess.check_output(['git', 'show', f'{REVISION}:notes/experiments/{version}.md'], cwd=ROOT))
    assert sha(frozen/'v312.cpp') == PARENT_SHA
    conditions = {}
    for label, model in MODELS.items():
        where = root/label/'training'; where.mkdir(parents=True, exist_ok=True)
        target = where/'model.json'; shutil.copy2(model, target)
        if label == 'base':
            source = ROOT/'adhoc/bin/v108_assisted_base.cpp'
            source.write_text('// '+source.name+'\n'+(frozen/'v312.cpp').read_text().split('\n', 1)[1])
            # 基準版は親のpacked値をそのまま保持。出力順を含め再量子化が一致するかだけ確認する。
            probe = root/'base/packed_check.cpp'; build(target, probe)
            def packed(s): return s.split('static constexpr char nn_packed[] =', 1)[1].split(';', 1)[0]
            assert packed(probe.read_text()) == packed(source.read_text())
        else: source = with_model(root, label, target)
        conditions[label] = dict(source=str(source.relative_to(ROOT)), source_sha256=sha(source),
                                model=str(target.relative_to(ROOT)), model_sha256=sha(target), spec=load(target)['spec'])
    source = ROOT/'adhoc/bin/v108_multistart_control.cpp'
    source.write_text('// '+source.name+'\n'+(frozen/'v311.cpp').read_text().split('\n', 1)[1])
    conditions['multistart'] = dict(source=str(source.relative_to(ROOT)), source_sha256=sha(source))
    cases = [dict(c, filename=f"{c['index']:06d}.txt") for c in load(BC_RUN/'input_manifest.json') if c['role'] == 'validation']
    assert len(cases) == 256
    save(root/'input_manifest.json', cases)
    config = dict(conditions=conditions, revision=subprocess.check_output(['git', 'rev-parse', REVISION], text=True).strip(),
                  parent_sha256=PARENT_SHA, jobs=12, input_manifest_sha256=sha(root/'input_manifest.json'), created_at=now())
    save(marker, config)
    return config


def numerical(root, label, source, model_path):
    where = root/label/'mechanism'; where.mkdir(parents=True, exist_ok=True)
    marker = where/'result.json'
    if marker.exists():
        result = load(marker); assert result['source_sha256'] == sha(source)
        return result
    checker = ROOT/'adhoc/bin'/f'check_v108_{label}.cpp'
    checker.write_text(f'''// {checker.name}
#define V090_DIAGNOSTIC
#define main v108_submission_main
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
    name = f'v108_{label}_{role}'; wrapper = ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag = f'v108_assisted_{label}_{role}_studio_j12'; scratch = ROOT/'results/out'/name
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
    root.mkdir(parents=True, exist_ok=True); pipe = root/'pipeline'; pipe.mkdir(exist_ok=True)
    lock = (pipe/'lock').open('a'); fcntl.flock(lock, fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (pipe/'exit.json').exists() and load(pipe/'exit.json')['exit_code'] == 0: return
    started = time.monotonic()
    try:
        config = prepare(root); results = {}
        paths = [Path(__file__), ROOT/'notes/experiments/v108.md'] + [ROOT/'adhoc/scripts'/x for x in (
            'v107_capacity.py', 'train_v107_capacity.py', 'build_v090_board.py', 'v101_compute.py',
            'v091_env.py', 'v090_data.py', 'v089_data.py', 'train_v090_board.py', 'check_v089_board.py',
            'run_v089_evaluation.py', 'run_v105_hybrid.py', 'v089_core.cpp.txt', 'v090_inference.cpp.txt',
            'v090_search.cpp.txt', 'memory_guard.py')] + [ROOT/'scripts/eval.py', ROOT/'scripts/build_solver.sh']
        identity = {str(p.relative_to(ROOT)): sha(p) for p in paths}
        if (pipe/'sources.json').exists(): assert load(pipe/'sources.json') == identity
        else:
            save(pipe/'sources.json', identity)
            for path in paths:
                target = pipe/'frozen'/path.relative_to(ROOT); target.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(path, target)
        for label in ('base', 'small', 'wide', 'multistart'):
            item = config['conditions'][label]; source = ROOT/item['source']
            status(pipe, 'mechanism', condition=label, pid=os.getpid())
            if 'model' in item: numerical(root, label, source, ROOT/item['model'])
            else:
                for local in (True, False):
                    mode = 'local' if local else 'judge'; where = root/label; where.mkdir(exist_ok=True)
                    compile_binary(source.stem, where/('solver_'+mode), local)
                    current = preprocess(source, where/(mode+'.ii'), local)
                    original = preprocess(root/'frozen/v311.cpp', where/('parent_'+mode+'.ii'), local)
                    assert without_model(current) == without_model(original)
        # 全条件の機構確認が終わってから成績を比較する。
        for label in ('base', 'small', 'wide', 'multistart'):
            item = config['conditions'][label]
            results[label] = evaluate(root, label, ROOT/item['source'])
        differences = {k: comparison(v, results['base']) for k, v in results.items() if k != 'base'}
        eligible = [k for k in ('small', 'wide') if valid(results[k]) and valid(results['base']) and differences[k]['mean_T_difference'] <= -1]
        eligible.sort(key=lambda k: (results[k]['metrics']['mean_T_completed'], results[k]['metrics']['mean_elapsed_ms'], k == 'wide'))
        selected = eligible[0] if eligible else None
        diagnostic = dict(selected_capacity=selected, comparisons=differences, metrics={k: v['metrics'] for k, v in results.items()},
                          structure={k: valid(v) for k, v in results.items()}, completed_at=now())
        save(root/'diagnostic/result.json', diagnostic); print(json.dumps(diagnostic), flush=True)
        main = None
        if selected:
            from v107_capacity import prepare as prepare_teachers
            prepare_teachers(root, 'main')
            where = root/'main'/selected; status(pipe, 'main_training', capacity=selected, pid=os.getpid())
            if not (where/'training/result.json').exists():
                with (pipe/'training.log').open('a') as stream:
                    proc = subprocess.run([sys.executable, ROOT/'adhoc/scripts/train_v107_capacity.py', '--run', where,
                        '--data', root/'main/data', '--capacity', selected, '--gpu-state', root/'gpu_state.json'], cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
                assert proc.returncode == 0, 'main learning interrupted; resume from checkpoint'
            learned = load(where/'training/result.json'); main = dict(complete_budget=learned['complete_budget'], adopted=False)
            if learned['complete_budget']:
                model = where/'training/model.json'; source = with_model(root, 'main', model)
                numerical(root, 'main', source, model); evaluated = evaluate(root, 'main', source)
                diffs = {k: comparison(evaluated, results[k]) for k in ('base', 'multistart')}
                accepted = valid(evaluated) and all(x['mean_T_difference'] < 0 and x['mean_S_difference'] < 0 for x in diffs.values())
                main.update(adopted=accepted, comparison=diffs, metrics=evaluated['metrics'])
                if accepted:
                    final = root/'final'; final.mkdir(exist_ok=True); submission = ROOT/'src/bin/v108_nn_assisted.cpp'
                    submission.write_text('// '+submission.name+'\n'+source.read_text().split('\n', 1)[1])
                    candidate = dict(source=str(submission.relative_to(ROOT)), source_sha256=sha(submission), model_sha256=sha(model), frozen_at=now())
                    if not (final/'candidate.json').exists(): save(final/'candidate.json', candidate)
                    else: assert load(final/'candidate.json')['source_sha256'] == sha(submission)
                    shutil.copy2(submission, final/submission.name)
                    # 先頭コメント以外は検証した単一ファイルと同じ。固定後の結果で再選択しない。
                    test = evaluate(root, 'main', source, 'test'); inputs = evaluate(root, 'main', source, 'final_in')
                    save(final/'result.json', dict(candidate=load(final/'candidate.json'), test=test, final_in=inputs, completed_at=now()))
        save(root/'result.json', dict(diagnostic=diagnostic, main=main, completed_at=now()))
        save(pipe/'exit.json', dict(exit_code=0, seconds=time.monotonic()-started, finished_at=now()))
        status(pipe, 'completed', selected_capacity=selected, main_adopted=main and main['adopted'])
    except BaseException as error:
        save(pipe/'exit.json', dict(exit_code=1, error=repr(error), traceback=traceback.format_exc(), finished_at=now()))
        status(pipe, 'failed', error=repr(error)); raise


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN); a = p.parse_args()
    torch.set_num_threads(2); torch.set_num_interop_threads(2)
    execute(a.run.resolve())
