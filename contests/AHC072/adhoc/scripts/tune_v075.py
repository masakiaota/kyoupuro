#!/usr/bin/env python3
"""候補サイズの指数6条件を新規100件で比較する。生成AIの呼び出しは含まない。"""
from __future__ import annotations

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import csv
from dataclasses import asdict, dataclass
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

from tune_v073 import (FLAGS, MODES, checked_case, configure_environment, digest,
                       evaluator, execute, log_values, now, parallel_cases, write_json)

ROOT = Path(__file__).resolve().parents[2]
BIN = "v075_size_exponent"
SOURCE = ROOT / "src/bin" / (BIN + ".cpp")
PARENT = ROOT / "results/tuning/v074/best/v074_multi_tuning.cpp"
PARENT_MANIFEST = ROOT / "results/tuning/v074/best.json"
AUDIT = ROOT / "adhoc/v075_audit"
RUN = ROOT / "results/tuning/v075"
NOTE = ROOT / "notes/experiments/v075.md"
BACKLOG = ROOT / "notes/backlog.md"
CASE_COUNT, JOBS = 100, 2
REPAIR_WEIGHT = 0.83
ORDER_SEED = 7501


@dataclass(frozen=True)
class Parameter:
    name: str
    macro: str
    default: float | int
    values: tuple
    cxx_type: str = "double"


PARAMETERS = (
    Parameter("removal_size_exponent", "AHC072_SIZE_EXPONENT", 0.95, (0.8, 0.95, 1.0, 1.2, 1.4, 1.6)),
    Parameter("priority_noise_amplitude", "AHC072_NOISE_AMPLITUDE", 0.10, (0.10,)),
    Parameter("lns_start_temperature", "AHC072_START_TEMPERATURE", 0.70, (0.70,)),
    Parameter("lns_end_temperature", "AHC072_END_TEMPERATURE", 0.08, (0.08,)),
    Parameter("dependency_interval", "AHC072_DEPENDENCY_INTERVAL", 4, (4,), "int"),
    Parameter("regular_cooldown_fraction", "AHC072_COOLDOWN_FRACTION", 0.50, (0.50,)),
)
BASE = tuple(p.values.index(p.default) for p in PARAMETERS)
COUNTERS = ("lns_attempts", "lns_insertions", "lns_uphill", "lns_restarts",
            "lns_repair_priority_evaluated", "lns_dependency_attempts", "lns_dependency_completed")


def configurations():
    return [BASE, *[(level, *BASE[1:]) for level in range(len(PARAMETERS[0].values))
                   if level != BASE[0]]]


CONFIGS = configurations()


def parameter_values(config):
    if len(config) != len(PARAMETERS) or any(
            type(level) is not int or not 0 <= level < len(p.values)
            for p, level in zip(PARAMETERS, config)):
        raise ValueError("設定の水準が探索範囲外")
    return {p.name: p.values[level] for p, level in zip(PARAMETERS, config)}


def literal(value):
    text = format(value, ".12g")
    return text if isinstance(value, int) or "." in text else text + ".0"


def evaluation_plan():
    others = list(range(1, len(CONFIGS)))
    random.Random(ORDER_SEED).shuffle(others)
    return [("warmup", 0), ("warmup", 0), ("measure", 0)] + [("measure", n) for n in others]


def rank_key(row):
    value = parameter_values(CONFIGS[row["config_id"]])["removal_size_exponent"]
    return row["total_sum"], abs(value - PARAMETERS[0].default), value


def choose_best(rows):
    eligible = [row for row in rows if row["eligible"]]
    if not eligible:
        raise RuntimeError("全候補が時間上限を超えたため最良条件を決定できない")
    return min(eligible, key=rank_key)


def source_with_parameters(source, config):
    values = parameter_values(config)
    for parameter in PARAMETERS:
        pattern = rf"^#define {parameter.macro} (.+)$"
        matches = re.findall(pattern, source, re.MULTILINE)
        if len(matches) != 1 or float(matches[0]) != parameter.default:
            raise RuntimeError("既定値を一意に変更できない: " + parameter.name)
        source = re.sub(pattern, f"#define {parameter.macro} {literal(values[parameter.name])}",
                        source, count=1, flags=re.MULTILINE)
    return source


