#!/usr/bin/env python3
"""固定幅RRTを機構確認し、未使用256入力と採用時のinだけを評価する。"""
import difflib
import fcntl
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import sys
import time
import traceback

from check_v089_board import compile_binary, trace
from check_v113_integrated import block, normalize
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output, statistics
from v089_data import ROOT, now, save, sha, status
from v090_data import RUN as BC_RUN
from v111_portfolio import comparison, valid

RUN = ROOT / 'results/nn_rank/v130/20261004_rrt_studio'
PREVIOUS = ROOT / 'results/nn_rank/v129/20261004_structure_allocation_studio'
BASE = ROOT / 'src/bin/v113_integrated_nn_lns.cpp'
SOURCE = ROOT / 'src/bin/v130_record_to_record.cpp'
BASE_SHA = '037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'


def load(path):
    return json.loads(path.read_text())


def replace_once(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new, 1)


def build():
    assert sha(BASE) == BASE_SHA
    original = BASE.read_text()
    text = '// ' + SOURCE.name + '\n' + original.split('\n', 1)[1]
    text = replace_once(text,
        'constexpr double lns_start_temperature = 0.70;\nconstexpr double lns_end_temperature = 0.12;',
        '''// 最良記録から2手以内を探索中ずっと許し、最良列自体は別に保持する。
constexpr int lns_record_tolerance = 2;
static bool rrt_accept(size_t candidate_length, size_t best_length) {
    return candidate_length <= best_length + lns_record_tolerance;
}''')
    text = replace_once(text, 'static_assert(lns_end_temperature < lns_start_temperature);\n', '')
    text = replace_once(text, '''        // 切り替え時の再加熱を避け、全候補を同じ全体進捗で冷却する。
        const double initial_progress=clamp((time_keeper.exact_elapsed_sec()-lns_start)/max(PROGRAM_TIME_LIMIT_SEC*(0.02/1.90),end-lns_start),0.0,1.0);
        double temperature=lns_start_temperature*pow(lns_end_temperature/lns_start_temperature,initial_progress);
''', '')
    text = replace_once(text, '''            if((iteration&31)==1) {
                double progress=clamp((time_keeper.exact_elapsed_sec()-lns_start)/max(LOCAL_SECONDS(0.02),end-lns_start),0.0,1.0);
                temperature=lns_start_temperature*pow(lns_end_temperature/lns_start_temperature,progress);
            }
''', '')
    text = replace_once(text,
        'auto probability=[&](size_t n){int d=int(n)-int(current.size());return d<=0?1.0:(d<=2&&n<=best.size()+4?exp(-d/temperature):0.0);};',
        'auto probability=[&](size_t n){return rrt_accept(n,best.size())?1.0:0.0;};')
    text = replace_once(text,
        '            bool accept=delta<=0||(delta<=2&&polished.size()<=best.size()+4&&rng.unit()<exp(-delta/temperature));',
        '''            // 同じ候補に到達するまでの乱数消費は親の受理判定と揃える。
            if(delta>0&&delta<=2&&polished.size()<=best.size()+4)(void)rng.unit();
            const bool accept=rrt_accept(polished.size(),best.size());
            LOCAL_NOTE(
                trace.count_by("rrt_considered",1);
                trace.count_by("rrt_accepted_above_record",accept&&polished.size()>best.size());
                trace.count_by("rrt_rejected_outside_band",!accept);
                assert(!accept||polished.size()<=best.size()+lns_record_tolerance);
            )''')
    assert 'temperature' not in text
    if SOURCE.exists():
        assert SOURCE.read_text() == text
    else:
        SOURCE.write_text(text)
    (RUN / 'source.patch').write_text(''.join(difflib.unified_diff(
        original.splitlines(True), text.splitlines(True), fromfile=BASE.name, tofile=SOURCE.name)))


