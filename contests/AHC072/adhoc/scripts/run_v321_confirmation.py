#!/usr/bin/env python3
"""固定済みv321の新規確認3版と最終Inを順番に1回ずつ実行する。"""
import collections
import csv
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

from train_v321_router import comparison

ROOT = Path(__file__).resolve().parents[2]


def now():
    return datetime.datetime.now().astimezone().isoformat()


def save(path, value):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2)+'\n')
    temporary.replace(path)


def analyze(run, stage):
    manifest = json.loads((run/'manifest.json').read_text())
    config = manifest['stages'][stage]
    state = json.loads((run/f'{stage}_state.json').read_text())
    assert state['status'] == 'completed' and state['exit_code'] == 0
    hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
              for p in (ROOT/config['input_dir']).glob('*.txt')}
    assert hashes == manifest['input_sha256'][config['dataset']]
    assert hashlib.sha256((ROOT/'src/bin'/f"{config['bin']}.cpp").read_bytes()).hexdigest() == config['source_sha256']
    records = [json.loads(line) for line in (ROOT/'results/eval_records.jsonl').open()]
    current = [r for r in records if r.get('label') == config['label']]
    assert len(current) == len(hashes) and {r['case_name'] for r in current} == hashes.keys()
    assert all(r['status'] == 'ok' and r['bin'] == config['bin'] and r['local']
               and r['input_dir'] == config['input_dir'] for r in current)
    errors = collections.Counter()
    mechanism = collections.Counter()
    home = complete = 0
    for row in current:
        output = run/f'{stage}_outputs'/row['case_name']
        stderr = output.with_name(output.name+'.err').read_text()
        counts = {key: int(value) for key, value in re.findall(
            r'^\[summary.count\] (\S+)=(-?\d+)$', stderr, re.M)}
        T = sum(bool(line.strip()) for line in output.read_text().splitlines())
        assert row['score'] == T+100000*counts['E'] and counts['final_ops'] == T
        assert counts['state_pool_free_at_end'] == 4
        home += counts['E'] == 0
        complete += counts['nn_complete'] == 1
        for key in ('construction_errors', 'lns_errors', 'baseline_recovery', 'final_recovery'):
            errors[key] += counts[key]
        for key, value in counts.items():
            if key.startswith('cnn_'):
                mechanism[key] += value
        detail = {key: float(value) for key, value in re.findall(
            r'\b(v(?:311|315|318)_\w+)=(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)', stderr)}
        mechanism['multiple_seed_cases'] += detail['v311_nn_retained'] > 1
        mechanism['additional_nn_incomplete'] += int(detail['v311_nn_incomplete'])
        mechanism['racing_active_cases'] += detail.get('v315_slices', 0) > 0
        mechanism['history_active_cases'] += detail.get('v318_history_queries', 0) > 0
    scores = {r['case_name']: r['score'] for r in current}
    names = sorted(scores)
    baselines = {}
    if config['dataset'] == 'confirm200':
        for previous, c in manifest['stages'].items():
            if c['dataset'] != 'confirm200':
                continue
            selected = [r for r in records if r.get('label') == c['label']]
            if len(selected) == len(names) and all(r['status'] == 'ok' for r in selected):
                baselines[c['expert']] = {r['case_name']: r['score'] for r in selected}
        reference = baselines['v111']
    else:
        for v, label in manifest['baseline_labels']['in'].items():
            selected = [r for r in records if r.get('label') == label and r['bin'] == manifest['candidates'][v]]
            assert len(selected) == len(names) and all(r['status'] == 'ok' for r in selected)
            baselines[v] = {r['case_name']: r['score'] for r in selected}
        reference = manifest['reference']['in']
    assert all(values.keys() == scores.keys() for values in baselines.values())
    result = {'stage': stage, 'label': config['label'], 'source_sha256': config['source_sha256'],
              'count': len(current), 'sum': sum(scores.values()), 'mean': sum(scores.values())/len(scores),
              'jobs': config['jobs'], 'local_time_ratio': manifest['local_time_ratio'],
              'all_home': home == len(current), 'all_initial_nn_complete': complete == len(current),
              'error_counts': dict(errors), 'mechanism': dict(mechanism), 'scores': scores,
              'max_elapsed_ms': max(r['elapsed'] for r in current),
              'mean_elapsed_ms': sum(r['elapsed'] for r in current)/len(current),
              'comparisons': {v: comparison([scores[n] for n in names], [values[n] for n in names],
                                             [reference[n] for n in names]) for v, values in baselines.items()}}
    result['quality_passed'] = result['all_home'] and result['all_initial_nn_complete'] and not any(errors.values()) and result['max_elapsed_ms'] <= 2000
    summary = [r for r in csv.DictReader((ROOT/'results/score_summary.csv').open()) if r['label'] == config['label']]
    assert len(summary) == 1 and int(summary[0]['total_sum']) == result['sum']
    result['csv_rounded_mean'] = summary[0]['total_avg']
    save(run/f'{stage}_result.json', result)
    with (run/f'{stage}_paired.csv').open('w') as f:
        writer = csv.writer(f)
        writer.writerow(['case', 'current', *baselines, 'reference'])
        for name in names:
            writer.writerow([name, scores[name], *(values[name] for values in baselines.values()), reference[name]])
    return result