def source_check():
    if digest(PARENT) != json.loads(PARENT_MANIFEST.read_text())["selected_source_sha256"]:
        raise RuntimeError("v074の選択済みソースのハッシュが一致しない")
    expected = PARENT.read_text()
    for edit in json.loads((AUDIT / "registered_changes.json").read_text()):
        if expected.count(edit["old"]) != 1:
            raise RuntimeError("登録した親ソースとの差分が一意ではない")
        expected = expected.replace(edit["old"], edit["new"], 1)
    if SOURCE.read_text() != expected:
        raise RuntimeError("登録範囲外のsolver変更を検出した")
    return expected


def verify_expanded(expanded, config):
    values = parameter_values(config)
    if "constexpr double repair_weight = 0.83;" not in expanded:
        raise RuntimeError("固定する修復係数が0.83ではない")
    for p in PARAMETERS:
        declaration = f"constexpr {p.cxx_type} {p.name} = {literal(values[p.name])};"
        if expanded.count(declaration) != 1 or expanded.count(p.name) < 4:
            raise RuntimeError("前処理後の値または使用箇所が不一致: " + p.name)
    for expression in (
        "divisor[n]=pow(double(n),removal_size_exponent)",
        "(1.0-priority_noise_amplitude)+2.0*priority_noise_amplitude*rng.unit()",
        "temperature=lns_start_temperature*pow(lns_end_temperature/lns_start_temperature,progress)",
        "++regular_selections%dependency_interval==0",
        "int(candidates.size()*regular_cooldown_fraction)",
    ):
        if expression not in expanded:
            raise RuntimeError("パラメーターが探索式へ接続されていない: " + expression)


def compile_solver(source, binary, mode, config, directory, use_defines=True):
    values = parameter_values(config)
    defines = [f"-D{p.macro}={literal(values[p.name])}" for p in PARAMETERS] if use_defines else []
    compiler = os.environ.get("CXX", "g++-15")
    flags = [*FLAGS, *MODES[mode], *defines]
    command = [compiler, *flags, str(source), "-o", str(binary)]
    execute(command, directory / (binary.name + ".build.log"))
    expanded = subprocess.check_output([compiler, *flags, "-E", "-P", str(source)], cwd=ROOT, text=True)
    verify_expanded(expanded, config)
    return dict(mode=mode, levels=list(config), parameters=values, repair_weight=REPAIR_WEIGHT,
                command=command, binary=str(binary.relative_to(ROOT)), sha256=digest(binary),
                preprocessed_sha256=hashlib.sha256(expanded.encode()).hexdigest())


def compare_parent_preprocessing(mode):
    compiler = os.environ.get("CXX", "g++-15")
    flags = [*FLAGS, *MODES[mode]]
    texts = []
    for path in (PARENT, SOURCE):
        expanded = subprocess.check_output([compiler, *flags, "-E", "-P", str(path)], cwd=ROOT, text=True)
        (AUDIT / (path.stem + "_" + mode + ".ii")).write_text(expanded)
        texts.append(expanded)
    child = texts[1]
    for edit in json.loads((AUDIT / "registered_changes.json").read_text()):
        if edit["old"].startswith("//"):
            continue
        if child.count(edit["new"]) != 1:
            raise RuntimeError("前処理後の登録差分を一意に戻せない")
        child = child.replace(edit["new"], edit["old"], 1)
    def normalize(text):
        for path in (PARENT, SOURCE):
            text = text.replace(str(path), "solver.cpp").replace('"' + path.name + '"', '"solver.cpp"')
        return re.sub(r'("solver\.cpp",\s*)\d+(,)', r"\g<1>0\2", text)
    if normalize(texts[0]) != normalize(child):
        raise RuntimeError(mode + ": 登録した差分以外の前処理結果の差を検出した")
    return True