def mechanism():
    where = RUN / 'mechanism'
    where.mkdir(exist_ok=True)
    marker = where / 'result.json'
    if marker.exists():
        result = load(marker)
        assert result['source_sha256'] == sha(SOURCE)
        return result
    status(RUN / 'pipeline', 'mechanism', pid=os.getpid())
    for local in (True, False):
        mode = 'local' if local else 'judge'
        compile_binary(SOURCE.stem, where / ('solver_' + mode), local)
        old = normalize(preprocess(BASE, where / ('base_' + mode + '.ii'), local))
        current = normalize(preprocess(SOURCE, where / (mode + '.ii'), local))
        (where / (mode + '.patch')).write_text(''.join(difflib.unified_diff(
            old.splitlines(True), current.splitlines(True), fromfile='v113', tofile='v130')))
        # 登録した受理と温度計算の範囲を除き、完全な前処理結果全体が同じ。
        aligned = current.replace(block(current, 'void optimize('), block(old, 'void optimize('), 1)
        aligned = replace_once(aligned, block(aligned, 'static bool rrt_accept('), '')
        aligned = replace_once(aligned, 'constexpr int lns_record_tolerance = 2;', '')
        for declaration in ('constexpr double lns_start_temperature = 0.70;',
                            'constexpr double lns_end_temperature = 0.12;',
                            'static_assert(lns_end_temperature < lns_start_temperature);'):
            old = replace_once(old, declaration, '')
        assert ' '.join(aligned.split()) == ' '.join(old.split()), mode
    cases = [c for c in load(BC_RUN / 'input_manifest.json')
             if c['role'] == 'train' and c['M'] >= 80][:4]
    assert len(cases) == 4
    reports = []
    for case in cases:
        path = BC_RUN / case['path']
        assert sha(path) == case['sha256']
        for local in (True, False):
            mode = 'local' if local else 'judge'
            began = time.monotonic()
            proc = subprocess.run([where / ('solver_' + mode)], input=path.read_text(),
                text=True, capture_output=True, check=True, timeout=10)
            checked = validated_output(path, proc.stdout)
            assert checked['E'] == 0
            counts = trace(proc.stderr)
            if local:
                assert counts['state_pool_free_at_end'] == 4
                assert counts.get('rrt_considered', 0) > 0
                assert counts.get('rrt_accepted_above_record', 0) > 0
                assert all(counts.get(k, 0) == 0 for k in (
                    'lns_errors', 'construction_errors', 'lns_invalid_candidates',
                    'baseline_recovery', 'final_recovery'))
            dest = where / f"{case['index']}_{mode}"
            dest.mkdir(exist_ok=True)
            (dest / 'output.txt').write_text(proc.stdout)
            (dest / 'stderr.log').write_text(proc.stderr)
            reports.append(dict(case=case['index'], mode=mode, seconds=time.monotonic()-began,
                                trace=counts, **checked))
    assert sum(r['trace'].get('rrt_rejected_outside_band', 0) for r in reports) > 0
    result = dict(passed=True, source_sha256=sha(SOURCE), preprocessing_outside_change_identical=True,
                  reports=reports, completed_at=now())
    save(marker, result)
    return result


def inputs():
    marker = RUN / 'input_manifest.json'
    if marker.exists():
        return load(marker)
    assert load(PREVIOUS / 'result.json')['decision'] == 'diagnostic_failed'
    assert not (PREVIOUS / 'base/evaluation/validation/result.json').exists()
    assert not (PREVIOUS / 'learned/evaluation/validation/result.json').exists()
    cases = [dict(c) for c in load(PREVIOUS / 'input_manifest.json') if c['role'] == 'validation']
    assert len(cases) == len({c['sha256'] for c in cases}) == 256
    assert {c['index'] for c in cases} == set(range(512, 768))
    directory = RUN / 'inputs'
    directory.mkdir(exist_ok=True)
    for case in cases:
        original = PREVIOUS / case['path']
        assert sha(original) == case['sha256']
        target = directory / original.name
        shutil.copy2(original, target)
        case['path'] = str(target.relative_to(RUN))
    save(marker, cases)
    for name in ('in_reference.json', 'excluded_hashes.json'):
        shutil.copy2(PREVIOUS / name, RUN / name)
    return cases


