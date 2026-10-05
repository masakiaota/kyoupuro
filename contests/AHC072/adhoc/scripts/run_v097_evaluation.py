#!/usr/bin/env python3
"""終了時のPPO重みを1回ずつ測り、保存済みBCと入力ごとに比較する。"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys

import numpy as np

from check_v089_board import compile_binary, trace
from run_v089_evaluation import validated_output, statistics
from v091_env import load, save, sha, now
from v097_policy import ROOT, BC_RUN, RUN, PARENT


def evaluate(root, role, condition, cases, phase):
    directory = root / 'evaluation' / role / condition; directory.mkdir(parents=True, exist_ok=True)
    source = ROOT / ('adhoc/bin/v097_flat.cpp' if condition == 'flat' else 'src/bin/v097_nn_factor.cpp'); marker = directory / 'result.json'
    if marker.exists():
        result = load(marker); assert result['solver_sha256'] == sha(source); return result
    name = f'v097_{role}_{condition}'
    wrapper = ROOT / 'adhoc/bin' / f'{name}.cpp'
    wrapper.write_text(f'// {name}.cpp\n' + ('#define V097_TOP2\n' if condition == 'top2' else '')
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
    label = f'{root.name}_v097_{role}_{condition}'; started = directory / 'started.json'
    identity = dict(label=label, name=name, solver_sha256=sha(source), model_sha256=sha(root / ('flat/model.json' if condition == 'flat' else 'factor/model.json')),
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


def execute(root):
    assert load(root/'numerical/flat/result.json')['passed'] and load(root/'numerical/factor/result.json')['passed']
    cases=[dict(c,filename=f"{c['index']:06d}.txt") for c in load(BC_RUN/'input_manifest.json') if c['role']=='validation']
    baseline=evaluate(root,'validation','flat',cases,'flat');reports={}
    for condition in ('greedy','top2'):
        current=evaluate(root,'validation',condition,cases,'factor');comparison=compare(current,baseline)
        difference=comparison['mean_T_difference_on_common']
        comparison['promising_steps']=bool(comparison['completion_difference']>=0 and comparison['mean_S_difference_all']<0 and difference is not None and difference<0)
        comparison['elapsed_ratio']=current['metrics']['mean_elapsed_ms']/baseline['metrics']['mean_elapsed_ms']
        comparison['promising_cost']=bool(comparison['completion_difference']>=0 and difference is not None and difference<=0 and comparison['elapsed_ratio']<=.90)
        reports[condition]=comparison
    result=dict(comparisons=reports,common_training_step=load(root/'selection.json')['step'],used_independent_test=False,used_tools_in=False,completed_at=now())
    save(root/'evaluation/comparison.json',result);print(json.dumps(result),flush=True);return result


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,default=RUN);a=p.parse_args();execute(a.run.resolve())