def preflight_check():
    configure_environment()
    source_check()
    args = argparse.Namespace(bin_name=BIN + "_check", jobs=JOBS, wait_lock=False)
    with evaluator.acquire_eval_lock(args, "no_solver_execution"):
        write_json(AUDIT / "preflight.json", dict(passed=False, source_sha256=digest(SOURCE)))
        def build(item):
            mode, edge = item
            config = CONFIGS[-1] if edge else BASE
            result = compile_solver(SOURCE, AUDIT / f"check_{mode}_{int(edge)}", mode, config, AUDIT)
            if not edge:
                result["registered_preprocessed_difference_only"] = compare_parent_preprocessing(mode)
            return result
        builds = []
        parallel_cases([(mode, edge) for edge in (False, True) for mode in MODES], build, builds.append)
    write_json(AUDIT / "preflight.json", dict(passed=True, solver_executions=0,
               source_sha256=digest(SOURCE), parent_sha256=digest(PARENT),
               runner_sha256=digest(Path(__file__)), builds=builds))
    print("LOCAL・非LOCAL、基準・境界設定、登録差分・完全前処理: OK（solver実行なし）", flush=True)


def check_preflight():
    source_check()
    report = json.loads((AUDIT / "preflight.json").read_text())
    if not report["passed"] or report["source_sha256"] != digest(SOURCE) or \
            report["parent_sha256"] != digest(PARENT) or report["runner_sha256"] != digest(Path(__file__)):
        raise RuntimeError("ソースと実行コードが一致する事前確認が必要")


