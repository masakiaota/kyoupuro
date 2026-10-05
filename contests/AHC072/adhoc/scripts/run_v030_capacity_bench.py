from pathlib import Path
import fcntl
import hashlib
import json
import os
import platform
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'adhoc/v030_state_capacity'
DEST.mkdir(parents=True, exist_ok=True)

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

paths = [ROOT / 'src/bin/v000_template.cpp', ROOT / 'adhoc/bin/v030_state_capacity_bench.cpp', ROOT / 'notes/experiments/v030.md', ROOT / 'target/release/v030_state_capacity_bench']
paths += sorted((ROOT / 'tools/in').glob('*.txt'))
paths += sorted((ROOT / 'results/out/v025_fast_math').glob('*.txt'))
manifest = {str(p.relative_to(ROOT)): sha(p) for p in paths}
(DEST / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
(DEST / 'benchmark_source.cpp.txt').write_text(paths[1].read_text())
(DEST / 'template_source.cpp.txt').write_text(paths[0].read_text())
environment = {
    'system': platform.system(), 'release': platform.release(), 'machine': platform.machine(),
    'cpu': subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
    'compiler': subprocess.check_output(['g++-15', '--version'], text=True).splitlines()[0],
    'flags': '-std=gnu++23 -O2 -Wall -Wextra -march=native -pthread -ftrivial-auto-var-init=zero -fopenmp -DATCODER -DONLINE_JUDGE -DNOMINMAX',
    'source_pragma': 'O3,unroll-loops,fast-math', 'repetitions': 6, 'iterations': 131072,
    'warmup_iterations': 16384, 'threads': 1, 'pool_sizes': [1, 64, 8192],
    'local_time_ratio': 'unused: fixed work component benchmark',
}
with (ROOT / 'results/.eval.lock').open('a+') as lock:
    print('Waiting for the evaluation lock', flush=True)
    fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
    environment['started_at'] = time.strftime('%Y-%m-%dT%H:%M:%S%z')
    (DEST / 'environment.json').write_text(json.dumps(environment, indent=2) + '\n')
    with (DEST / 'run.log').open('w') as log:
        process = subprocess.Popen([str(paths[3]), '--measure', str(ROOT), str(DEST)], cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        for line in process.stdout:
            log.write(line)
            log.flush()
            print(line, end='', flush=True)
        status = process.wait()
    environment['finished_at'] = time.strftime('%Y-%m-%dT%H:%M:%S%z')
    environment['exit_code'] = status
    (DEST / 'environment.json').write_text(json.dumps(environment, indent=2) + '\n')
    fcntl.flock(lock.fileno(), fcntl.LOCK_UN)
require_unchanged = {str(p.relative_to(ROOT)): sha(p) == manifest[str(p.relative_to(ROOT))] for p in paths}
(DEST / 'unchanged.json').write_text(json.dumps(require_unchanged, indent=2) + '\n')
if not all(require_unchanged.values()):
    raise RuntimeError('A source or corpus file changed during measurement')
raise SystemExit(status)