def evaluate(label, source, role, cases=None):
    root = RUN
    where=root/label/'evaluation'/role;where.mkdir(parents=True,exist_ok=True)
    marker=where/'result.json'
    if marker.exists():
        result=load(marker);assert result['source_sha256']==sha(source);return result
    if role=='final_in':
        cases=[dict(index=i,filename=p.name,path=str(p),sha256=sha(p))
               for i,p in enumerate(sorted((ROOT/'tools/in').glob('*.txt')))]
    else:
        assert cases is not None
        cases=[dict(c,filename=f"{c['index']:04d}.txt") for c in cases]
    assert len(cases)>0
    if role=='final_in':assert len(cases)==100
    inputs=where/'inputs';inputs.mkdir(exist_ok=True)
    for case in cases:
        original=Path(case['path']) if role=='final_in' else RUN/case['path']
        assert sha(original)==case['sha256'];shutil.copy2(original,inputs/case['filename'])
    save(where/'input_manifest.json',cases)
    name=f'v130_{label}_{role}';wrapper=ROOT/'adhoc/bin'/(name+'.cpp')
    wrapper.write_text(f'// {name}.cpp\n#include "../../{source.relative_to(ROOT)}"\n')
    tag=f'v130_late_{label}_{role}_studio_j12';scratch=ROOT/'results/out'/name
    identity=dict(source_sha256=sha(source),label=tag,jobs=12,manifest_sha256=sha(where/'input_manifest.json'))
    started=where/'started.json'
    if not started.exists():
        assert not scratch.exists(),scratch
        save(started,identity);status(root/'pipeline','evaluating',condition=label,role=role,pid=os.getpid())
        with (where/'eval.log').open('w') as stream:
            proc=subprocess.run([sys.executable,ROOT/'scripts/eval.py',name,inputs,'-j','12','--wait-lock','--label',tag],
                                cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
        save(where/'exit.json',dict(exit_code=proc.returncode,finished_at=now()))
    else:
        assert load(started)==identity
    assert load(where/'exit.json')['exit_code']==0,'inspect partial evaluation before resuming'
    records=[]
    with (ROOT/'results/eval_records.jsonl').open() as stream:
        for line in stream:
            row=json.loads(line)
            if row['label']==tag:records.append(row)
    assert len(records)==len({r['case_name'] for r in records})==len(cases)
    assert len({r['run_id'] for r in records})==1
    save(where/'official_records.json',records)
    outputs=where/'outputs'
    if not outputs.exists():shutil.copytree(scratch,outputs)
    indexed={r['case_name']:r for r in records};rows=[]
    for case in cases:
        name=case['filename'];record=indexed[name]
        assert record['status']=='ok' and record['local']
        result=validated_output(inputs/name,(outputs/name).read_text());assert result['S']==record['score']
        stderr=(outputs/(name+'.err')).read_text();counts=trace(stderr)
        custom={}
        for key,value in re.findall(r'\b(v31\d_\w+|pre_lns|lns_attempts)=(-?\d+(?:\.\d*)?(?:e[+-]?\d+)?)',stderr):
            custom[key]=float(value) if any(c in value for c in '.e') else int(value)
        for key,diagnostic in (('inferences','v312_inferences'),('depth','v312_nn_steps'),('deadline','v312_nn_deadline')):
            counts.setdefault(key,int(custom.get(diagnostic,0)))
        rows.append(dict(case=case['index'],filename=name,elapsed_ms=record['elapsed'],trace=counts,construction=custom,**result))
    result=dict(metrics=statistics(rows),rows=rows,source_sha256=sha(source),all_legal=True,completed_at=now())
    save(marker,result);return result


def run():
    RUN.mkdir(parents=True, exist_ok=True)
    lock = (RUN / 'pipeline.lock').open('a')
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    if (RUN / 'pipeline/exit.json').exists() and load(RUN / 'pipeline/exit.json')['exit_code'] == 0:
        return
    began = time.monotonic()
    save(RUN / 'launch.json', dict(pid=os.getpid(), started_at=now()))
    try:
        build()
        cases = inputs()
        files = [Path(__file__)]
        identity = dict(source_sha256=sha(SOURCE), parent_sha256=sha(BASE), record_tolerance=2,
            jobs=12, input_manifest_sha256=sha(RUN / 'input_manifest.json'),
            scripts={p.name: sha(p) for p in files})
        if (RUN / 'config.json').exists():
            assert load(RUN / 'config.json') == identity
        else:
            save(RUN / 'config.json', identity)
            frozen = RUN / 'frozen'
            frozen.mkdir(exist_ok=True)
            for p in [BASE, SOURCE, *files]:
                shutil.copy2(p, frozen / p.name)
        checked = mechanism()
        base = evaluate('base', BASE, 'validation', cases)
        current = evaluate('rrt', SOURCE, 'validation', cases)
        difference = comparison(current, base)
        activation = {k: sum(r['trace'].get(k, 0) for r in current['rows']) for k in (
            'rrt_considered', 'rrt_accepted_above_record', 'rrt_rejected_outside_band')}
        assert all(v > 0 for v in activation.values())
        accepted = valid(base) and valid(current) and difference['mean_T_difference'] < 0 and difference['mean_S_difference'] < 0
        assessment = dict(accepted=accepted, difference=difference, activation=activation,
                          base_valid=valid(base), current_valid=valid(current), fixed_at=now())
        save(RUN / 'assessment.json', assessment)
        result = dict(assessment=assessment, validation=dict(base=base, rrt=current),
                      mechanism_sha256=sha(RUN / 'mechanism/result.json'))
        if accepted:
            candidate = dict(source=str(SOURCE.relative_to(ROOT)), source_sha256=sha(SOURCE),
                             fixed_at=now(), selection='unused_validation_256')
            save(RUN / 'final/candidate.json', candidate)
            final_in = evaluate('final', SOURCE, 'final_in')
            reference = load(RUN / 'in_reference.json')
            final = dict(candidate=candidate, tools_in=final_in, final_valid=valid(final_in),
                fixed_reference_relative=sum(reference['values'][r['filename']]/r['T'] for r in final_in['rows']),
                reference_sha256=sha(RUN / 'in_reference.json'), completed_at=now())
            save(RUN / 'final/result.json', final)
            result['final'] = final
        else:
            result['retained'] = dict(version='v113', source=str(BASE.relative_to(ROOT)), sha256=sha(BASE))
        result['completed_at'] = now()
        save(RUN / 'result.json', result)
        status(RUN / 'pipeline', 'completed', accepted=accepted)
        save(RUN / 'pipeline/exit.json', dict(exit_code=0, seconds=time.monotonic()-began, completed_at=now()))
        print(json.dumps(assessment, ensure_ascii=False), flush=True)
    except BaseException as e:
        status(RUN / 'pipeline', 'failed', error=repr(e))
        save(RUN / 'pipeline/exit.json', dict(exit_code=1, error=repr(e), traceback=traceback.format_exc(), completed_at=now()))
        raise


if __name__ == '__main__':
    run()
