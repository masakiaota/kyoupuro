#!/usr/bin/env python3
"""親経路の保持、二本の途中経路、完成列と前処理差分を確認する。"""
from concurrent.futures import ThreadPoolExecutor
import difflib
import subprocess
from build_v131_paths import ROOT, RUN, BASE, SOURCE, build
from check_v113_integrated import block, normalize
from check_v089_board import compile_binary, trace
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save, sha, now, status
from v090_data import RUN as BC_RUN
from run_v130_rrt import load


def check():
    source = build()
    where = RUN / 'mechanism'
    where.mkdir(parents=True, exist_ok=True)
    marker = where / 'result.json'
    if marker.exists():
        result = load(marker)
        assert result['source_sha256'] == sha(source)
        return result
    status(RUN / 'pipeline', 'mechanism_builds')
    for local in (True, False):
        mode = 'local' if local else 'judge'
        compile_binary(source.stem, where / ('solver_' + mode), local)
        original = normalize(preprocess(BASE, where / ('base_' + mode + '.ii'), local))
        current = normalize(preprocess(SOURCE, where / (mode + '.ii'), local))
        (where / (mode + '.patch')).write_text(''.join(difflib.unified_diff(
            original.splitlines(True), current.splitlines(True), fromfile='v113', tofile='v131')))
        aligned = current
        for name in ['bool insert_at(', 'bool insert_two_orders(', 'void optimize(']:
            aligned = aligned.replace(block(aligned, name), block(original, name), 1)
        aligned = aligned.replace('template<bool CollectAlternative=false>', '', 1)
        assert ' '.join(aligned.split()) == ' '.join(original.split()), mode
    frozen = RUN / 'frozen'
    frozen.mkdir(exist_ok=True)
    parent = BASE.read_text()
    methods = [block(parent, 'bool insert_at(').replace('bool insert_at(', 'bool original_insert_at(', 1),
               block(parent, 'bool insert(').replace('bool insert(', 'bool original_insert(', 1).replace('return insert_at(', 'return original_insert_at(', 1),
               block(parent, 'bool insert_two_orders(').replace('bool insert_two_orders(', 'bool original_two_orders(', 1).replace('if(!insert(', 'if(!original_insert(')]
    core = SOURCE.read_text().replace('    // 第2順序では先頭2匹だけを交換する。',
        '\n'.join(methods) + '\n    // 第2順序では先頭2匹だけを交換する。', 1)
    assert core != SOURCE.read_text()
    (frozen / 'probe_core.cpp').write_text(core)
    compile_binary('check_v131_paths', where / 'probe', True)
    cases = [c for c in load(BC_RUN / 'input_manifest.json') if c['role'] == 'train'][:32]
    save(where / 'input_manifest.json', cases)

    def one(case):
        import json
        dest = where / f"{case['index']:06d}"
        dest.mkdir(exist_ok=True)
        path = BC_RUN / case['path']
        assert sha(path) == case['sha256']
        proc = subprocess.run([where / 'probe'], input=path.read_text(), text=True,
                              capture_output=True, timeout=180)
        (dest / 'rows.jsonl').write_text(proc.stdout)
        (dest / 'stderr.log').write_text(proc.stderr)
        assert proc.returncode == 0, (case['index'], proc.returncode, proc.stderr[-2000:])
        rows = [json.loads(line) for line in proc.stdout.splitlines()]
        # 独立Python再生でも全完成列を確認する。
        checked = 0
        for row in rows:
            if not row['extracted']:
                continue
            paths = ([row['path']] if row['new_ok'] else []) + row['extra_paths']
            for codes in paths:
                text = '\n'.join(f"{(c&511)//20} {(c&511)%20} {(c>>9)&7} {'UDLR'[(c>>12)&3]} {1+((c>>14)&7)}" for c in codes) + '\n'
                actual = validated_output(path, text)
                assert actual['E'] == 0
                checked += 1
        return dict(case=case['index'], rows=rows, python_replayed=checked)

    status(RUN / 'pipeline', 'mechanism_partial_problems')
    with ThreadPoolExecutor(max_workers=12) as pool:
        diagnostics = list(pool.map(one, cases))
    rows = [r for d in diagnostics for r in d['rows'] if r['extracted']]
    counts = {k: sum(r[k] for r in rows) for k in ['primary_matches', 'alternates', 'extra_completed']}
    assert rows and all(v > 0 for v in counts.values())
    reports = []
    for case in cases[:4]:
        path = BC_RUN / case['path']
        for local in (True, False):
            mode = 'local' if local else 'judge'
            proc = subprocess.run([where / ('solver_' + mode)], input=path.read_text(), text=True,
                                  capture_output=True, check=True, timeout=10)
            checked = validated_output(path, proc.stdout)
            assert checked['E'] == 0
            counters = trace(proc.stderr)
            if local:
                assert counters['state_pool_free_at_end'] == 4
                assert all(counters.get(k, 0) == 0 for k in (
                    'lns_errors', 'construction_errors', 'lns_invalid_candidates', 'baseline_recovery', 'final_recovery'))
            dest = where / f"{case['index']}_{mode}"
            dest.mkdir(exist_ok=True)
            (dest / 'output.txt').write_text(proc.stdout)
            (dest / 'stderr.log').write_text(proc.stderr)
            reports.append(dict(case=case['index'], mode=mode, trace=counters, **checked))
    assert sum(r['trace'].get('path_pair_completed', 0) for r in reports) > 0
    result = dict(passed=True, source_sha256=sha(SOURCE), partial_problems=len(rows), counts=counts,
        python_replayed=sum(d['python_replayed'] for d in diagnostics),
        locally_better=sum(r['old_ok'] and r['new_ok'] and r['new_T'] < r['old_T'] for r in rows),
        local_saved=sum(r['old_T']-r['new_T'] for r in rows if r['old_ok'] and r['new_ok']),
        rescued=sum(not r['old_ok'] and r['new_ok'] for r in rows),
        old_seconds=sum(r['old_seconds'] for r in rows), new_seconds=sum(r['new_seconds'] for r in rows),
        reports=reports, completed_at=now())
    save(marker, result)
    return result


if __name__ == '__main__':
    check()
