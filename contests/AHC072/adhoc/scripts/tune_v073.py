#!/usr/bin/env python3
"""Tune one frozen parameter without an LLM; run once on 100 fresh cases."""
from __future__ import annotations

import argparse
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
import csv
from dataclasses import replace
from datetime import datetime
import hashlib
import json
import os
from pathlib import Path
import random
import re
import secrets
import shutil
import signal
import subprocess
import sys
import time
import traceback

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import eval as evaluator
from audit_v057 import FLAGS, MODES
from check_v037_results import ERRORS, log_values

BIN = "v073_repair_weight"
SOURCE = ROOT / "src/bin" / (BIN + ".cpp")
PARENT = ROOT / "src/bin/v071_refactored.cpp"
AUDIT = ROOT / "adhoc/v073_audit"
RUN = ROOT / "results/tuning/v073"
NOTE = ROOT / "notes/experiments/v073.md"
BACKLOG = ROOT / "notes/backlog.md"
BASE_TICK = 8
TICKS = tuple(range(21))
CASE_COUNT = 100
JOBS = 2
ORDER_SEED = 7301
DEFAULT_DEFINE = "#define AHC072_REPAIR_WEIGHT 1.0"


def now():
    return datetime.now().astimezone().isoformat(timespec="seconds")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, data):
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
    temporary.replace(path)


def weight(tick):
    return f"{tick / 8:.3f}"


def evaluation_plan():
    others = [n for n in TICKS if n != BASE_TICK]
    random.Random(ORDER_SEED).shuffle(others)
    return [("warmup", BASE_TICK), ("warmup", BASE_TICK)] + [
        ("measure", n) for n in [BASE_TICK, *others]]


def rank_key(row):
    return row["total_sum"], abs(row["tick"] - BASE_TICK), row["tick"]


def choose_best(rows):
    eligible = [r for r in rows if r["eligible"]]
    if not eligible:
        raise RuntimeError("全候補が時間上限を超えたため、最良候補を決定できない")
    return min(eligible, key=rank_key)


def source_with_weight(source, tick):
    if source.count(DEFAULT_DEFINE) != 1 or tick not in TICKS:
        raise RuntimeError("係数の既定値を一意に置き換えられない")
    return source.replace(DEFAULT_DEFINE, "#define AHC072_REPAIR_WEIGHT " + weight(tick), 1)


def configure_environment():
    # Only case-level concurrency is used, including in compiler/tool children.
    os.environ["CARGO_BUILD_JOBS"] = "2"
    os.environ["OMP_NUM_THREADS"] = "1"
    os.environ["OPENBLAS_NUM_THREADS"] = "1"
    if sys.platform == "darwin":
        if not os.environ.get("SDKROOT"):
            os.environ["SDKROOT"] = subprocess.check_output(
                ["xcrun", "--sdk", "macosx", "--show-sdk-path"], text=True).strip()
        os.environ.setdefault("MACOSX_DEPLOYMENT_TARGET", "15.0")


def execute(command, log):
    with log.open("a") as handle:
        handle.write(json.dumps(command) + "\n")
        handle.flush()
        subprocess.run(command, cwd=ROOT, stdout=handle, stderr=handle, check=True)


def source_check():
    expected = PARENT.read_text()
    for edit in json.loads((AUDIT / "registered_changes.json").read_text()):
        if expected.count(edit["old"]) != 1:
            raise RuntimeError("登録した親ソースとの差分が一致しない")
        expected = expected.replace(edit["old"], edit["new"], 1)
    if SOURCE.read_text() != expected:
        raise RuntimeError("登録範囲外のsolver変更を検出した")
    return expected


