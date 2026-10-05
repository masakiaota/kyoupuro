#!/usr/bin/env python3
"""代役の評価器だけで6条件の制御を検査する。contest solverは実行しない。"""
from collections import Counter
import csv
import json
from pathlib import Path
import re
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

import tune_v075 as tuning
import tune_v073 as common


class TuningControlTests(unittest.TestCase):
    def test_design_changes_only_size_exponent_on_the_fixed_grid(self):
        self.assertEqual(len(tuning.PARAMETERS), 6)
        self.assertEqual(tuning.CONFIGS, tuning.configurations())
        self.assertEqual(len(tuning.CONFIGS), 6)
        self.assertEqual(len(set(tuning.CONFIGS)), 6)
        values = [tuning.parameter_values(c) for c in tuning.CONFIGS]
        self.assertEqual(values[0]["removal_size_exponent"], 0.95)
        self.assertEqual(sorted(v["removal_size_exponent"] for v in values),
                         [0.8, 0.95, 1.0, 1.2, 1.4, 1.6])
        for config in tuning.CONFIGS[1:]:
            self.assertEqual(sum(a != b for a, b in zip(config, tuning.BASE)), 1)
        for parameter in tuning.PARAMETERS[1:]:
            self.assertTrue(all(v[parameter.name] == parameter.default for v in values))
            self.assertEqual(len(parameter.values), 1)

    def test_two_identical_warmups_then_baseline_and_other_measurements(self):
        plan = tuning.evaluation_plan()
        self.assertEqual(plan[:3], [("warmup", 0), ("warmup", 0), ("measure", 0)])
        measured = [c for phase, c in plan if phase == "measure"]
        self.assertEqual(len(measured), 6)
        self.assertEqual(set(measured), set(range(6)))
        self.assertEqual(plan, tuning.evaluation_plan())

    def test_two_cases_at_most_and_failures_stop_new_cases(self):
        lock = threading.Lock()
        active = peak = 0
        def operation(value):
            nonlocal active, peak
            with lock:
                active += 1
                peak = max(peak, active)
            time.sleep(0.002)
            with lock:
                active -= 1
            return value
        completed = []
        tuning.parallel_cases(range(20), operation, completed.append)
        self.assertEqual(peak, 2)
        self.assertCountEqual(completed, range(20))
        started = []
        def record(value):
            started.append(value)
            return operation(value)
        def fail(value):
            raise RuntimeError("intentional failure")
        with self.assertRaisesRegex(RuntimeError, "intentional"):
            tuning.parallel_cases(range(20), record, fail)
        self.assertLessEqual(len(started), 2)

    def test_selection_and_ties(self):
        rows = [dict(total_sum=100, config_id=n, eligible=True) for n in (5, 2, 0)]
        self.assertEqual(tuning.choose_best(rows)["config_id"], 0)
        tied = [dict(total_sum=100, config_id=n, eligible=True) for n in (1, 2)]
        self.assertEqual(tuning.choose_best(tied)["config_id"], 2)
        rows += [dict(total_sum=99, config_id=4, eligible=True)]
        self.assertEqual(tuning.choose_best(rows)["config_id"], 4)
        rows += [dict(total_sum=1, config_id=5, eligible=False)]
        self.assertEqual(tuning.choose_best(rows)["config_id"], 4)
        with self.assertRaises(RuntimeError):
            tuning.choose_best([dict(total_sum=1, config_id=0, eligible=False)])

    def test_export_contains_all_selected_defaults_and_fixed_repair(self):
        source = tuning.source_check()
        self.assertIn("constexpr double repair_weight = 0.83;", source)
        def without_defaults(text):
            for p in tuning.PARAMETERS:
                text = re.sub(rf"^#define {p.macro} .+$", f"#define {p.macro} VALUE", text, flags=re.MULTILINE)
            return text
        for config in tuning.CONFIGS:
            changed = tuning.source_with_parameters(source, config)
            self.assertEqual(without_defaults(changed), without_defaults(source))
            for p, level in zip(tuning.PARAMETERS, config):
                self.assertIn(f"#define {p.macro} {tuning.literal(p.values[level])}\n", changed)
        with self.assertRaises(ValueError):
            tuning.parameter_values((0, 0, 0, 0, 0, 8))

    def test_remaining_slimes_are_rejected(self):
        with tempfile.TemporaryDirectory(prefix="v075-validation-", dir=tuning.ROOT / "adhoc") as folder:
            root = Path(folder)
            answer = root / "0000.txt"
            answer.write_text("control test, not a contest answer\n")
            counts = {key: 0 for key in common.ERRORS}
            counts.update(E=1, T=1, state_pool_free_at_end=4, state_slots=4)
            answer.with_suffix(".txt.err").write_text(
                "".join(f"[summary.count] {key}={value}\n" for key, value in counts.items()))
            result = tuning.evaluator.CaseResult("0000.txt", "ok", 100001, 10,
                                                str(answer.relative_to(tuning.ROOT)))
            with patch.object(tuning.evaluator, "run_case", return_value=result):
                checked = tuning.checked_case(root / "fake-input", root / "fake-bin", root / "fake-score", root)
            self.assertEqual(checked.status, "validation_fail")
            self.assertIsNone(checked.score)

    def test_full_fake_run_warmup_exclusion_deduplication_and_completion(self):
        with tempfile.TemporaryDirectory(prefix="v075-control-", dir=tuning.ROOT / "adhoc") as folder:
            root = Path(folder)
            records, summary = root / "official.jsonl", root / "official.csv"
            note, backlog = root / "note.md", root / "backlog.md"
            note.write_text("# fake preregistration\n")
            backlog.write_text("## 実験中\n\n- **[B-95] fake**\n\n## 決着済み\n")
            with patch.object(tuning, "RUN", root), patch.object(tuning, "CASE_COUNT", 4), \
                 patch.object(tuning, "NOTE", note), patch.object(tuning, "BACKLOG", backlog), \
                 patch.object(tuning.evaluator, "RECORDS_JSONL", records), \
                 patch.object(tuning.evaluator, "SUMMARY_CSV", summary):
                run = tuning.TuningRun()
                run.verify_frozen = lambda: None
                run.input_dir = root / "input"
                run.input_dir.mkdir()
                run.inputs = [run.input_dir / f"{i:04d}.txt" for i in range(4)]
                run.scorer = root / "fake-scorer"
                for path in run.inputs:
                    path.write_text("control test, not a contest input\n")
                snapshot = root / "snapshot/src/bin" / tuning.SOURCE.name
                snapshot.parent.mkdir(parents=True)
                snapshot.write_text(tuning.SOURCE.read_text())
                goal = tuning.CONFIGS[4]
                def fake_case(path, binary, scorer, output):
                    config_id = int(binary.name.rsplit("_", 1)[1])
                    score = 100 + sum((a - b) ** 2 for a, b in zip(tuning.CONFIGS[config_id], goal))
                    elapsed = 10
                    if config_id == 5:
                        score, elapsed = 1, 2001
                    answer = output / path.name
                    answer.write_text("control test output\n")
                    answer.with_suffix(".txt.err").write_text(
                        "".join(f"[summary.count] {key}=1\n" for key in tuning.COUNTERS))
                    return tuning.evaluator.CaseResult(path.name, "ok", score, elapsed,
                                                       str(answer.relative_to(tuning.ROOT)))
                def fake_compile(source, binary, mode, config, directory, use_defines):
                    self.assertFalse(use_defines)
                    for p, level in zip(tuning.PARAMETERS, config):
                        self.assertIn(f"#define {p.macro} {tuning.literal(p.values[level])}\n", source.read_text())
                    binary.write_text("not executable; control test only\n")
                    return dict(mode=mode, parameters=tuning.parameter_values(config), sha256=tuning.digest(binary))
                tuning.evaluator.ensure_csv_header(summary, tuning.evaluator.SUMMARY_HEADER)
                with patch.object(tuning, "checked_case", fake_case), patch.object(tuning, "compile_solver", fake_compile):
                    run.evaluate("warmup", 0, 1)
                    run.evaluate("warmup", 0, 2)
                    self.assertEqual(run.measured, [])
                    self.assertFalse(records.exists())
                    self.assertFalse((root / "trials.csv").exists())
                    self.assertEqual(len(summary.read_text().splitlines()), 1)
                    self.assertEqual(list((root / "warmup").iterdir()), [])
                    self.assertNotIn("total_sum", json.loads((root / "warmup_1.json").read_text()))
                    self.assertNotIn("elapsed", json.loads((root / "warmup_2.json").read_text()))
                    for ordinal, (_, config_id) in enumerate(tuning.evaluation_plan()[2:], 1):
                        run.evaluate("measure", config_id, ordinal)
                    with self.assertRaisesRegex(RuntimeError, "再測定"):
                        run.evaluate("measure", 0, 7)
                    run.finish()
                self.assertEqual(len(records.read_text().splitlines()), 24)
                self.assertEqual(len(summary.read_text().splitlines()), 7)
                self.assertEqual(len((root / "cases.jsonl").read_text().splitlines()), 24)
                self.assertEqual(len((root / "mechanism.jsonl").read_text().splitlines()), 6)
                best = json.loads((root / "best.json").read_text())
                self.assertEqual(best["best"]["config_id"], 4)
                self.assertEqual(best["repair_weight"], 0.83)
                self.assertEqual(best["baseline"]["config_id"], 0)
                self.assertEqual(best["selected_source_sha256"], tuning.digest(root / "best" / tuning.SOURCE.name))
                self.assertEqual(json.loads((root / "status.json").read_text())["status"], "completed")
                self.assertIn("## 実験後", note.read_text())
                self.assertIn("[B-95]", backlog.read_text().split("## 決着済み")[1])
                with (root / "single_parameter_curves.csv").open() as stream:
                    effects = list(csv.DictReader(stream))
                self.assertEqual(len(effects), 6)
                self.assertEqual(Counter(r["parameter"] for r in effects), {"removal_size_exponent": 6})


if __name__ == "__main__":
    unittest.main()
