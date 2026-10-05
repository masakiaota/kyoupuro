#!/usr/bin/env python3
"""節目単位の共同探索を固定条件で一回診断する。"""
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime
from build_v132_tasks import ROOT, RUN, OUT, SOURCE, build


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def phase(name, **values):
    save(RUN / 'status.json', dict(phase=name, updated_at=datetime.now().astimezone().isoformat(), **values))


def main():
    RUN.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(exist_ok=True)
    assert not (RUN / 'trials.csv').exists(), 'Existing experiment must not be rerun.'
    phase('build')
    build()
    for local in (True, False):
        mode = 'local' if local else 'judge'
        args = ['scripts/build_solver.sh'] + ([] if local else ['--no-local']) + [SOURCE.stem]
        proc = subprocess.run(args, cwd=ROOT, text=True, capture_output=True)
        (RUN / f'build_{mode}.log').write_text(proc.stdout + proc.stderr)
        assert proc.returncode == 0, proc.stderr
        shutil.copy2(ROOT / 'target/release' / SOURCE.stem, RUN / f'probe_{mode}')
    env = os.environ.copy()
    env.setdefault('SDKROOT', subprocess.check_output(['xcrun', '--show-sdk-path'], text=True).strip())
    env.setdefault('MACOSX_DEPLOYMENT_TARGET', '15.0')
    preprocessed = {}
    for local in (True, False):
        mode = 'local' if local else 'judge'
        cmd = ['g++-15', '-std=gnu++23', '-E', '-P']
        cmd += ['-DLOCAL'] if local else ['-DATCODER', '-DONLINE_JUDGE', '-DNOMINMAX']
        data = subprocess.check_output(cmd + [str(SOURCE)], cwd=ROOT, env=env, text=True)
        (RUN / f'{mode}.ii').write_text(data)
        first = data.index('struct TaskEdge {')
        last = data.index('struct Trial {', first)
        preprocessed[mode] = ' '.join(data[first:last].split())
    assert preprocessed['local'] == preprocessed['judge']
    save(RUN / 'build_verification.json', dict(complete_preprocessing=True, task_search_identical=True,
                                              source_sha256=sha(SOURCE)))
    old = ROOT / 'adhoc/v044_joint_temporal'
    shutil.copy2(old / 'trials.txt', OUT / 'trials.txt')
    settings = json.loads((old / 'settings.json').read_text())
    settings.update(label_limit=100000, expansion_limit=4096, stop_on_first_improvement=True)
    save(OUT / 'settings.json', settings)
    checker = ROOT / 'adhoc/scripts/check_v132_tasks.py'
    check_text = (ROOT / 'adhoc/scripts/check_v044_joint_temporal.py').read_text()
    check_text = check_text.replace('adhoc/v044_joint_temporal', str(OUT.relative_to(ROOT)))
    check_text = check_text.replace('results/analysis/v044_joint_temporal.csv', str((RUN / 'trials.csv').relative_to(ROOT)))
    check_text = check_text.replace('capped_main_trials=sum(row["status"] == "label_cap" for row in main_rows)',
                                  'capped_main_trials=sum(row["status"] in ("label_cap", "expansion_cap") for row in main_rows)')
    check_text = check_text.replace('assert len(fixtures) == 3 and all(int(row["best"]) == 3 for row in fixtures)',
        'assert len(fixtures) == 3 and all(3 <= int(row["best"]) <= limit for row, limit in zip(fixtures, (4, 3, 5)))')
    checker.write_text(check_text)
    shutil.copy2(ROOT / 'notes/experiments/v132.md', OUT / 'preregistration.md')
    sources = [SOURCE, ROOT / 'src/bin/v000_template.cpp', ROOT / 'notes/experiments/v132.md', checker,
               ROOT / 'adhoc/scripts/build_v132_tasks.py', ROOT / 'adhoc/scripts/run_v132_tasks.py',
               ROOT / 'adhoc/scripts/v132_temporal_tasks.cpp.txt', ROOT / 'adhoc/bin/v044_joint_temporal_probe.cpp',
               OUT / 'trials.txt', OUT / 'settings.json', OUT / 'preregistration.md']
    for trial in settings['trials']:
        sources.extend([ROOT / f"tools/in/{trial['case']}.txt", ROOT / f"results/out/v039_exact_board_lns/{trial['case']}.txt"])
        if trial['reference'] != '-':
            sources.append(ROOT / trial['reference'])
    save(OUT / 'sources_before.json', {str(p.relative_to(ROOT)): sha(p) for p in sorted(set(sources))})
    phase('diagnostic')
    start = time.monotonic()
    with (RUN / 'stdout.log').open('w') as stdout, (RUN / 'stderr.log').open('w') as stderr:
        proc = subprocess.run([str(RUN / 'probe_local'), str(ROOT)], cwd=ROOT, stdout=stdout, stderr=stderr, timeout=540)
    save(RUN / 'exit.json', dict(returncode=proc.returncode, elapsed_sec=time.monotonic() - start))
    assert proc.returncode == 0, (RUN / 'stderr.log').read_text()[-3000:]
    phase('independent_verification')
    proc = subprocess.run([sys.executable, str(checker)], cwd=ROOT, capture_output=True, text=True)
    (RUN / 'verification.log').write_text(proc.stdout + proc.stderr)
    assert proc.returncode == 0, proc.stderr
    phase('complete')
    print((OUT / 'verification.json').read_text())


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        phase('failed', error=repr(error))
        raise