def main():
    run = Path(sys.argv[1]).resolve()
    if len(sys.argv) == 4 and sys.argv[2] == 'analyze':
        result = analyze(run, sys.argv[3])
        print(json.dumps({k: v for k, v in result.items() if k != 'scores'}, ensure_ascii=False))
        return
    manifest = json.loads((run/'manifest.json').read_text())
    assert manifest['first_push_commit'] == manifest['first_push_verified_remote_sha']
    check = json.loads((run/'mechanism_check/result.json').read_text())
    assert check['body_equal_v113'] and len(check['modes']) == 2
    assert all(m['full_preprocessing_equal'] for m in check['modes'])
    state = {'status': 'running', 'started_at': now(), 'pid': os.getpid(), 'completed_stages': []}
    path = run/'pipeline_state.json'
    with path.open('x') as f:
        json.dump(state, f, ensure_ascii=False, indent=2)
    (run/'pipeline_pid.txt').write_text(str(os.getpid())+'\n')
    try:
        for stage, config in manifest['stages'].items():
            if state['completed_stages']:
                previous = json.loads((run/f"{state['completed_stages'][-1]}_state.json").read_text())
                state.update(active_stage=stage, phase='cooldown')
                save(path, state)
                remaining = previous['finished_epoch']+manifest['cooldown_seconds']-time.time()
                if remaining > 0:
                    time.sleep(remaining)
            assert hashlib.sha256((ROOT/'src/bin'/f"{config['bin']}.cpp").read_bytes()).hexdigest() == config['source_sha256']
            hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                      for p in (ROOT/config['input_dir']).glob('*.txt')}
            assert hashes == manifest['input_sha256'][config['dataset']]
            state.update(active_stage=stage, phase='evaluating')
            save(path, state)
            stage_state = {'status': 'running', 'pid': os.getpid(), 'stage': stage, 'label': config['label'],
                           'started_at': now(), 'started_epoch': time.time()}
            stage_path = run/f'{stage}_state.json'
            with stage_path.open('x') as f:
                json.dump(stage_state, f, ensure_ascii=False, indent=2)
            print(now(), 'START', stage, flush=True)
            with (run/f'{stage}.log').open('w') as log:
                completed = subprocess.run([sys.executable, 'scripts/eval.py', config['bin'], config['input_dir'],
                                            '-j', str(config['jobs']), '--wait-lock', '--label', config['label']],
                                           cwd=ROOT, stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
            stage_state.update(status='completed' if completed.returncode == 0 else 'failed',
                               exit_code=completed.returncode, finished_at=now(), finished_epoch=time.time())
            if completed.returncode == 0:
                shutil.copytree(ROOT/'results/out'/config['bin'], run/f'{stage}_outputs')
            save(stage_path, stage_state)
            assert completed.returncode == 0, f'{stage} failed: see its log'
            result = analyze(run, stage)
            print(now(), 'FINISHED', stage, json.dumps({k: result[k] for k in
                  ['count', 'sum', 'mean', 'max_elapsed_ms', 'quality_passed', 'comparisons']}, ensure_ascii=False), flush=True)
            state['completed_stages'].append(stage)
            save(path, state)
        # 同じ新規分子・全候補で比較表をそろえる。保存結果の読取のみ。
        for stage in manifest['stages']:
            analyze(run, stage)
        state.update(status='completed', phase='awaiting_notes_and_push', finished_at=now(), exit_code=0)
    except Exception as error:
        state.update(status='failed', error=repr(error), finished_at=now(), exit_code=1)
        raise
    finally:
        save(path, state)


if __name__ == '__main__':
    main()
