#!/usr/bin/env python3
"""学習用入力だけから順序を全列挙し、入力単位で保存・再開する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import fcntl
import gzip
import json
import os
from pathlib import Path
import random
import subprocess
import time

from build_v122_order import ROOT, RUN, BASE
from v089_data import save, sha, now, status
from v090_data import RUN as BC_RUN
from run_v089_evaluation import validated_output


def load(path):
    return json.loads(path.read_text())


def prepare():
    marker = RUN / 'inputs.json'
    if marker.exists():
        return load(marker)
    source = load(BC_RUN / 'input_manifest.json')
    excluded = {c['sha256'] for c in source if c['role'] != 'train'}
    for directory in ['tools/in', 'tools/validation1']:
        excluded.update(sha(p) for p in (ROOT / directory).glob('*.txt'))
    cases = [c for c in source if c['role'] == 'train' and c['sha256'] not in excluded]
    random.Random(122001).shuffle(cases)
    cases = [dict(c,order_role='train' if i < 256 else 'development') for i, c in enumerate(cases[:320])]
    assert len(cases) == 320 and len({c['sha256'] for c in cases}) == 320
    assert all(c['sha256'] not in excluded for c in cases)
    save(marker, cases)
    save(RUN / 'input_exclusion.json', dict(excluded_hashes=sorted(excluded),
                                         source_sha256=sha(BC_RUN / 'input_manifest.json')))
    return cases


def codes_to_text(codes):
    lines = []
    for code in codes:
        p = code & 511
        lines.append(f"{p//20} {p%20} {(code>>9)&7} {'UDLR'[(code>>12)&3]} {1+((code>>14)&7)}")
    return '\n'.join(lines) + '\n'


def one(case):
    where = RUN / 'data' / f"{case['index']:06d}"
    where.mkdir(parents=True, exist_ok=True)
    marker = where / 'result.json'
    identity = dict(input_sha256=case['sha256'],binary_sha256=sha(RUN / 'collector'))
    if marker.exists():
        result = load(marker)
        assert result['identity'] == identity
        return result
    inp = BC_RUN / case['path']
    assert sha(inp) == case['sha256']
    started = time.monotonic()
    with inp.open() as stream, (where / 'rows.tmp').open('w') as out, (where / 'stderr.log').open('w') as err:
        proc = subprocess.run([RUN / 'collector'], stdin=stream, stdout=out, stderr=err, timeout=300)
    assert proc.returncode == 0, (case['index'], proc.returncode, (where / 'stderr.log').read_text()[-2000:])
    rows = [json.loads(line) for line in (where / 'rows.tmp').read_text().splitlines()]
    extracted = [row for row in rows if row['extracted']]
    assert len({(row['phase'], row['group']) for row in rows}) == len(rows)
    replayed = 0
    for phase in (0, 1):
        for row in extracted:
            if row['phase'] != phase or not any(row['complete']):
                continue
            first = next(i for i, ok in enumerate(row['complete']) if ok)
            best = min(range(len(row['costs'])), key=row['costs'].__getitem__)
            for at in sorted({first, best}):
                checked = validated_output(inp, codes_to_text(row['paths'][at]))
                assert checked['E'] == 0 and checked['T'] == row['costs'][at]
                replayed += 1
            break
    with gzip.open(where / 'rows.jsonl.gz', 'wt') as stream:
        for row in rows:
            stream.write(json.dumps(row, separators=(',', ':')) + '\n')
    (where / 'rows.tmp').unlink()
    differences = [min(row['costs'][0], row['costs'][row['alternate']]) - min(row['costs']) for row in extracted]
    result = dict(identity=identity,case=case['index'],role=case['order_role'],groups=len(extracted),
                  attempted=len(rows),oracle_saved=sum(differences),better_groups=sum(x > 0 for x in differences),
                  permutations=sum(len(row['costs']) for row in extracted),
                  complete_paths=sum(sum(row['complete']) for row in extracted),
                  independent_python_replays=replayed,seconds=time.monotonic()-started,completed_at=now())
    save(marker,result)
    return result


def collect(full=False):
    RUN.mkdir(parents=True, exist_ok=True)
    lock = (RUN / 'collection.lock').open('a')
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    cases = prepare()
    if full:
        assert load(RUN / 'pilot/result.json')['passed']
    selected = cases if full else cases[:32]
    started = time.monotonic()
    results = []
    label = 'full' if full else 'pilot'
    status(RUN / 'collection', 'collecting', role=label, total=len(selected), pid=os.getpid())
    with ThreadPoolExecutor(max_workers=12) as pool:
        futures = [pool.submit(one, case) for case in selected]
        for future in as_completed(futures):
            results.append(future.result())
            if len(results) % 8 == 0:
                status(RUN / 'collection', 'collecting', role=label, completed=len(results), total=len(selected),pid=os.getpid())
    groups = sum(r['groups'] for r in results)
    summary = dict(cases=len(results),groups=groups,oracle_saved=sum(r['oracle_saved'] for r in results),
                   better_cases=sum(r['better_groups'] > 0 for r in results),
                   complete_paths=sum(r['complete_paths'] for r in results),
                   independent_python_replays=sum(r['independent_python_replays'] for r in results),
                   seconds=time.monotonic()-started,completed_at=now())
    summary['oracle_mean_improvement'] = summary['oracle_saved'] / max(1,groups)
    summary['passed'] = groups >= 320 and summary['better_cases'] >= 12 and summary['oracle_mean_improvement'] >= .25
    save(RUN / label / 'result.json', summary)
    status(RUN / 'collection', 'completed',role=label,**summary)


if __name__ == '__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--full', action='store_true')
    collect(parser.parse_args().full)