class TuningRun:
    def __init__(self):
        self.measured, self.inputs = [], []
        self.frozen = {}

    def status(self, state, stage, **extra):
        write_json(RUN / "status.json", dict(status=state, stage=stage, pid=os.getpid(), updated_at=now(),
                   measured_trials=len(self.measured), total_trials=len(CONFIGS), **extra))
        print(f"{now()} {state}: {stage}", flush=True)

    def verify_frozen(self):
        for name, expected in self.frozen.items():
            if digest(ROOT / name) != expected:
                raise RuntimeError("凍結ファイルの変更を検出: " + name)

    def freeze(self):
        files = [SOURCE, PARENT, PARENT_MANIFEST, Path(__file__), ROOT / "adhoc/scripts/test_tune_v075.py",
                 ROOT / "adhoc/scripts/tune_v073.py", ROOT / "scripts/eval.py",
                 ROOT / "adhoc/scripts/audit_v057.py", ROOT / "adhoc/scripts/check_v037_results.py",
                 ROOT / "adhoc/scripts/check_v028_two_orders.py", ROOT / "src/bin/v000_template.cpp",
                 ROOT / "notes/notations.md", ROOT / "notes/important_properties.md",
                 ROOT / "AGENTS.md", ROOT / "ahc-llm-rules-en.txt", AUDIT / "registered_changes.json",
                 AUDIT / "preflight.json", AUDIT / "control_tests.log"]
        for source in files:
            rel = source.relative_to(ROOT)
            target = RUN / "snapshot" / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)
            if digest(source) != digest(target):
                raise RuntimeError("保存中に変更されたファイル: " + str(source))
            self.frozen[str(rel)] = digest(source)
            self.frozen[str(target.relative_to(ROOT))] = digest(target)
        shutil.copy2(NOTE, RUN / "preregistration.md")
        write_json(RUN / "plan.json", dict(case_count=CASE_COUNT, jobs=JOBS, threads_per_process=1,
            repair_weight=REPAIR_WEIGHT, parameters=[asdict(p) for p in PARAMETERS],
            order_seed=ORDER_SEED, local=True, local_time_ratio=0.80, one_off=True,
            configurations=[dict(config_id=i, kind="baseline" if i == 0 else "single",
                                 levels=list(c), parameters=parameter_values(c)) for i, c in enumerate(CONFIGS)],
            phases=[dict(phase=p, config_id=c) for p, c in evaluation_plan()]))
        for path in (RUN / "preregistration.md", RUN / "plan.json"):
            self.frozen[str(path.relative_to(ROOT))] = digest(path)

    def prepare(self):
        self.freeze()
        evaluator.build_tool("gen")
        evaluator.build_tool(evaluator.SCORER_BIN_NAME)
        (RUN / "bin").mkdir()
        self.scorer = RUN / "bin/vis"
        generator = RUN / "bin/gen"
        shutil.copy2(evaluator.TOOLS_BIN_DIR / evaluator.SCORER_BIN_NAME, self.scorer)
        shutil.copy2(evaluator.TOOLS_BIN_DIR / "gen", generator)
        seeds = sorted({secrets.randbits(62) | (1 << 62) for _ in range(CASE_COUNT)})
        if len(seeds) != CASE_COUNT:
            raise RuntimeError("seedの重複を検出した")
        (RUN / "seeds.txt").write_text("".join(f"{s}\n" for s in seeds))
        self.input_dir = RUN / "input"
        execute([str(generator), str(RUN / "seeds.txt"), "--dir", str(self.input_dir)], RUN / "generation.log")
        self.inputs = sorted(self.input_dir.glob("*.txt"))
        if len(self.inputs) != CASE_COUNT:
            raise RuntimeError("生成した入力が100件ではない")
        previous = {digest(p) for folder in (ROOT / "tools/in", ROOT / "tools/validation1")
                    for p in folder.glob("*.txt")}
        for manifest in (ROOT / "results/tuning").glob("*/input_manifest.json"):
            if manifest.parent != RUN:
                previous.update(case["sha256"] for case in json.loads(manifest.read_text())["cases"])
        hashes = [digest(p) for p in self.inputs]
        if len(set(hashes)) != CASE_COUNT or set(hashes) & previous:
            raise RuntimeError("既存または新規入力の重複を検出した")
        write_json(RUN / "input_manifest.json", dict(one_off=True, reusable_for_future_evaluation=False,
            cases=[dict(case=p.name, seed=seed, sha256=h) for p, seed, h in zip(self.inputs, seeds, hashes)]))
        for path in [*self.inputs, generator, self.scorer, RUN / "seeds.txt", RUN / "input_manifest.json"]:
            self.frozen[str(path.relative_to(ROOT))] = digest(path)
        self.status("running", "building_all_candidates")
        source = RUN / "snapshot/src/bin" / SOURCE.name
        builds = []
        def build(config_id):
            return dict(config_id=config_id, **compile_solver(source, self.binary(config_id), "local",
                        CONFIGS[config_id], RUN / "bin"))
        parallel_cases(range(len(CONFIGS)), build, builds.append)
        builds.sort(key=lambda row: row["config_id"])
        write_json(RUN / "builds.json", builds)
        for build in builds:
            self.frozen[build["binary"]] = build["sha256"]
        self.frozen[str((RUN / "builds.json").relative_to(ROOT))] = digest(RUN / "builds.json")
        write_json(RUN / "frozen.json", self.frozen)
        evaluator.ensure_csv_header(evaluator.SUMMARY_CSV, evaluator.SUMMARY_HEADER)
        self.verify_frozen()

    def binary(self, config_id):
        return RUN / "bin" / f"config_{config_id:02d}"

    def evaluate(self, phase, config_id, ordinal):
        if phase not in ("warmup", "measure") or config_id not in range(len(CONFIGS)):
            raise ValueError("未知の評価条件")
        if phase == "measure" and any(r["config_id"] == config_id for r in self.measured):
            raise RuntimeError("同一設定の再測定を拒否する")
        self.verify_frozen()
        stage = f"{phase}_{ordinal:02d}_config_{config_id:02d}"
        values = parameter_values(CONFIGS[config_id])
        self.status("running", stage, config_id=config_id, parameters=values)
        output = RUN / ("warmup" if phase == "warmup" else "trials") / stage
        output.mkdir(parents=True, exist_ok=False)
        results, counters = [], Counter()
        executed = datetime.now().astimezone()
        label = f"v075_tuning_config={config_id:02d}_trial={ordinal:02d}"
        run_id = evaluator.make_run_id(executed, BIN) + f"_c{config_id:02d}"
        def accept(result):
            results.append(result)
            if phase == "measure":
                records = evaluator.make_records([result], run_id, executed.isoformat(timespec="seconds"),
                            BIN, label, str(self.input_dir.relative_to(ROOT)), True)
                evaluator.append_jsonl(evaluator.RECORDS_JSONL, records)
                evaluator.append_jsonl(RUN / "cases.jsonl", [dict(records[0], trial=ordinal,
                    config_id=config_id, repair_weight=REPAIR_WEIGHT, parameters=values)])
            if result.status != "ok":
                raise RuntimeError(f"{stage}: {result.case_name}: {result.status}")
            counts, _, _ = log_values((ROOT / result.stdout_path).with_suffix(".txt.err"))
            counters.update({key: counts[key] for key in COUNTERS})
        parallel_cases(self.inputs, lambda p: checked_case(p, self.binary(config_id), self.scorer, output), accept)
        if len(results) != CASE_COUNT:
            raise RuntimeError("評価件数の不一致")
        self.verify_frozen()
        if phase == "warmup":
            write_json(RUN / f"warmup_{ordinal}.json", dict(completed=True, cases=CASE_COUNT,
                       config_id=config_id, repair_weight=REPAIR_WEIGHT, parameters=values,
                       excluded_from_measurement=True))
            shutil.rmtree(output)
            return
        for key in ("lns_attempts", "lns_repair_priority_evaluated", "lns_dependency_attempts"):
            if counters[key] == 0:
                raise RuntimeError("既存計数による機構の発動確認に失敗: " + key)
        avg, total, minimum, maximum, avg_ms, max_ms = evaluator.summarize(results)
        evaluator.append_csv_row(evaluator.SUMMARY_CSV, [BIN, avg, total, minimum, maximum, avg_ms, max_ms,
            str(self.input_dir.relative_to(ROOT)), CASE_COUNT, label, executed.isoformat(timespec="seconds")])
        row = dict(trial=ordinal, config_id=config_id, repair_weight=REPAIR_WEIGHT, **values,
                   total_sum=total, mean_score=total / CASE_COUNT, max_elapsed_ms=max_ms,
                   eligible=max_ms <= 2000, run_id=run_id, output_dir=str(output.relative_to(ROOT)),
                   baseline_delta=total - (self.measured[0]["total_sum"] if self.measured else total))
        self.measured.append(row)
        evaluator.append_jsonl(RUN / "mechanism.jsonl", [dict(config_id=config_id, counters=dict(counters))])
        with (RUN / "trials.csv").open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(row))
            writer.writeheader()
            writer.writerows(self.measured)
        write_json(RUN / "trials.json", self.measured)
        print(f"  config={config_id:02d} total={total} delta={row['baseline_delta']:+d} eligible={row['eligible']}", flush=True)

    def finish(self):
        self.verify_frozen()
        if len(self.measured) != len(CONFIGS) or self.measured[0]["config_id"] != 0:
            raise RuntimeError("全条件の測定が完了していない")
        best, baseline = choose_best(self.measured), self.measured[0]
        folder = RUN / "best"
        folder.mkdir()
        selected = folder / SOURCE.name
        config = CONFIGS[best["config_id"]]
        selected.write_text(source_with_parameters((RUN / "snapshot/src/bin" / SOURCE.name).read_text(), config))
        self.status("running", "building_selected_artifact", config_id=best["config_id"])
        with ThreadPoolExecutor(max_workers=JOBS) as pool:
            builds = list(pool.map(lambda mode: compile_solver(selected, folder / (BIN + "_" + mode),
                              mode, config, folder, use_defines=False), MODES))
        result = dict(completed=True, repair_weight=REPAIR_WEIGHT, best=best, baseline=baseline,
                      best_parameters=parameter_values(config),
                      improved_on_tuning_set=best["total_sum"] < baseline["total_sum"],
                      delta_sum=best["total_sum"] - baseline["total_sum"],
                      measured_trials=len(self.measured), warmup_evaluations=2, fresh_cases=CASE_COUNT,
                      jobs=JOBS, one_off=True, generalization_verified=False,
                      selected_source=str(selected.relative_to(ROOT)), selected_source_sha256=digest(selected),
                      selected_builds=builds)
        write_json(RUN / "best.json", result)
        self.write_report(result)
        self.verify_frozen()
        self.record_completion(result)
        self.status("completed", "finished", best_config_id=best["config_id"],
                    best_parameters=result["best_parameters"], delta_sum=result["delta_sum"], input_set_retired=True)

    def write_report(self, result):
        by_id = {row["config_id"]: row for row in self.measured}
        effects = []
        for axis, parameter in enumerate(PARAMETERS):
            if len(parameter.values) == 1:
                continue
            for level, value in enumerate(parameter.values):
                config = list(BASE)
                config[axis] = level
                row = by_id[CONFIGS.index(tuple(config))]
                effects.append(dict(parameter=parameter.name, value=value, config_id=row["config_id"],
                    total_sum=row["total_sum"], baseline_delta=row["baseline_delta"], eligible=row["eligible"]))
        with (RUN / "single_parameter_curves.csv").open("w", newline="") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(effects[0]))
            writer.writeheader()
            writer.writerows(effects)
        best, baseline = result["best"], result["baseline"]
        report = ["# v075 候補サイズの指数の調整結果", "",
            f"- 修復負担の減点係数は全条件で{REPAIR_WEIGHT}に固定した。",
            f"- 基準からの合計スコア差: {result['delta_sum']:+d}手。",
            f"- 合計スコア: {baseline['total_sum']} → {best['total_sum']}。",
            f"- 最良設定: {best['config_id']:02d}、最大外部時間{best['max_elapsed_ms']} ms。",
            "- 新規100件をケース単位2並列で実行した。基準0.95の2周を除外し、6条件を各1回測定した。",
            "- 選択に使った入力での結果であり、未使用入力への改善は未確認である。", "",
            "| パラメーター | 基準 | 最良設定 |", "| --- | ---: | ---: |",
            *[f"| {p.name} | {p.default} | {result['best_parameters'][p.name]} |" for p in PARAMETERS], "",
            "指数順の曲線用の集計はsingle_parameter_curves.csv、全設定はplan.jsonにある。", "",
            "| 設定ID | 合計スコア | 基準との差 | 最大ms | 時間条件 |",
            "| ---: | ---: | ---: | ---: | :--- |",
            *[f"| {r['config_id']:02d} | {r['total_sum']} | {r['baseline_delta']:+d} | {r['max_elapsed_ms']} | "
              f"{'適合' if r['eligible'] else '上限超過'} |" for r in sorted(self.measured, key=rank_key)]]
        (RUN / "report.md").write_text("\n".join(report) + "\n")

    def record_completion(self, result):
        verdict = "今回の調整用100件で改善" if result["improved_on_tuning_set"] else "今回の調整用100件で改善なし"
        settings = "、".join(f"{p.name}={result['best_parameters'][p.name]}" for p in PARAMETERS)
        text = (f"\n## 実験後\n\n- 判定: {verdict}。未使用入力と提出基準への効果は未判定。\n"
                f"- 結果: 6条件を測定し、指数0.95の基準に対して合計{result['delta_sum']:+d}手。"
                f"最良設定は{settings}。最大外部時間{result['best']['max_elapsed_ms']} ms。\n"
                "- 機構確認: 全設定の値・使用式・ビルド引数・前処理結果を照合した。全測定の公式採点、全帰巣、"
                "既存エラー計数、盤面領域返却が正常だった。候補評価と依存拡張の発動を各条件で確認し、"
                "既存計数をmechanism.jsonlへ集計した。ウォームアップ200ケースは集計から除外した。\n"
                "- 考察: 同じ新規100件での条件選択であり、係数ごとの最適性や未使用入力での改善は未確認。"
                "指数順の保存結果から頭打ちや悪化の範囲を調べられる。ほかの係数との組み合わせはB-96で未着手のまま保持する。\n"
                "- 保存資料: results/tuning/v075/のbest.json、report.md、trials.csv、cases.jsonl、"
                "single_parameter_curves.csv、mechanism.jsonl、plan.json、input_manifest.json、frozen.json。"
                "最良設定の単一C++はbest/v075_size_exponent.cpp。\n"
                "- 再開条件: ユーザーから追加検証の指示がある場合。今回の評価セットは再利用しない。\n")
        if "## 実験後" in NOTE.read_text():
            raise RuntimeError("実験後の記録が既に存在する")
        with NOTE.open("a") as stream:
            stream.write(text)
        backlog = BACKLOG.read_text()
        matches = [line for line in backlog.splitlines(True) if line.startswith("- **[B-95]")]
        if len(matches) != 1:
            raise RuntimeError("台帳B-95が一意ではない")
        replacement = ("- **[B-95] 候補サイズの指数を0.8から1.6まで広げて調整する** → "
                       f"[v075](experiments/v075.md) {verdict}。新規100件、2並列、6条件、指数0.95の基準2周を除外。"
                       f"基準比{result['delta_sum']:+d}手。未使用入力での効果は未検証。"
                       "再開条件: ユーザーの追加検証指示がある場合。今回の入力は再利用しない。"
                       "他の係数との組み合わせはB-96に保持する。資料はresults/tuning/v075/。\n")
        BACKLOG.write_text(backlog.replace(matches[0], "", 1).replace("## 決着済み\n", "## 決着済み\n\n" + replacement, 1))


