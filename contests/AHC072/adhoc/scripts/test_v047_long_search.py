#!/usr/bin/env python3
"""探索を実行せず、保存形式・独立再生・2プロセス制限・停止を確認する。"""

from concurrent.futures import ThreadPoolExecutor
from contextlib import redirect_stderr, redirect_stdout
import io
import json
from pathlib import Path
import sys
import tempfile
import textwrap
import time
import unittest

import run_v047_long_search as runner

PLAN = "".join(f"0 {j} 0 R 1\n" for j in (0, 2, 4, 6))
INPUT = "12 4\naAbBcCdD....\n" + "............\n" * 11


class RunnerTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="v047_runner_test_")
        self.root = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def problem(self):
        path = self.root / "input.txt"
        path.write_text(INPUT)
        return runner.Problem.read(path)

    def run_fixture(self, fail=False):
        """代役は既存の操作列を転記するだけで、入力盤面を読まず探索しない。"""
        base = self.root / "cases/0001"
        (base / "seeds").mkdir(parents=True)
        (base / "input.txt").write_text(INPUT)
        (base / "seeds/00.txt").write_text(PLAN)
        case = dict(case="0001", input="cases/0001/input.txt", reference_T=4,
                    seeds=[dict(path="cases/0001/seeds/00.txt")])
        manifest = dict(cases=[case], config=dict(minutes_per_case=0.01, restart_seconds=0.18,
                                                  progress_seconds=0.03, seed=123, workers=2))
        runner.write_json(self.root / "manifest.json", manifest)
        fake = self.root / "recording_stub.py"
        body = r'''
import argparse, json, pathlib, signal, sys, time
p=argparse.ArgumentParser()
for key in ("initial-plan","output-dir","seconds","seed","progress-seconds"):
    p.add_argument("--"+key,required=True)
a=p.parse_args(); root=pathlib.Path(a.output_dir); root.mkdir()
(root/"solutions").mkdir(); text=pathlib.Path(a.initial_plan).read_text()
(root/"solutions/000000.txt").write_text(text); (root/"best.txt").write_text(text)
stop=False
def interrupt(*unused):
    global stop
    stop=True
signal.signal(signal.SIGTERM,interrupt)
signal.signal(signal.SIGINT,interrupt)
start=time.monotonic()
with (root/"events.jsonl").open("w") as log:
    def emit(value):
        value.update(elapsed_sec=time.monotonic()-start,cpu_sec=0,peak_rss_bytes=0)
        log.write(json.dumps(value)+"\n");log.flush()
    emit(dict(type="initial",T=4,E=0,plan="solutions/000000.txt",reason="saved_seed",before_plan=""))
    if FAIL and "continuous" in str(root):
        sys.exit(2)
    while time.monotonic()-start<float(a.seconds) and not stop:
        time.sleep(0.01)
    emit(dict(type="finish",status="interrupted" if stop else "time_limit",T=4,E=0))
sys.stdout.write(text)
sys.exit(130 if stop else 0)
'''
        fake.write_text(f"#!{sys.executable}\nFAIL={fail!r}\n" + textwrap.dedent(body))
        fake.chmod(0o755)
        return case, manifest, fake

    def test_saved_plan_and_illegal_distance(self):
        problem = self.problem()
        self.assertEqual(problem.replay(PLAN)["T"], 4)
        self.assertEqual(problem.replay(PLAN)["E"], 0)
        with self.assertRaises(ValueError):
            problem.replay(PLAN.replace("0 0 0 R 1", "0 0 0 R 2"))
        with self.assertRaises(ValueError):
            problem.replay(PLAN.splitlines()[0])

    def test_wall_and_capacity_are_checked_before_home(self):
        problem = self.problem()
        problem.C[1] = "#..........."
        with self.assertRaises(ValueError):
            problem.replay("0 0 0 D 1\n")
        # Nine same-colored residents must not be accepted just because the
        # destination is their nest. They are merged before home processing.
        problem.C[1] = "............"
        problem.initial = {(1, j): 0 for j in range(9)} | {(0, 0): 1}
        problem.nests[(1, 8)] = 1
        collect = "".join(f"1 {j} 0 R 1\n" for j in range(8))
        with self.assertRaisesRegex(ValueError, "容量超過"):
            problem.replay(collect)

    def test_workers_and_nonfinite_arguments(self):
        self.assertEqual(runner.parse_args([]).workers, 2)
        for argv in (["--workers", "3"], ["--minutes-per-case", "nan"], ["--cases", "0001", "0001"]):
            with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                runner.parse_args(argv)

    def test_partial_jsonl_and_atomic_replace(self):
        path = self.root / "events.jsonl"
        path.write_text('{"a":1}\n{"a":')
        rows, offset = runner.read_events(path)
        self.assertEqual(rows, [{"a": 1}])
        with path.open("a") as f:
            f.write("2}\n")
        self.assertEqual(runner.read_events(path, offset)[0], [{"a": 2}])
        runner.atomic_text(path, "replacement")
        self.assertEqual(path.read_text(), "replacement")
        self.assertFalse(list(self.root.glob("*.tmp")))

    def test_two_workers_and_distinct_round_seeds(self):
        _, manifest, fake = self.run_fixture()
        with redirect_stdout(io.StringIO()):
            code = runner.run_search(self.root, manifest, fake)
        self.assertEqual(code, 0)
        status = json.loads((self.root / "status.json").read_text())
        self.assertEqual(status["max_active_search_processes"], 2)
        metas = [json.loads(p.read_text()) for p in self.root.glob("cases/*/*/round_*/round.json")]
        continuous = [m for m in metas if m["mode"] == "continuous"]
        multistart = [m for m in metas if m["mode"] == "multistart"]
        self.assertEqual(len(continuous), 1)
        self.assertGreaterEqual(len(multistart), 2)
        self.assertEqual(len({m["seed"] for m in metas}), len(metas))
        self.assertEqual((self.root / "cases/0001/best.txt").read_text(), PLAN)
        self.assertTrue((self.root / "summary.csv").exists())

    def test_failure_stops_other_worker(self):
        _, manifest, fake = self.run_fixture(fail=True)
        with redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            code = runner.run_search(self.root, manifest, fake)
        self.assertEqual(code, 1)
        self.assertEqual(json.loads((self.root / "status.json").read_text())["status"], "failed")
        self.assertEqual((self.root / "cases/0001/best.txt").read_text(), PLAN)
        states = [json.loads(p.read_text())["status"] for p in self.root.glob("cases/0001/*/status.json")]
        self.assertIn("failed", states)
        self.assertNotIn("running", states)

    def test_improvement_history_and_cutoff(self):
        case, manifest, _ = self.run_fixture()
        before = "0 0 0 D 1\n1 0 0 U 1\n" + PLAN
        (self.root / case["seeds"][0]["path"]).write_text(before)
        case["reference_T"] = 6
        manifest["config"]["minutes_per_case"] = 5 / 60
        runner.write_json(self.root / "manifest.json", manifest)
        worker = self.root / "cases/0001/continuous"
        folder = worker / "round_0000"
        search = folder / "search"
        (search / "solutions").mkdir(parents=True)
        (search / "before").mkdir()
        (search / "solutions/000000.txt").write_text(before)
        (search / "solutions/000001.txt").write_text(PLAN)
        (search / "before/000001.txt").write_text(before)
        runner.write_json(worker / "status.json", dict(status="completed", elapsed_sec=5))
        runner.write_json(folder / "round.json", dict(round=0, status="completed", started_offset_sec=0))
        records = [
            dict(type="initial", elapsed_sec=0, T=6, E=0, plan="solutions/000000.txt", before_plan=""),
            dict(type="improvement", elapsed_sec=3, T=4, E=0, saved=2, reason="regular",
                 plan="solutions/000001.txt", before_plan="before/000001.txt", before_T=6,
                 previous_best_plan="solutions/000000.txt", common_prefix_moves=0, common_suffix_moves=4),
            dict(type="finish", elapsed_sec=5, cpu_sec=1, T=4, E=0),
        ]
        (search / "events.jsonl").write_text("".join(json.dumps(row) + "\n" for row in records))
        summaries = runner.summarize(self.root)
        self.assertEqual(summaries[0]["saved"], 2)
        self.assertEqual((self.root / "cases/0001/best.txt").read_text(), PLAN)
        curve = list(runner.csv.DictReader(io.StringIO((self.root / "learning_curve.csv").read_text())))
        self.assertEqual([(float(r["elapsed_sec"]), int(r["best_T"])) for r in curve], [(2, 6), (5, 4)])
        improvement = list(runner.csv.DictReader(io.StringIO((self.root / "improvements.csv").read_text())))[0]
        self.assertTrue(improvement["before_plan"].endswith("before/000001.txt"))

    def test_stop_keeps_plan_and_reaps_process(self):
        case, manifest, fake = self.run_fixture()
        manifest["config"]["minutes_per_case"] = 1
        control = runner.ProcessControl()
        with ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(runner.run_worker, self.root, fake, case, "continuous", manifest["config"], time.monotonic(), control)
            limit = time.monotonic() + 2
            while not control.processes and time.monotonic() < limit:
                time.sleep(0.01)
            control.stop_all()
            future.result(timeout=3)
        self.assertFalse(control.processes)
        state = json.loads((self.root / "cases/0001/continuous/status.json").read_text())
        self.assertEqual(state["status"], "interrupted")
        self.assertEqual((self.root / "cases/0001/continuous/best.txt").read_text(), PLAN)


if __name__ == "__main__":
    unittest.main()