def compile_solver(source, binary, mode, tick, directory):
    compiler = os.environ.get("CXX", "g++-15")
    flags = [*FLAGS, *MODES[mode], f"-DAHC072_REPAIR_WEIGHT={weight(tick)}"]
    command = [compiler, *flags, str(source), "-o", str(binary)]
    log = directory / (binary.name + ".build.log")
    execute(command, log)
    expanded = subprocess.check_output(
        [compiler, *flags, "-E", "-P", str(source)], cwd=ROOT, text=True)
    if f"constexpr double repair_weight = {weight(tick)};" not in expanded:
        raise RuntimeError("前処理後の係数が指定値と異なる")
    formula = "cand.priority=(saved-repair_weight*cand.broken_support_jumps+0.25)"
    if formula not in expanded:
        raise RuntimeError("係数を使う候補順位の式を確認できない")
    return dict(mode=mode, tick=tick, repair_weight=tick / 8, command=command,
                binary=str(binary.relative_to(ROOT)), sha256=digest(binary),
                preprocessed_sha256=hashlib.sha256(expanded.encode()).hexdigest())


def compare_parent_preprocessing(mode):
    compiler = os.environ.get("CXX", "g++-15")
    flags = [*FLAGS, *MODES[mode], "-DAHC072_REPAIR_WEIGHT=1.000"]
    expanded = []
    for path in (PARENT, SOURCE):
        text = subprocess.check_output([compiler, *flags, "-E", "-P", str(path)], cwd=ROOT, text=True)
        (AUDIT / (path.stem + "_" + mode + ".ii")).write_text(text)
        expanded.append(text)
    child = expanded[1]
    for declaration in ("constexpr double repair_weight = 1.000;\n",
                        "static_assert(0.0 <= repair_weight && repair_weight <= 2.5);\n"):
        if child.count(declaration) != 1:
            raise RuntimeError("前処理後の追加宣言が一意ではない")
        child = child.replace(declaration, "", 1)
    child = child.replace("saved-repair_weight*cand.broken_support_jumps+0.25",
                          "saved-cand.broken_support_jumps+0.25")
    def normalize(text):
        for path in (PARENT, SOURCE):
            text = text.replace(str(path), "solver.cpp")
            text = text.replace('"' + path.name + '"', '"solver.cpp"')
        return re.sub(r'("solver\.cpp",\s*)\d+(,)', r"\g<1>0\2", text)
    if normalize(expanded[0]) != normalize(child):
        raise RuntimeError(f"{mode}: 登録した差分以外の前処理結果の差を検出した")
    return True


def preflight_check():
    configure_environment()
    source_check()
    args = argparse.Namespace(bin_name=BIN + "_check", jobs=JOBS, wait_lock=False)
    with evaluator.acquire_eval_lock(args, "no_solver_execution"):
        write_json(AUDIT / "preflight.json", dict(passed=False, source_sha256=digest(SOURCE)))
        def build(mode):
            result = compile_solver(SOURCE, AUDIT / ("check_" + mode), mode, BASE_TICK, AUDIT)
            result["registered_preprocessed_difference_only"] = compare_parent_preprocessing(mode)
            return result
        with ThreadPoolExecutor(max_workers=JOBS) as pool:
            builds = list(pool.map(build, MODES))
    result = dict(passed=True, solver_executions=0, source_sha256=digest(SOURCE),
                  parent_sha256=digest(PARENT), builds=builds)
    write_json(AUDIT / "preflight.json", result)
    print("両構成のビルド・登録差分・完全な前処理の確認: OK（solver実行なし）", flush=True)


def parallel_cases(paths, operation, accept):
    """Keep only two cases in flight; fail before scheduling more work."""
    iterator = iter(paths)
    pool = ThreadPoolExecutor(max_workers=JOBS)
    pending = set()
    try:
        for _ in range(JOBS):
            path = next(iterator, None)
            if path is not None:
                pending.add(pool.submit(operation, path))
        while pending:
            done, pending = wait(pending, return_when=FIRST_COMPLETED)
            # Persist/check every completed case before issuing any new case.
            for future in done:
                accept(future.result())
            for _ in done:
                path = next(iterator, None)
                if path is not None:
                    pending.add(pool.submit(operation, path))
    finally:
        for future in pending:
            future.cancel()
        pool.shutdown(wait=True, cancel_futures=True)


