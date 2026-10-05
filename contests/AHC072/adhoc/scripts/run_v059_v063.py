#!/usr/bin/env python3
"""v059〜v063の5実験を、120秒の休止を挟み最大2探索プロセスで手動実行する。"""
from __future__ import annotations

import csv
import json
from pathlib import Path

from experiment_batch_support import ROOT, PARENT_RUN, load_module

base = load_module(ROOT / 'adhoc/scripts/run_v059_v061.py', 'five_experiment_batch')
base.BATCH_NAME = 'v059_v063'
base.STAGES = ('v059', 'v060', 'v061', 'v062', 'v063')
base.ENTRYPOINTS |= dict(v062='audit_v062.py', v063='analyze_v063_sparse.py')
base.SOURCES += (
    'src/bin/v062_deferred_passenger.cpp', 'adhoc/bin/check_v062_deferred_passenger.cpp',
    'adhoc/bin/v063_sparse_lns_probe.cpp', 'adhoc/scripts/audit_v062.py',
    'adhoc/scripts/analyze_v063_sparse.py', 'adhoc/scripts/run_v059_v063.py',
    'adhoc/scripts/check_v059_v063_control.py', 'adhoc/v062_audit/registered_changes.json',
    'notes/experiments/v062.md', 'notes/experiments/v063.md',
)
_original_preflight = base.preflight


def preflight():
    # This batch extends the original three experiments. Refuse to rerun any
    # started stage, even when it produced no ordinary evaluation records.
    for batch_name in ('v059_v061', 'v059_v063'):
        latest = ROOT / 'results/experiment_batches' / batch_name / 'latest.json'
        if not latest.exists():
            continue
        previous = Path(json.loads(latest.read_text())['run'])
        state = json.loads((previous / 'status.json').read_text())
        if state.get('stage') or state.get('after') or state.get('status') == 'completed' or any(previous.glob('v0??.json')):
            raise RuntimeError(f'{batch_name}は探索開始済み。重複実行を防ぐため停止する。保存先: {previous}')
    baseline = _original_preflight()
    records = [json.loads(line) for line in (ROOT / 'results/eval_records.jsonl').read_text().splitlines() if line.strip()]
    if any(r['bin'] == 'v062_deferred_passenger' for r in records):
        raise RuntimeError('v062は評価済み。重複評価を防ぐため停止する。')
    extra = set()
    directory = 'results/analysis/v800/20260929_all_100/complete/'
    intervals, witnesses = directory + 'shortest_intervals.csv', directory + 'deferred_passengers.csv'
    rows = list(csv.DictReader((ROOT / intervals).open()))
    long_rows = [r for r in rows if r['group'] == 'long' and int(r['before_moves']) > 64]
    witness_rows = list(csv.DictReader((ROOT / witnesses).open()))
    if len(long_rows) != 316 or len(witness_rows) != 6:
        raise RuntimeError('固定した316更新・B-81の6例と異なる')
    extra.update((intervals, witnesses))
    for row in long_rows + witness_rows:
        for key in ('before_plan', 'plan'):
            path = (PARENT_RUN / row[key])
            if not (ROOT / path).resolve().is_relative_to((ROOT / PARENT_RUN).resolve()) or not (ROOT / path).is_file():
                raise RuntimeError(f'追加診断の出典が存在しない: {path}')
            extra.add(str(path))
    base.EXTRA_DATA = tuple(sorted(extra))
    return baseline


base.preflight = preflight

if __name__ == '__main__':
    raise SystemExit(base.main())
