#!/usr/bin/env python3
"""時間予算を固定して独立入力の教師を増やす。完了した経路は再計算しない。"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import fcntl
import json
import os
from pathlib import Path
import random
import shutil
import signal
import subprocess
import sys
import time
import traceback

from run_v077_overnight import input_properties, prior_inputs
import run_v085_teacher as teacher
from v085_data import ROOT, Problem, now, save, sha, status

SOURCE = ROOT / 'results/nn_rank/v085/20261002T144801_studio'
RUN = ROOT / 'results/nn_rank/v090/20261003_scaling_studio'
COUNTS = {'train': 4096, 'validation': 256, 'test': 256}
SOURCES = ['adhoc/scripts/run_v090_teacher.py', 'adhoc/scripts/run_v085_teacher.py',
           'adhoc/scripts/v085_data.py', 'adhoc/scripts/run_v047_long_search.py',
           'adhoc/scripts/run_v077_overnight.py', 'adhoc/scripts/run_v079_experiment.py',
           'adhoc/scripts/memory_guard.py']


def read(path):
    return json.loads(path.read_text())


def copy_checked(source, destination, digest=None):
    if digest is not None and sha(source) != digest:
        raise RuntimeError(f'original artifact changed: {source}')
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    if sha(destination) != sha(source):
        raise RuntimeError(f'copy mismatch: {destination}')


def verify(run):
    config = read(run / 'config.json')
    assert (config['v800_seconds'], config['v801_rounds'], config['v801_round_seconds']) == (120, 2, 60)
    for name, digest in config['source_sha256'].items():
        if sha(ROOT / name) != digest:
            raise RuntimeError(f'teacher pipeline source changed: {name}')
    for name, digest in config['frozen_sha256'].items():
        if sha(run / 'frozen' / name) != digest:
            raise RuntimeError(f'frozen artifact changed: {name}')
    if sha(run / 'input_manifest.json') != config['manifest_sha256']:
        raise RuntimeError('input manifest changed')
    if sha(run / 'probe_manifest.json') != config['probe_manifest_sha256']:
        raise RuntimeError('probe manifest changed')
    manifest = read(run / 'input_manifest.json')
    assert {role: sum(c['role'] == role for c in manifest) for role in COUNTS} == COUNTS
    assert len({c['sha256'] for c in manifest}) == sum(COUNTS.values())
    assert [c['index'] for c in manifest] == list(range(len(manifest)))
    for item in manifest:
        assert sha(run / item['path']) == item['sha256']
    return config, manifest


def prepare(run, deadline):
    if (run / 'config.json').exists():
        verify(run)
        return
    # 生成途中のファイルを完成扱いにしない。再準備が必要なら元の記録を先に確認する。
    if (run / 'frozen').exists():
        raise RuntimeError('partial preparation exists; inspect it before restarting preparation')
    status(run, 'preparing')
    original = read(SOURCE / 'config.json')
    (run / 'frozen').mkdir()
    for name in ('baseline', 'v800', 'v800_nonlocal', 'v801', 'v801_nonlocal', 'gen', 'vis'):
        copy_checked(SOURCE / 'frozen' / name, run / 'frozen' / name,
                     original['frozen_sha256'][f'frozen/{name}'])
    for rel in SOURCES + ['notes/experiments/v090.md']:
        copy_checked(ROOT / rel, run / 'frozen' / rel)
    status(run, 'excluding_known_inputs')
    seeds, hashes = prior_inputs(run)
    manifest = []
    for item in read(SOURCE / 'input_manifest.json'):
        dest = run / 'inputs' / f"{item['index']:06d}.txt"
        copy_checked(SOURCE / item['path'], dest, item['sha256'])
        case = run / 'cases' / f"{item['index']:06d}"
        old_case = SOURCE / 'cases' / case.name
        previous = read(old_case / 'complete.json')
        assert previous['input_sha256'] == item['sha256']
        copy_checked(old_case / 'best.txt', case / 'best.txt', previous['best_sha256'])
        # 元の全経路を複製せず、正本の場所とハッシュを保持する。
        save(case / 'complete.json', {
            'input_sha256': item['sha256'], 'best_sha256': previous['best_sha256'],
            'T': previous['T'], 'E': 0, 'baseline_T': previous['baseline_T'],
            'source': 'v085_reused', 'original_case': str(old_case.relative_to(ROOT)),
            'original_complete_sha256': sha(old_case / 'complete.json'), 'completed_at': now()})
        manifest.append(dict(item, path=str(dest.relative_to(run)), origin='v085'))
        seeds.add(item['seed']); hashes.add(item['sha256'])
    assert len(manifest) == 512
    roles = ['train'] * 3712 + ['validation'] * 128 + ['test'] * 256
    random.Random(90001).shuffle(roles)
    seed = 970000000
    while len(manifest) < sum(COUNTS.values()):
        batch = []
        while len(batch) < sum(COUNTS.values()) - len(manifest):
            if seed not in seeds:
                batch.append(seed)
            seed += 1
        folder = run / 'generated' / str(batch[0])
        folder.mkdir(parents=True)
        seed_file = folder / 'seeds.txt'
        seed_file.write_text(''.join(f'{value}\n' for value in batch))
        teacher.command([run / 'frozen/gen', seed_file, '--dir', folder / 'inputs'],
                        run / 'generation.log', deadline)
        for i, value in enumerate(batch):
            inp = folder / 'inputs' / f'{i:04d}.txt'
            digest = sha(inp)
            seeds.add(value)
            if digest in hashes:
                continue
            hashes.add(digest)
            index = len(manifest)
            dest = run / 'inputs' / f'{index:06d}.txt'
            copy_checked(inp, dest, digest)
            props = input_properties(inp.read_bytes())
            assert props is not None
            manifest.append(dict(index=index, seed=value, sha256=digest, role=roles[index - 512],
                                 path=str(dest.relative_to(run)), origin='new', **props))
    assert {role: sum(c['role'] == role for c in manifest) for role in COUNTS} == COUNTS
    save(run / 'input_manifest.json', manifest)
    # 動作確認は新しい学習入力から取る。検証・保留入力を試運転へ使わない。
    candidates = [m for m in manifest if m['origin'] == 'new' and m['role'] == 'train']
    probes = [next(m for m in candidates if m['M'] < 80), next(m for m in candidates if m['M'] >= 80)]
    save(run / 'probe_manifest.json', probes)
    config = {
        'prepared_at': now(), 'source_run': str(SOURCE.relative_to(ROOT)),
        'source_config_sha256': sha(SOURCE / 'config.json'),
        'source_sha256': {p: sha(ROOT / p) for p in SOURCES},
        'frozen_sha256': {str(p.relative_to(run / 'frozen')): sha(p)
                          for p in (run / 'frozen').rglob('*') if p.is_file()},
        'manifest_sha256': sha(run / 'input_manifest.json'),
        'probe_manifest_sha256': sha(run / 'probe_manifest.json'),
        'roles': COUNTS, 'reused_teacher_inputs': 512, 'new_teacher_inputs': 3840,
        'baseline_workers': 20, 'teacher_workers': 30,
        'v800_seconds': 120, 'v801_rounds': 2, 'v801_round_seconds': 60,
        'maximum_seconds': 43200, 'memory_stop_bytes': 56000000000,
        'platform': subprocess.check_output(['uname', '-a'], text=True).strip(),
        'chip': subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
    }
    save(run / 'config.json', config)
    verify(run)
    status(run, 'prepared', roles=COUNTS, new_teacher_inputs=3840)


def unfinished_folders(run, items):
    folders = []
    for item in items:
        case = run / 'cases' / f"{item['index']:06d}"
        # 予算短縮前に始まった後半の再出発も、未完なら保存してから外す。
        candidates = [case / 'baseline']
        for mode in ('v800', 'v801'):
            candidates.extend(sorted((case / mode).glob('round_*')))
        for folder in candidates:
            if folder.exists() and not (folder / 'complete.json').exists():
                folders.append(folder)
    return folders


def archive_incomplete(run, items):
    folders = unfinished_folders(run, items)
    if not folders:
        return
    stamp = str(time.time_ns())
    moves = []
    for folder in folders:
        dest = run / 'interrupted' / stamp / folder.relative_to(run / 'cases')
        dest.parent.mkdir(parents=True, exist_ok=True)
        folder.rename(dest)
        moves.append({'old': str(folder.relative_to(run)), 'saved': str(dest.relative_to(run))})
    save(run / 'interrupted' / stamp / 'moves.json', moves)


def tasks(run, stage, work, function, workers, deadline):
    completed = 0
    started = time.monotonic()
    status(run, stage, total=len(work), completed=0, workers=workers)
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = {pool.submit(function, item): item for item in work}
        try:
            for future in as_completed(futures):
                future.result()
                completed += 1
                seconds = time.monotonic() - started
                remaining = seconds * (len(work) - completed) / completed
                status(run, stage, total=len(work), completed=completed, workers=workers,
                       elapsed_seconds=seconds, estimated_remaining_seconds=remaining)
                if time.time() >= deadline:
                    raise TimeoutError('registered time limit')
        except BaseException:
            teacher.STOP.set()
            for future in futures:
                future.cancel()
            raise


def branch(run, item, mode, deadline, config):
    folder = run / 'cases' / f"{item['index']:06d}"
    initial = folder / 'baseline/output.txt'
    count = 1 if mode == 'v800' else config['v801_rounds']
    seconds = config['v800_seconds'] if mode == 'v800' else config['v801_round_seconds']
    for round_id in range(count):
        teacher.round_once(run, item, mode, round_id, initial, seconds, deadline)
        initial = folder / mode / f'round_{round_id:02d}' / 'output.txt'


def finish_cases(run, items, config):
    for item in items:
        folder = run / 'cases' / f"{item['index']:06d}"
        if (folder / 'complete.json').exists():
            continue
        last = config['v801_rounds'] - 1
        reports = [folder / 'baseline/complete.json', folder / 'v800/round_00/complete.json']
        reports += [folder / f'v801/round_{i:02d}/complete.json' for i in range(last + 1)]
        if not all(p.exists() for p in reports):
            raise RuntimeError(f'teacher rounds are not complete: {folder}')
        audits = [read(p) for p in reports]
        paths = [folder / 'baseline/output.txt', folder / 'v800/round_00/output.txt',
                 folder / f'v801/round_{last:02d}/output.txt']
        lengths = [len(p.read_text().splitlines()) for p in paths]
        assert lengths == [audits[0]['T'], audits[1]['T'], audits[-1]['T']]
        selected = min(range(3), key=lambda k: lengths[k])
        shutil.copyfile(paths[selected], folder / 'best.txt')
        save(folder / 'complete.json', {
            'input_sha256': item['sha256'], 'baseline_T': lengths[0], 'v800_T': lengths[1],
            'v801_T': lengths[2], 'T': lengths[selected], 'E': 0,
            'source': ('baseline', 'v800', 'v801')[selected], 'best_sha256': sha(folder / 'best.txt'),
            'requested_search_seconds': sum(r['requested_seconds'] for r in audits[1:]),
            'reports': {str(p.relative_to(folder)): sha(p) for p in reports},
            'unique_transitions': sum(r['episode_audit']['unique_transitions'] for r in audits[1:]),
            'episodes': sum(r['episode_audit']['episodes'] for r in audits[1:]), 'completed_at': now()})


def summary(run, manifest):
    rows = []
    for item in manifest:
        case = run / 'cases' / f"{item['index']:06d}"
        if item['role'] == 'test':
            assert not case.exists(), 'held-out test input was used to generate a teacher'
            continue
        record = read(case / 'complete.json')
        assert record['input_sha256'] == item['sha256'] and sha(case / 'best.txt') == record['best_sha256']
        assert len((case / 'best.txt').read_text().splitlines()) == record['T']
        rows.append({key: item[key] for key in ('index', 'role', 'M', 'N', 'origin')} | {
            'T': record['T'], 'baseline_T': record['baseline_T'], 'saved': record['baseline_T'] - record['T']})
    result = {'completed_at': now(), 'inputs': len(rows), 'new_inputs': sum(r['origin'] == 'new' for r in rows),
              'held_out_inputs': 256, 'roles': {}}
    for role in ('train', 'validation'):
        selected = [r for r in rows if r['role'] == role]
        result['roles'][role] = {'inputs': len(selected), 'operations': sum(r['T'] for r in selected),
                                 'mean_T': sum(r['T'] for r in selected) / len(selected),
                                 'mean_saved': sum(r['saved'] for r in selected) / len(selected)}
        assert len(selected) == COUNTS[role]
    save(run / 'teacher_rows.json', rows)
    save(run / 'teacher_result.json', result)
    return result


def main_run(run, deadline):
    config, manifest = verify(run)
    if (run / 'teacher_exit.json').exists() and read(run / 'teacher_exit.json')['exit_code'] == 0:
        print('teacher generation already completed; no computation started', flush=True)
        return
    assert read(run / 'mechanism.json')['passed']
    items = [m for m in manifest if m['origin'] == 'new' and m['role'] != 'test']
    archive_incomplete(run, items)
    warmup = run / 'warmup'
    if not (warmup / 'complete.json').exists():
        if warmup.exists():
            archive = run / 'interrupted' / f'warmup_{time.time_ns()}'
            archive.parent.mkdir(exist_ok=True)
            warmup.rename(archive)
        warmup.mkdir()
        inp = run / items[0]['path']
        teacher.command([run / 'frozen/baseline'], warmup / 'stderr.log', deadline,
                        input_path=inp, output_path=warmup / 'output.txt', timeout=60)
        metrics = teacher.score(run, inp, warmup / 'output.txt', warmup, deadline)
        save(warmup / 'complete.json', metrics)
    baselines = []
    for item in items:
        if not teacher.completed(run / 'cases' / f"{item['index']:06d}" / 'baseline'):
            baselines.append(item)
    tasks(run, 'baselines', baselines, lambda item: teacher.baseline(run, item, deadline), 20, deadline)
    work = []
    for item in items:
        case = run / 'cases' / f"{item['index']:06d}"
        for mode, last in [('v800', 0), ('v801', config['v801_rounds'] - 1)]:
            if not teacher.completed(case / mode / f'round_{last:02d}'):
                work.append((item, mode))

    def execute_branch(task):
        item, mode = task
        branch(run, item, mode, deadline, config)
        case = run / 'cases' / f"{item['index']:06d}"
        # 各ケースのv801側だけが完成印を書き、並行するv800との書き込みを競合させない。
        if mode == 'v801' and (case / 'v800/round_00/complete.json').exists():
            finish_cases(run, [item], config)

    tasks(run, 'teachers', work, execute_branch, 30, deadline)
    finish_cases(run, items, config)
    status(run, 'validating_teachers')
    result = summary(run, manifest)
    save(run / 'teacher_exit.json', {'exit_code': 0, 'completed_at': now(), 'result': 'teacher_result.json'})
    status(run, 'teachers_completed', inputs=result['inputs'], roles=result['roles'], next_stage='prepare_bc_dataset')


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--run', type=Path, default=RUN)
    p.add_argument('--mode', choices=['prepare', 'check', 'main'], required=True)
    p.add_argument('--seconds', type=int, default=43140)
    args = p.parse_args()
    assert 0 < args.seconds <= 43200
    run = args.run.resolve()
    run.mkdir(parents=True, exist_ok=True)
    deadline = time.time() + args.seconds
    for sig in (signal.SIGTERM, signal.SIGINT):
        signal.signal(sig, lambda *_: teacher.STOP.set())
    with (ROOT / 'results/.eval.lock').open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        lock.seek(0); lock.truncate()
        lock.write(json.dumps({'tool': 'v090_teacher', 'pid': os.getpid(), 'run': str(run), 'started_at': now()}))
        lock.flush()
        if args.mode == 'prepare':
            prepare(run, deadline)
        elif args.mode == 'check':
            verify(run)
            teacher.mechanism(run, deadline)
        else:
            main_run(run, deadline)


if __name__ == '__main__':
    try:
        main()
    except BaseException as error:
        teacher.STOP.set()
        location = Path(sys.argv[sys.argv.index('--run') + 1]).resolve() if '--run' in sys.argv else RUN
        if location.is_dir():
            save(location / 'teacher_failure.json', {'error': repr(error), 'at': now(), 'traceback': traceback.format_exc()})
            if '--mode' in sys.argv and sys.argv[sys.argv.index('--mode') + 1] == 'main':
                save(location / 'teacher_exit.json', {'exit_code': 1, 'error': repr(error), 'completed_at': now()})
            status(location, 'teacher_failed', error=repr(error))
        raise