def checked_case(path, binary, scorer, output):
    result = evaluator.run_case(path, binary, scorer, output, verbose=False)
    if result.status != "ok":
        return result
    answer = ROOT / result.stdout_path
    error_path = answer.with_suffix(".txt.err")
    try:
        counts, _, diagnostics = log_values(error_path)
        T = len(answer.read_text().splitlines())
        assert not diagnostics and not any(counts[key] for key in ERRORS)
        assert counts["E"] == 0 and T == counts["T"] == result.score <= 100000
        assert counts["state_pool_free_at_end"] == counts["state_slots"] == 4
    except (AssertionError, KeyError, ValueError) as error:
        with error_path.open("a") as handle:
            handle.write(f"tuning validation failed: {type(error).__name__}: {error}\n")
        return replace(result, status="validation_fail", score=None)
    return result


class TuningRun:
    def __init__(self):
        self.measured = []
        self.frozen = {}
        self.inputs = []

    def status(self, state, stage, **extra):
        write_json(RUN / "status.json", dict(status=state, stage=stage, pid=os.getpid(),
                   updated_at=now(), measured_trials=len(self.measured), total_trials=len(TICKS), **extra))
        print(f"{now()} {state}: {stage}", flush=True)

    def verify_frozen(self):
        for name, expected in self.frozen.items():
            if digest(ROOT / name) != expected:
                raise RuntimeError(f"凍結ファイルの変更を検出: {name}")

    def freeze(self):
        sources = [
            SOURCE, PARENT, Path(__file__), ROOT / "adhoc/scripts/test_tune_v073.py",
            ROOT / "scripts/eval.py", ROOT / "adhoc/scripts/audit_v057.py",
            ROOT / "adhoc/scripts/check_v037_results.py", ROOT / "adhoc/scripts/check_v028_two_orders.py",
            ROOT / "src/bin/v000_template.cpp", ROOT / "notes/notations.md",
            ROOT / "notes/important_properties.md", ROOT / "README.md",
            AUDIT / "registered_changes.json", AUDIT / "preflight.json", AUDIT / "control_tests.log"]
        for source in sources:
            rel = source.relative_to(ROOT)
            dest = RUN / "snapshot" / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, dest)
            if digest(source) != digest(dest):
                raise RuntimeError("保存中にファイルが変更された: " + str(source))
            self.frozen[str(rel)] = digest(source)
            self.frozen[str(dest.relative_to(ROOT))] = digest(dest)
        shutil.copy2(NOTE, RUN / "preregistration.md")
        self.frozen[str((RUN / "preregistration.md").relative_to(ROOT))] = digest(RUN / "preregistration.md")
        write_json(RUN / "plan.json", dict(
            case_count=CASE_COUNT, jobs=JOBS, threads_per_process=1, one_off=True,
            parameter="repair_weight", weights=[n / 8 for n in TICKS], order_seed=ORDER_SEED,
            phases=[dict(phase=phase, tick=tick, repair_weight=tick / 8) for phase, tick in evaluation_plan()],
            source_sha256=digest(SOURCE), local=True, local_time_ratio=0.80))
        self.frozen[str((RUN / "plan.json").relative_to(ROOT))] = digest(RUN / "plan.json")

    def prepare(self):
        self.freeze()
        evaluator.build_tool("gen")
        evaluator.build_tool(evaluator.SCORER_BIN_NAME)
        (RUN / "bin").mkdir()
        self.scorer = RUN / "bin/vis"
        shutil.copy2(evaluator.TOOLS_BIN_DIR / evaluator.SCORER_BIN_NAME, self.scorer)
        generator = RUN / "bin/gen"
        shutil.copy2(evaluator.TOOLS_BIN_DIR / "gen", generator)
        # High nonzero seeds avoid the hand-written seed 0 and previous ranges.
        seeds = sorted({secrets.randbits(62) | (1 << 62) for _ in range(CASE_COUNT)})
        if len(seeds) != CASE_COUNT:
            raise RuntimeError("seedが重複したため生成を中止した")
        (RUN / "seeds.txt").write_text("".join(f"{seed}\n" for seed in seeds))
        self.input_dir = RUN / "input"
        execute([str(generator), str(RUN / "seeds.txt"), "--dir", str(self.input_dir)],
                RUN / "generation.log")
        self.inputs = sorted(self.input_dir.glob("*.txt"))
        if len(self.inputs) != CASE_COUNT:
            raise RuntimeError("生成した入力が100件ではない")
        previous = {digest(p) for directory in (ROOT / "tools/in", ROOT / "tools/validation1")
                    for p in directory.glob("*.txt")}
        hashes = [digest(p) for p in self.inputs]
        if len(set(hashes)) != CASE_COUNT or set(hashes) & previous:
            raise RuntimeError("新規入力の重複を検出した")
        write_json(RUN / "input_manifest.json", dict(one_off=True, reusable_for_future_evaluation=False,
                   cases=[dict(case=p.name, seed=seed, sha256=value)
                          for p, seed, value in zip(self.inputs, seeds, hashes)]))
        for p in [*self.inputs, generator, self.scorer, RUN / "seeds.txt", RUN / "input_manifest.json"]:
            self.frozen[str(p.relative_to(ROOT))] = digest(p)
        self.status("running", "building_all_candidates")
        source = RUN / "snapshot/src/bin" / SOURCE.name
        def build(tick):
            return compile_solver(source, self.binary(tick), "local", tick, RUN / "bin")
        builds = []
        parallel_cases(TICKS, build, builds.append)
        builds.sort(key=lambda row: row["tick"])
        write_json(RUN / "builds.json", builds)
        for build in builds:
            self.frozen[build["binary"]] = build["sha256"]
        write_json(RUN / "frozen.json", self.frozen)
        evaluator.ensure_csv_header(evaluator.SUMMARY_CSV, evaluator.SUMMARY_HEADER)
        self.verify_frozen()

    def binary(self, tick):
        return RUN / "bin" / f"weight_{tick:02d}"

    def evaluate(self, phase, tick, ordinal):
        self.verify_frozen()
        stage = f"{phase}_{ordinal:02d}_weight_{weight(tick)}"
        self.status("running", stage, repair_weight=tick / 8)
        output = RUN / ("warmup" if phase == "warmup" else "trials") / stage
        output.mkdir(parents=True)
        results = []
        executed = datetime.now().astimezone()
        label = f"v073_tuning_weight={weight(tick)}_trial={ordinal:02d}"
        run_id = evaluator.make_run_id(executed, BIN) + f"_w{tick:02d}"
        def accept(result):
            results.append(result)
            if phase == "measure":
                records = evaluator.make_records([result], run_id, executed.isoformat(timespec="seconds"),
                                                BIN, label, str(self.input_dir.relative_to(ROOT)), True)
                evaluator.append_jsonl(evaluator.RECORDS_JSONL, records)
                evaluator.append_jsonl(RUN / "cases.jsonl",
                    [dict(records[0], trial=ordinal, tick=tick, repair_weight=tick / 8)])
            if result.status != "ok":
                raise RuntimeError(f"{stage}: {result.case_name}: {result.status}")
        parallel_cases(self.inputs, lambda p: checked_case(p, self.binary(tick), self.scorer, output), accept)
        if len(results) != CASE_COUNT:
            raise RuntimeError("評価件数の不一致")
        self.verify_frozen()
        if phase == "warmup":
            # No warmup scores, times, or rows enter measured/official aggregates.
            write_json(RUN / f"warmup_{ordinal}.json", dict(completed=True, cases=CASE_COUNT,
                       repair_weight=tick / 8, excluded_from_measurement=True))
            shutil.rmtree(output)
            return
        avg, total, minimum, maximum, avg_ms, max_ms = evaluator.summarize(results)
        evaluator.append_csv_row(evaluator.SUMMARY_CSV, [
            BIN, avg, total, minimum, maximum, avg_ms, max_ms,
            str(self.input_dir.relative_to(ROOT)), CASE_COUNT, label, executed.isoformat(timespec="seconds")])
        row = dict(trial=ordinal, tick=tick, repair_weight=tick / 8, total_sum=total,
                   mean_score=total / CASE_COUNT, max_elapsed_ms=max_ms, eligible=max_ms <= 2000,
                   run_id=run_id, output_dir=str(output.relative_to(ROOT)),
                   baseline_delta=total - (self.measured[0]["total_sum"] if self.measured else total))
        self.measured.append(row)
        with (RUN / "trials.csv").open("w", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(row))
            writer.writeheader()
            writer.writerows(self.measured)
        write_json(RUN / "trials.json", self.measured)
        print(f"  weight={weight(tick)} total={total} delta={row['baseline_delta']:+d} eligible={row['eligible']}",
              flush=True)

    def finish(self):
        self.verify_frozen()
        if len(self.measured) != len(TICKS) or self.measured[0]["tick"] != BASE_TICK:
            raise RuntimeError("測定計画が完了していない")
        best = choose_best(self.measured)
        baseline = self.measured[0]
        folder = RUN / "best"
        folder.mkdir()
        selected_source = folder / SOURCE.name
        frozen_source = (RUN / "snapshot/src/bin" / SOURCE.name).read_text()
        selected_source.write_text(source_with_weight(frozen_source, best["tick"]))
        # Compile the exported artifact after all measurements; no new evaluation.
        self.status("running", "building_selected_artifact", repair_weight=best["repair_weight"])
        with ThreadPoolExecutor(max_workers=JOBS) as pool:
            selected_builds = list(pool.map(lambda mode: compile_solver(
                selected_source, folder / (BIN + "_" + mode), mode, best["tick"], folder), MODES))
        result = dict(completed=True, parameter="repair_weight", best=best, baseline=baseline,
                      improved_on_tuning_set=best["total_sum"] < baseline["total_sum"],
                      delta_sum=best["total_sum"] - baseline["total_sum"],
                      measured_trials=len(self.measured), warmup_evaluations=2,
                      fresh_cases=CASE_COUNT, jobs=JOBS, one_off=True, generalization_verified=False,
                      selected_source=str(selected_source.relative_to(ROOT)),
                      selected_source_sha256=digest(selected_source), selected_builds=selected_builds)
        write_json(RUN / "best.json", result)
        report = ["# v073 パラメーター調整結果", "",
                  f"- 最良の減点係数: {best['repair_weight']}",
                  f"- 合計スコア: {baseline['total_sum']} → {best['total_sum']}（{result['delta_sum']:+d}）",
                  f"- 最良候補の最大外部時間: {best['max_elapsed_ms']} ms",
                  "- 今回専用の新規100件。ウォームアップ2周を除外し、21候補を各1回測定した。",
                  "- 選択に使った100件での結果であり、未使用入力への改善は未検証である。",
                  "", "| 係数 | 合計スコア | 基準との差 | 最大ms | 時間条件 |",
                  "| ---: | ---: | ---: | ---: | :--- |"]
        report += [f"| {r['repair_weight']} | {r['total_sum']} | {r['baseline_delta']:+d} | "
                   f"{r['max_elapsed_ms']} | {'適合' if r['eligible'] else '上限超過'} |"
                   for r in sorted(self.measured, key=rank_key)]
        (RUN / "report.md").write_text("\n".join(report) + "\n")
        self.verify_frozen()
        self.record_completion(result)
        self.status("completed", "finished", best_repair_weight=best["repair_weight"],
                    delta_sum=result["delta_sum"], input_set_retired=True)

    def record_completion(self, result):
        verdict = "今回の調整用100件で改善" if result["improved_on_tuning_set"] else "今回の調整用100件で改善なし"
        best = result["best"]
        text = (f"\n## 実験後\n\n- 判定: {verdict}。提出基準の置き換えは未判定。\n"
                f"- 結果: 21候補の最良係数は{best['repair_weight']}。同じ新規100件の係数1に対し"
                f"合計{result['delta_sum']:+d}手、最大外部時間{best['max_elapsed_ms']} ms。\n"
                "- 機構確認: 候補ごとの係数・ビルド引数・ハッシュと前処理を照合した。"
                "全測定の公式採点・全帰巣・既存エラー計数・盤面領域返却が正常だった。"
                "ケース並列数は2、ウォームアップ200ケースは集計から除外した。\n"
                "- 考察: 選択に使った入力での最良値であり、未使用入力への改善と最適性は未検証である。"
                "通常のtools/inやvalidation1とのスコア比較は行っていない。\n"
                "- 保存資料: results/tuning/v073/のbest.json、report.md、trials.csv、cases.jsonl、"
                "input_manifest.json、frozen.json。最良係数の単一C++はbest/v073_repair_weight.cpp。\n"
                "- 再開条件: ユーザーから追加の検証指示がある場合。今回の評価セットは再利用しない。\n")
        if "## 実験後" in NOTE.read_text():
            raise RuntimeError("実験後の記録が既に存在する")
        with NOTE.open("a") as handle:
            handle.write(text)
        backlog = BACKLOG.read_text()
        lines = [s for s in backlog.splitlines(True) if s.startswith("- **[B-93]")]
        if len(lines) != 1:
            raise RuntimeError("台帳B-93が一意ではない")
        replacement = (f"- **[B-93] v059系の整理版v071で、修復負担の減点係数を調整する** → "
                       f"[v073](experiments/v073.md) {verdict}。新規100件・2並列、"
                       f"係数1のウォームアップ2周を除外し、21候補を測定した。最良係数"
                       f"{best['repair_weight']}、基準比{result['delta_sum']:+d}手。"
                       "未使用入力での効果は未検証。再開条件: ユーザーの追加検証指示がある場合。"
                       "今回の入力は再利用しない。資料はresults/tuning/v073/。\n")
        backlog = backlog.replace(lines[0], "", 1).replace("## 決着済み\n", "## 決着済み\n\n" + replacement, 1)
        BACKLOG.write_text(backlog)


