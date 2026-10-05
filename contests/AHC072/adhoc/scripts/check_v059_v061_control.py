#!/usr/bin/env python3
"""探索を呼ばず、代役プロセスと仮想時計で一括実行・v060の制御を検証する。"""
from __future__ import annotations

from argparse import ArgumentParser
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
from unittest.mock import patch

from experiment_batch_support import ROOT, load_module, write_json


class VirtualClock:
    def __init__(self):
        self.value = 0.0

    def monotonic(self):
        return self.value

    def sleep(self, seconds):
        self.value += seconds


class MockProblem:
    @classmethod
    def read(cls, path):
        return cls()

    def replay(self, text):
        # Deliberately not a contest input/output language. No solver is used.
        return dict(T=int(text.removeprefix('mock_T=').strip()), E=0)


def verify_worker(worker, destination):
    checks = []
    with tempfile.TemporaryDirectory(prefix='ahc072_control_') as temporary:
        root = Path(temporary)
        for scenario in ('multistart', 'continuous', 'failure', 'interrupt'):
            clock = VirtualClock()
            folder = root / scenario
            folder.mkdir()
            (folder / 'cases/test').mkdir(parents=True)
            (folder / 'initial.txt').write_text('mock_T=10\n')
            (folder / 'input.txt').write_text('mock_input\n')
            calls, signals = [], []
            class Process:
                def __init__(self, command, **kwargs):
                    self.pid = 900000 + len(calls)
                    self.returncode = None
                    self.polls = 0
                    self.duration = float(command[command.index('--seconds')+1])
                    seed_path = Path(command[command.index('--initial-plan')+1])
                    start = int(seed_path.read_text().removeprefix('mock_T=').strip())
                    end = start - 1
                    search = Path(command[command.index('--output-dir')+1])
                    search.mkdir()
                    (search / 'initial.txt').write_text(f'mock_T={start}\n')
                    (search / 'best.txt').write_text(f'mock_T={end}\n')
                    events = [dict(type='initial', plan='initial.txt', T=start, E=0, elapsed_sec=0, reason='mock'),
                              dict(type='improvement', plan='best.txt', T=end, E=0, elapsed_sec=self.duration,
                                   reason='mock', before_plan='initial.txt', before_T=start),
                              dict(type='finish', status='completed', T=end, E=0,
                                   elapsed_sec=self.duration, cpu_sec=0)]
                    (search / 'events.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in events))
                    kwargs['stdout'].write(f'mock_T={end}\n'.encode())
                    kwargs['stdout'].flush()
                    calls.append(dict(command=command, T=start, seed_sha256=worker.sha256(seed_path)))

                def poll(self):
                    if self.returncode is not None:
                        return self.returncode
                    self.polls += 1
                    if scenario == 'interrupt':
                        if self.polls == 1:
                            raise KeyboardInterrupt('mock interrupt')
                        return None
                    if self.polls == 1:
                        return None
                    clock.sleep(self.duration - 0.25)
                    self.returncode = 7 if scenario == 'failure' else 0
                    return self.returncode

                def wait(self, timeout=None):
                    if self.returncode is None:
                        self.returncode = -15
                    return self.returncode

            def send(process, sig):
                signals.append(sig)
                process.returncode = -sig
            mode = 'continuous' if scenario == 'continuous' else 'multistart'
            case = dict(case='test', input='input.txt', reference_T=10, seeds=[dict(path='initial.txt')])
            config = dict(minutes_per_case=5, restart_seconds=60, progress_seconds=10, seed=80020260929)
            control = worker.ProcessControl()
            with patch.object(worker, 'time', clock), patch.object(worker, 'Problem', MockProblem), \
                    patch.object(worker, 'subprocess', SimpleNamespace(Popen=Process, TimeoutExpired=subprocess.TimeoutExpired)), \
                    patch.object(worker.ProcessControl, 'send', staticmethod(send)):
                caught = None
                try:
                    worker.run_worker(folder, Path('/mock/solver'), case, mode, config, 0, control)
                except (RuntimeError, KeyboardInterrupt) as exc:
                    caught = exc
            state = json.loads((folder / 'cases/test' / mode / 'status.json').read_text())
            if scenario in ('multistart', 'continuous'):
                assert caught is None and state['status'] == 'completed'
                assert len(calls) == (5 if scenario == 'multistart' else 1)
                assert [r['T'] for r in calls] == list(range(10, 10-len(calls), -1))
                for index, call in enumerate(calls):
                    expected = int.from_bytes(hashlib.sha256(f'80020260929:test:{mode}:{index}'.encode()).digest()[:8], 'big')
                    assert int(call['command'][call['command'].index('--seed')+1]) == expected
                    meta = json.loads((folder / 'cases/test' / mode / f'round_{index:04d}/round.json').read_text())
                    assert meta['start_best_sha256'] == call['seed_sha256'] == meta['initial_sha256']
                    assert meta['new_saved'] == 1
                assert clock.value == 300 and not control.processes
            else:
                assert caught is not None and state['status'] == 'failed'
                assert control.stop.is_set() and not control.processes
                if scenario == 'interrupt':
                    assert isinstance(caught, KeyboardInterrupt) and signals
            checks.append(dict(scenario=scenario, passed=True, mock_launches=len(calls)))
        control = worker.ProcessControl()
        control.register(SimpleNamespace(pid=1))
        control.register(SimpleNamespace(pid=2))
        assert control.max_active == 2
        try:
            control.register(SimpleNamespace(pid=3))
        except RuntimeError:
            assert control.stop.is_set()
        else:
            raise AssertionError('上限超過を検出しない')
        checks.append(dict(scenario='process_limit', passed=True, actual_processes=0))
    write_json(destination, dict(status='passed', solver_executions=0, checks=checks))
    return checks


def verify_batch(batch):
    checks = []
    with tempfile.TemporaryDirectory(prefix='ahc072_batch_control_') as temporary:
        ctx = SimpleNamespace(run=Path(temporary))
        clock = VirtualClock()
        starts, pauses = [], []
        def pause(old, new):
            before = clock.value
            batch.cooldown(old, new, sleep=clock.sleep, monotonic=clock.monotonic, emit=lambda *a, **k: None)
            pauses.append(clock.value-before)
        def runner(name):
            def run(context):
                starts.append((name, clock.value))
                clock.sleep(7)
                return dict(status='evaluation_skipped' if name == 'v059' else 'mock_completed')
            return run
        result = batch.run_stages(ctx, {n: runner(n) for n in batch.STAGES}, pause=pause)
        assert list(result) == list(batch.STAGES)
        assert starts == [(name, 127*i) for i, name in enumerate(batch.STAGES)]
        assert pauses == [120] * (len(batch.STAGES)-1)
        checks.append('sequential_order_and_120_second_pauses')
        invoked = []
        def fail(context):
            invoked.append('v059')
            raise RuntimeError('mock failure')
        try:
            batch.run_stages(ctx, {'v059': fail, 'v060': lambda c: invoked.append('v060')}, pause=pause)
        except RuntimeError:
            assert invoked == ['v059']
        else:
            raise AssertionError('失敗後に後続実験が動いた')
        checks.append('abort_on_error')
        with patch.object(batch, 'ROOT', ctx.run):
            with batch.evaluation_lock():
                try:
                    with batch.evaluation_lock():
                        raise AssertionError('競合ロックを取得した')
                except RuntimeError as exc:
                    assert '実行中' in str(exc)
            checks.append('shared_eval_lock_excludes_concurrent_run')
            parent_status = ctx.run / batch.PARENT_RUN / 'status.json'
            write_json(parent_status, dict(status='running'))
            try:
                batch.preflight()
            except RuntimeError as exc:
                assert 'v800' in str(exc)
            else:
                raise AssertionError('未完了のv800を許可した')
            checks.append('unfinished_v800_is_rejected')
    return checks


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'adhoc/v060_audit/control_verification.json')
    args = parser.parse_args()
    worker = load_module(ROOT / 'adhoc/scripts/v060_worker.py', 'control_test_worker')
    batch = load_module(ROOT / 'adhoc/scripts/run_v059_v061.py', 'control_test_batch')
    checks = verify_worker(worker, args.output)
    report = dict(status='passed', solver_executions=0, worker=checks, batch=verify_batch(batch))
    write_json(args.output, report)
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
