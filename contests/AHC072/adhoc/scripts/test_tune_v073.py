#!/usr/bin/env python3
"""Control-flow tests only: never compile or execute a contest solver."""
from pathlib import Path
import json
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

import tune_v073 as tuning


class TuningControlTests(unittest.TestCase):
    def test_warmups_precede_unique_measurements(self):
        plan = tuning.evaluation_plan()
        self.assertEqual(plan[:3], [("warmup", 8), ("warmup", 8), ("measure", 8)])
        measured = [tick for phase, tick in plan if phase == "measure"]
        self.assertEqual(len(measured), 21)
        self.assertEqual(set(measured), set(range(21)))
        self.assertEqual(plan, tuning.evaluation_plan())

    def test_only_two_cases_run_concurrently(self):
        lock = threading.Lock()
        active = peak = 0
        completed = []
        def operation(value):
            nonlocal active, peak
            with lock:
                active += 1
                peak = max(peak, active)
            time.sleep(0.002)
            with lock:
                active -= 1
            return value
        tuning.parallel_cases(range(25), operation, completed.append)
        self.assertEqual(peak, 2)
        self.assertCountEqual(completed, range(25))

    def test_failure_stops_scheduling(self):
        started = []
        lock = threading.Lock()
        def operation(value):
            with lock:
                started.append(value)
            time.sleep(0.002)
            return value
        def reject(value):
            raise RuntimeError("intentional control-test failure")
        with self.assertRaisesRegex(RuntimeError, "intentional"):
            tuning.parallel_cases(range(100), operation, reject)
        self.assertLessEqual(len(started), 2)

    def test_selection_uses_score_then_baseline_distance(self):
        rows = [dict(total_sum=100, tick=n, eligible=True) for n in (7, 9, 8)]
        self.assertEqual(tuning.choose_best(rows)["tick"], 8)
        self.assertEqual(tuning.choose_best(rows[:2])["tick"], 7)
        self.assertEqual(tuning.choose_best(rows + [dict(total_sum=99, tick=20, eligible=True)])["tick"], 20)
        self.assertEqual(tuning.choose_best(rows + [dict(total_sum=1, tick=0, eligible=False)])["tick"], 8)
        with self.assertRaises(RuntimeError):
            tuning.choose_best([dict(total_sum=1, tick=0, eligible=False)])

    def test_export_changes_only_default_value(self):
        source = tuning.source_check()
        for tick in tuning.TICKS:
            changed = tuning.source_with_weight(source, tick)
            restored = changed.replace("#define AHC072_REPAIR_WEIGHT " + tuning.weight(tick),
                                       tuning.DEFAULT_DEFINE, 1)
            self.assertEqual(restored, source)

    def test_remaining_slimes_are_rejected(self):
        with tempfile.TemporaryDirectory(prefix="v073-validation-", dir=tuning.ROOT / "adhoc") as folder:
            root = Path(folder)
            output = root / "0000.txt"
            output.write_text("0 0 0 R 1\n")
            counts = {key: 0 for key in tuning.ERRORS}
            counts.update(E=1, T=1, state_pool_free_at_end=4, state_slots=4)
            error_file = root / "0000.txt.err"
            error_file.write_text("".join(f"[summary.count] {key}={value}\n" for key, value in counts.items()))
            result = tuning.evaluator.CaseResult("0000.txt", "ok", 100001, 10,
                                                 str(output.relative_to(tuning.ROOT)))
            with patch.object(tuning.evaluator, "run_case", return_value=result):
                actual = tuning.checked_case(root / "fake-input", root / "fake-bin", root / "fake-score", root)
            self.assertEqual(actual.status, "validation_fail")
            self.assertIsNone(actual.score)
            self.assertIn("tuning validation failed", error_file.read_text())

    def test_warmup_scores_never_enter_measured_logs(self):
        with tempfile.TemporaryDirectory(prefix="v073-control-", dir=tuning.ROOT / "adhoc") as folder:
            root = Path(folder)
            records, summary = root / "official.jsonl", root / "official.csv"
            with patch.object(tuning, "RUN", root), patch.object(tuning, "CASE_COUNT", 4), \
                 patch.object(tuning.evaluator, "RECORDS_JSONL", records), \
                 patch.object(tuning.evaluator, "SUMMARY_CSV", summary):
                run = tuning.TuningRun()
                run.verify_frozen = lambda: None
                run.status = lambda *args, **kwargs: None
                run.input_dir = root / "input"
                run.input_dir.mkdir()
                run.inputs = [run.input_dir / f"{i:04d}.txt" for i in range(4)]
                run.scorer = root / "fake-scorer"
                for path in run.inputs:
                    path.write_text("control test; not a contest input\n")
                def fake_case(path, binary, scorer, output):
                    answer = output / path.name
                    answer.write_text("control test output\n")
                    tick = int(binary.name.rsplit("_", 1)[1])
                    return tuning.evaluator.CaseResult(path.name, "ok", 100 + tick, 10,
                                                        str(answer.relative_to(tuning.ROOT)))
                tuning.evaluator.ensure_csv_header(summary, tuning.evaluator.SUMMARY_HEADER)
                with patch.object(tuning, "checked_case", fake_case):
                    run.evaluate("warmup", 8, 1)
                    run.evaluate("warmup", 8, 2)
                    self.assertEqual(run.measured, [])
                    self.assertFalse(records.exists())
                    self.assertFalse((root / "trials.csv").exists())
                    self.assertEqual(len(summary.read_text().splitlines()), 1)
                    self.assertEqual(list((root / "warmup").iterdir()), [])
                    self.assertNotIn("total_sum", json.loads((root / "warmup_1.json").read_text()))
                    run.evaluate("measure", 8, 1)
                    run.evaluate("measure", 4, 2)
                self.assertEqual(len(records.read_text().splitlines()), 8)
                self.assertEqual(len(summary.read_text().splitlines()), 3)
                self.assertEqual(len((root / "cases.jsonl").read_text().splitlines()), 8)
                self.assertEqual(run.measured[0]["baseline_delta"], 0)
                self.assertEqual(run.measured[1]["baseline_delta"], -16)


if __name__ == "__main__":
    unittest.main()