def worker():
    run = TuningRun()
    keep_awake = None
    try:
        with (RUN / ".worker_claim").open("x") as claim:
            claim.write(str(os.getpid()) + "\n")
    except FileExistsError:
        print("この実験は既に起動済み。workerの再実行を拒否する。", file=sys.stderr)
        return 1
    def interrupted(signum, frame):
        raise KeyboardInterrupt(f"signal {signum}")
    signal.signal(signal.SIGTERM, interrupted)
    try:
        configure_environment()
        source_check()
        check = json.loads((AUDIT / "preflight.json").read_text())
        if not check["passed"] or check["source_sha256"] != digest(SOURCE):
            raise RuntimeError("事前ビルドを確認できない")
        args = argparse.Namespace(bin_name=BIN + "_tuning", jobs=JOBS, wait_lock=False)
        with evaluator.acquire_eval_lock(args, str((RUN / "input").relative_to(ROOT))):
            run.status("running", "preparing")
            if sys.platform == "darwin":
                keep_awake = subprocess.Popen(["/usr/bin/caffeinate", "-i", "-w", str(os.getpid())],
                                               stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
            run.prepare()
            for index, (phase, tick) in enumerate(evaluation_plan()):
                run.evaluate(phase, tick, index + 1 if phase == "warmup" else index - 1)
            run.finish()
    except BaseException as error:
        traceback.print_exc()
        run.status("interrupted" if isinstance(error, KeyboardInterrupt) else "failed", "stopped",
                   error=f"{type(error).__name__}: {error}")
        return 1
    finally:
        if keep_awake is not None:
            keep_awake.terminate()
            keep_awake.wait()
    return 0


def launch():
    source_check()
    report = json.loads((AUDIT / "preflight.json").read_text())
    if not report["passed"] or report["source_sha256"] != digest(SOURCE):
        raise RuntimeError("先にcheckを実行する必要がある")
    RUN.mkdir(parents=True, exist_ok=False)
    with (RUN / "run.log").open("x") as log:
        child = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "worker"], cwd=ROOT,
                                 stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                                 start_new_session=True)
    write_json(RUN / "launch.json", dict(pid=child.pid, started_at=now(), command=sys.argv))
    # Startup handshake only. No score/log polling follows this acknowledgement.
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if (RUN / "status.json").exists():
            status = json.loads((RUN / "status.json").read_text())
            if status["status"] in ("failed", "interrupted"):
                raise RuntimeError(status["error"])
            print(json.dumps(dict(started=True, pid=child.pid, run=str(RUN), status=status["stage"]),
                             ensure_ascii=False))
            return 0
        if child.poll() is not None:
            raise RuntimeError("起動失敗。run.logを確認すること")
        time.sleep(0.05)
    raise RuntimeError(f"起動応答を確認できない。PID={child.pid}。二重起動はしないこと")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("check", "launch", "worker"))
    action = parser.parse_args().action
    if action == "check":
        preflight_check()
    else:
        raise SystemExit(launch() if action == "launch" else worker())
