#!/usr/bin/env python3
"""探索を実行せず、5実験の順序、冷却、中断、二重実行防止を検証する。"""
from __future__ import annotations

import json
from pathlib import Path
import tempfile
from unittest.mock import patch

from experiment_batch_support import ROOT, THREAD_ENV, load_module, write_json


def main():
    check = load_module(ROOT / 'adhoc/scripts/check_v059_v061_control.py', 'five_checks')
    wrapper = load_module(ROOT / 'adhoc/scripts/run_v059_v063.py', 'five_wrapper')
    batch = wrapper.base
    worker = load_module(ROOT / 'adhoc/scripts/v060_worker.py', 'five_worker')
    destination = ROOT / 'adhoc/v063_audit/control_verification.json'
    worker_checks = check.verify_worker(worker, destination.with_name('worker_control_verification.json'))
    with patch.object(batch, 'preflight', wrapper._original_preflight):
        batch_checks = check.verify_batch(batch)
    assert len(batch.STAGES) == 5 and batch.COOLDOWN_SECONDS == 120
    assert all(v == '1' for v in THREAD_ENV.values())
    for name in ('v059_v061', 'v059_v063'):
        with tempfile.TemporaryDirectory(prefix='ahc072_no_repeat_') as tmp:
            root = Path(tmp)
            run = root / 'results/experiment_batches' / name / 'mock'
            write_json(run.parent / 'latest.json', dict(run=str(run)))
            write_json(run / 'status.json', dict(status='interrupted', stage='v060'))
            with patch.object(wrapper, 'ROOT', root), patch.object(batch, 'ROOT', root):
                try:
                    wrapper.preflight()
                except RuntimeError as exc:
                    assert '探索開始済み' in str(exc)
                else:
                    raise AssertionError('開始済みの実験を再実行した')
    batch_checks += ['five_stages_four_120_second_pauses', 'single_thread_per_process', 'no_repeated_started_experiments']
    report = dict(status='passed', solver_executions=0, worker=worker_checks, batch=batch_checks)
    write_json(destination, report)
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
