#!/usr/bin/env python3
"""探索せず、保存解の再生、再開、全100件の登録とviewerの計算を検証する。"""

from contextlib import redirect_stdout, redirect_stderr
import csv
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import run_v800 as runner
from test_v047_long_search import RunnerTest as RecordingFixture

REAL_ROOT = runner.ROOT
PLAN = "".join(f"0 {j} 0 R 1\n" for j in (0, 2, 4, 6))
LONG_PLAN = "0 0 0 D 1\n1 0 0 U 1\n" + PLAN
INPUT = "12 4\naAbBcCdD....\n" + "............\n" * 11


class V800Test(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="v800_test_")
        self.root = Path(self.temp.name).resolve()
        self.addCleanup(self.temp.cleanup)
        self.root_patch = patch.object(runner, "ROOT", self.root)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)
        old_search_root = runner.search.ROOT
        self.addCleanup(setattr, runner.search, "ROOT", old_search_root)
        for name in runner.SNAPSHOTS:
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REAL_ROOT / name, destination)
        inp = self.root / "tools/in"
        saved = self.root / "results/out/v050_joint_towers"
        inp.mkdir(parents=True)
        saved.mkdir(parents=True)
        reuse = self.root / "results/long_search/v047/test"
        cases = []
        for case in runner.CASES:
            (inp / f"{case}.txt").write_text(INPUT)
            (saved / f"{case}.txt").write_text(LONG_PLAN)
            if case in runner.REUSE_CASES:
                base = reuse / "cases" / case
                base.mkdir(parents=True)
                (base / "input.txt").write_text(INPUT)
                (base / "best.txt").write_text(PLAN)
                for mode in runner.search.MODES:
                    runner.search.write_json(base / mode / "status.json", dict(status="completed", elapsed_sec=2700))
                cases.append(dict(case=case, input=f"cases/{case}/input.txt", input_sha256=runner.search.sha256(base / "input.txt")))
        runner.search.write_json(reuse / "status.json", dict(status="completed"))
        runner.search.write_json(reuse / "manifest.json", dict(cases=cases))
        self.run = self.root / "results/long_search/v800/test"
        with redirect_stdout(io.StringIO()):
            self.manifest = runner.prepare(runner.parse_args([]), self.run)

    def complete_attempt(self, attempt, manifest, status="completed", elapsed=300):
        """記録用の代役。固定の合法手順を書き写すだけで探索しない。"""
        for case in manifest["cases"]:
            for mode in runner.search.MODES:
                base = attempt / "cases" / case["case"] / mode
                runner.search.atomic_text(base / "best.txt", PLAN)
                runner.search.write_json(base / "status.json", dict(status=status, elapsed_sec=elapsed, T=4))
        runner.search.write_json(attempt / "status.json", dict(status=status, max_active_search_processes=2))
        return 0 if status == "completed" else 130

    def complete_all(self, except_cases=()):
        for case in self.manifest["cases"]:
            if case["reused"] or case["case"] in except_cases:
                continue
            attempt, manifest = runner.make_attempt(self.run, self.manifest, case)
            self.complete_attempt(attempt, manifest)
            runner.reconcile_attempts(self.run, case)

    def publish(self):
        with redirect_stdout(io.StringIO()):
            runner.publish(self.run, self.manifest, REAL_ROOT / "tools/target/release/vis")

    def test_prepare_has_90_searches_and_10_reuses(self):
        self.assertEqual(len(self.manifest["cases"]), 100)
        self.assertEqual(sum(c["reused"] for c in self.manifest["cases"]), 10)
        self.assertEqual(self.manifest["cases"][0]["case"], "0000")
        self.assertFalse(self.manifest["cases"][0]["reused"])
        self.assertEqual(self.manifest["config"]["minutes_per_case"], 5)
        for c in self.manifest["cases"]:
            self.assertEqual(c["reference_T"], 4 if c["reused"] else 6)
        self.assertFalse((self.root / "results/eval_records.jsonl").exists())
        runner.verify_snapshot(self.run, self.manifest)

    def test_invalid_arguments_and_input_change_are_rejected(self):
        for argv in (["--workers", "3"], ["--seed", "-1"], ["--resume", "x", "--run-dir", "y"]):
            with redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                runner.parse_args(argv)
        (self.root / "tools/in/0000.txt").write_text(INPUT + "\n")
        with self.assertRaisesRegex(ValueError, "入力"):
            runner.verify_snapshot(self.run, self.manifest)

    def test_resume_skips_completed_and_restarts_only_unfinished_case(self):
        self.complete_all(except_cases=("0000", "0001"))
        calls = []

        def stub(attempt, manifest, binary):
            case = manifest["cases"][0]
            calls.append(case["case"])
            if len(calls) == 2:
                return self.complete_attempt(attempt, manifest, "interrupted", 12)
            if len(calls) == 3:
                self.assertEqual(case["reference_T"], 4)
            return self.complete_attempt(attempt, manifest)

        with patch.object(runner.search, "run_search", side_effect=stub), redirect_stdout(io.StringIO()):
            self.assertEqual(runner.run_pending(self.run, self.manifest, Path("unused")), 130)
            self.assertFalse((self.root / "results/eval_records.jsonl").exists())
            self.assertEqual(runner.run_pending(self.run, self.manifest, Path("unused")), 0)
        self.assertEqual(calls, ["0000", "0001", "0001"])
        state = runner.read_json(self.run / "cases/0001/status.json")
        self.assertEqual(state["T"], 4)
        self.assertEqual(state["new_elapsed_ms"], 312000)
        self.assertEqual(len(state["attempts"]), 2)

    def test_process_adapter_uses_two_processes(self):
        self.complete_all(except_cases=("0000",))
        case = self.manifest["cases"][0]
        seed = self.run / case["seeds"][0]["path"]
        seed.write_text(PLAN)
        (self.run / "cases/0000/best.txt").write_text(PLAN)
        case["seeds"] = [case["seeds"][0]]
        self.manifest["config"].update(minutes_per_case=0.01, restart_seconds=0.18, progress_seconds=0.03)
        fixture = RecordingFixture()
        fixture.setUp()
        self.addCleanup(fixture.tearDown)
        _, _, binary = fixture.run_fixture()
        with redirect_stdout(io.StringIO()):
            self.assertEqual(runner.run_pending(self.run, self.manifest, binary), 0)
        attempt = self.run / "cases/0000/attempts/attempt_000"
        self.assertEqual(runner.read_json(attempt / "status.json")["max_active_search_processes"], 2)
        self.assertGreaterEqual(len(list(attempt.glob("cases/0000/multistart/round_*"))), 2)

    def test_incomplete_reference_is_not_published(self):
        self.complete_all(except_cases=("0000",))
        with self.assertRaisesRegex(ValueError, "未完了"):
            self.publish()
        self.assertFalse((self.root / "results/eval_records.jsonl").exists())
        self.assertFalse((self.root / "results/score_summary.csv").exists())

    def test_publication_retry_is_idempotent_and_viewer_uses_reference(self):
        self.complete_all()
        old = [dict(run_id="old", executed_at="2026-09-28", bin="v001", label="old", input_dir="tools/in",
                    case_name=c + ".txt", status="ok", score=8, elapsed=1200) for c in runner.CASES]
        records_path = self.root / "results/eval_records.jsonl"
        records_path.write_text("".join(json.dumps(r) + "\n" for r in old))
        atomic = runner.search.atomic_text

        def fail_commit(path, text):
            if path == records_path:
                raise OSError("interrupted publication")
            return atomic(path, text)

        with patch.object(runner.search, "atomic_text", side_effect=fail_commit):
            with self.assertRaisesRegex(OSError, "interrupted publication"):
                self.publish()
        self.assertEqual(len(records_path.read_text().splitlines()), 100)
        self.publish()
        saved = records_path.read_bytes()
        self.publish()
        self.assertEqual(saved, records_path.read_bytes())
        records = [json.loads(line) for line in records_path.read_text().splitlines()]
        self.assertEqual(len(records), 200)
        current = [r for r in records if r["bin"] == "v800"]
        self.assertEqual(len(current), 100)
        self.assertEqual(max(r["elapsed"] for r in current), 2700000)
        for name in ("score_summary.csv", "score_detail.csv"):
            with (self.root / "results" / name).open() as handle:
                self.assertEqual(len(list(csv.DictReader(handle))), 1)
        self.assertEqual(len(list((self.root / "results/out/v800").glob("*.txt"))), 100)
        # Exercise the viewer's actual aggregation function, leaving the user's
        # live files and UI untouched. Metadata lookup is irrelevant to scores.
        js = r'''
import fs from 'node:fs'; import vm from 'node:vm';
const source = fs.readFileSync(process.argv[1], 'utf8');
const start = source.indexOf('function buildEvalViewData()');
const end = source.indexOf('function buildEvalViewVersion()', start);
const rows = fs.readFileSync(process.argv[2], 'utf8').trim().split('\n').map(JSON.parse);
const data = vm.runInNewContext(source.slice(start, end) + '\nbuildEvalViewData()', {
  readEvalRecords: () => rows, CASE_SORT_OPTIONS: [], PROJECT_KEY: 'fixture',
  buildCaseMetaByEvalSet: () => ({})
});
process.stdout.write(JSON.stringify(data));
'''
        data = json.loads(subprocess.check_output(["node", "--input-type=module", "-e", js,
                                                   str(REAL_ROOT / "vite.config.js"), str(records_path)], text=True))
        by_bin = {r["bin"]: r for r in data["runsByEvalSet"]["tools/in"]}
        self.assertEqual(by_bin["v800"]["relativeAvg"], 10**9)
        self.assertEqual(by_bin["v001"]["relativeAvg"], 5*10**8)
        self.assertEqual(by_bin["v800"]["maxElapsed"], 2700000)
        self.assertEqual(len(by_bin["v800"]["caseScores"]), 100)


if __name__ == "__main__":
    unittest.main()