def worker():
    run, keep_awake = TuningRun(), None
    try:
        with (RUN / ".worker_claim").open("x") as stream:
            stream.write(str(os.getpid()) + "\n")
    except FileExistsError:
        print("起動済みの実験を再実行することはできない", file=sys.stderr)
        return 1
    def interrupted(signum, frame):
        raise KeyboardInterrupt(f"signal {signum}")
    signal.signal(signal.SIGTERM, interrupted)
    try:
        configure_environment()
        check_preflight()
        args = argparse.Namespace(bin_name=BIN + "_tuning", jobs=JOBS, wait_lock=False)
        with evaluator.acquire_eval_lock(args, str((RUN / "input").relative_to(ROOT))):
            run.status("running", "preparing")
            if sys.platform == "darwin":
                keep_awake = subprocess.Popen(["/usr/bin/caffeinate", "-i", "-w", str(os.getpid())],
                                              stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL)
            run.prepare()
            for index, (phase, config_id) in enumerate(evaluation_plan()):
                run.evaluate(phase, config_id, index + 1 if phase == "warmup" else index - 1)
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
    check_preflight()
    RUN.mkdir(parents=True, exist_ok=False)
    with (RUN / "run.log").open("x") as stream:
        child = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "worker"], cwd=ROOT,
                                 stdin=subprocess.DEVNULL, stdout=stream, stderr=subprocess.STDOUT,
                                 start_new_session=True)
    write_json(RUN / "launch.json", dict(pid=child.pid, started_at=now(), command=sys.argv))
    # 起動応答だけを確認する。スコアや進捗の継続監視はしない。
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        if (RUN / "status.json").exists():
            status = json.loads((RUN / "status.json").read_text())
            if status["status"] in ("failed", "interrupted"):
                raise RuntimeError(status["error"])
            print(json.dumps(dict(started=True, pid=child.pid, run=str(RUN), stage=status["stage"],
                             measured_trials=len(CONFIGS), repair_weight=REPAIR_WEIGHT), ensure_ascii=False))
            return 0
        if child.poll() is not None:
            raise RuntimeError("起動失敗。run.logを確認すること")
        time.sleep(0.05)
    raise RuntimeError(f"起動応答を確認できない。PID={child.pid}。二重起動はしないこと")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("check", "launch", "worker"))
    action = parser.parse_args().action
    if action == "check":
        preflight_check()
    else:
        raise SystemExit(launch() if action == "launch" else worker())
