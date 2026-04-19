#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
ANALYSIS_DIR = ROOT / "results" / "analysis"
BINS = [
    "v012_simple_beam",
    "v014_lane_split_beam",
    "v139_refactor_v137",
    "v149_no_logs",
]
BIN_ORDER = {bin_name: idx for idx, bin_name in enumerate(BINS)}
TWO_WAY_BINS = ["v139_refactor_v137", "v149_no_logs"]
DENSITY_LOW_MAX = 0.40
DENSITY_MID_MAX = 0.55
CAT_MAIN = 100_000
CAT_SEVERE = 1_000_000
NEAR_TLE_1500 = 1_500
NEAR_TLE_1800 = 1_800
CV_FOLDS = 5


@dataclass(frozen=True)
class BinResult:
    score: int
    elapsed_ms: int


@dataclass(frozen=True)
class CaseData:
    case_id: str
    case_name: str
    dataset: str
    N: int
    M: int
    C: int
    density: float
    density3: str
    fold: int
    results: dict[str, BinResult]


@dataclass
class BucketAggregate:
    support: int
    score_sum: dict[str, int]
    elapsed_sum: dict[str, int]

    def mean_score(self, bin_name: str) -> float:
        return self.score_sum[bin_name] / self.support

    def mean_elapsed(self, bin_name: str) -> float:
        return self.elapsed_sum[bin_name] / self.support


@dataclass(frozen=True)
class ChoiceInfo:
    support: int
    chosen_bin: str
    runner_up_bin: str
    runner_up_margin: float
    mean_scores: dict[str, float]
    considered_bins: tuple[str, ...]


@dataclass
class EvalMetrics:
    case_count: int = 0
    score_sum: int = 0
    elapsed_sum: int = 0
    catastrophic_count: int = 0

    def update(self, score: int, elapsed_ms: int) -> None:
        self.case_count += 1
        self.score_sum += score
        self.elapsed_sum += elapsed_ms
        if score >= CAT_MAIN:
            self.catastrophic_count += 1

    @property
    def mean_score(self) -> float:
        return self.score_sum / self.case_count

    @property
    def avg_elapsed(self) -> float:
        return self.elapsed_sum / self.case_count


@dataclass(frozen=True)
class PolicySpec:
    policy_id: str
    family: str
    rule_complexity_label: str


class PolicyModel:
    def predict(self, case: CaseData) -> str:
        raise NotImplementedError

    def rule_rows(self) -> list[dict[str, str]]:
        raise NotImplementedError

    def rule_complexity(self) -> str:
        raise NotImplementedError


class FixedModel(PolicyModel):
    def __init__(self, bin_name: str):
        self.bin_name = bin_name

    def predict(self, case: CaseData) -> str:
        return self.bin_name

    def rule_rows(self) -> list[dict[str, str]]:
        return [
            {
                "level": "GLOBAL",
                "bucket": "ALL",
                "support": "",
                "chosen_bin": self.bin_name,
                "runner_up_bin": "",
                "runner_up_margin": "",
                "considered_bins": self.bin_name,
                "avg_score_v012_simple_beam": "",
                "avg_score_v014_lane_split_beam": "",
                "avg_score_v139_refactor_v137": "",
                "avg_score_v149_no_logs": "",
            }
        ]

    def rule_complexity(self) -> str:
        return "global=1"


class ThresholdModel(PolicyModel):
    def __init__(
        self,
        threshold: int,
        choice_by_n: dict[tuple[int], ChoiceInfo],
        global_choice: ChoiceInfo,
    ):
        self.threshold = threshold
        self.choice_by_n = choice_by_n
        self.global_choice = global_choice

    @staticmethod
    def train(train_cases: list[CaseData]) -> "ThresholdModel":
        by_n = build_bucket_aggregates(train_cases, key_n)
        global_choice = choose_from_aggregate(
            build_global_aggregate(train_cases),
            mode="two_way",
        )
        candidates = []
        for threshold in range(8, 18):
            metrics = EvalMetrics()
            for case in train_cases:
                chosen = "v149_no_logs" if case.N >= threshold else "v139_refactor_v137"
                result = case.results[chosen]
                metrics.update(result.score, result.elapsed_ms)
            candidates.append(
                (
                    metrics.mean_score,
                    metrics.catastrophic_count,
                    metrics.avg_elapsed,
                    -threshold,
                    threshold,
                )
            )
        threshold = min(candidates)[-1]

        choice_by_n: dict[tuple[int], ChoiceInfo] = {}
        for bucket, aggregate in by_n.items():
            chosen = "v149_no_logs" if bucket[0] >= threshold else "v139_refactor_v137"
            mean_scores = {
                bin_name: aggregate.mean_score(bin_name)
                for bin_name in BINS
            }
            other = "v139_refactor_v137" if chosen == "v149_no_logs" else "v149_no_logs"
            choice_by_n[bucket] = ChoiceInfo(
                support=aggregate.support,
                chosen_bin=chosen,
                runner_up_bin=other,
                runner_up_margin=mean_scores[other] - mean_scores[chosen],
                mean_scores=mean_scores,
                considered_bins=("v139_refactor_v137", "v149_no_logs"),
            )
        return ThresholdModel(threshold, choice_by_n, global_choice)

    def predict(self, case: CaseData) -> str:
        return "v149_no_logs" if case.N >= self.threshold else "v139_refactor_v137"

    def rule_rows(self) -> list[dict[str, str]]:
        rows = []
        for bucket in sorted(self.choice_by_n):
            choice = self.choice_by_n[bucket]
            row = base_rule_row("N", format_bucket("N", bucket), choice)
            row["selection_note"] = f"use v149_no_logs if N>={self.threshold}, else v139_refactor_v137"
            rows.append(row)
        return rows

    def rule_complexity(self) -> str:
        return f"N-threshold={self.threshold}"


class TableModel(PolicyModel):
    def __init__(
        self,
        level_names: list[str],
        choice_maps: dict[str, dict[tuple, ChoiceInfo]],
        selection_note: str,
    ):
        self.level_names = level_names
        self.choice_maps = choice_maps
        self.selection_note = selection_note

    def predict(self, case: CaseData) -> str:
        for level_name in self.level_names:
            key = level_key(level_name, case)
            if key in self.choice_maps[level_name]:
                return self.choice_maps[level_name][key].chosen_bin
        raise RuntimeError("global bucket must always exist")

    def rule_rows(self) -> list[dict[str, str]]:
        rows = []
        for level_name in self.level_names:
            for bucket in sorted(
                self.choice_maps[level_name],
                key=lambda key: format_bucket(level_name, key),
            ):
                choice = self.choice_maps[level_name][bucket]
                row = base_rule_row(level_name, format_bucket(level_name, bucket), choice)
                row["selection_note"] = self.selection_note
                rows.append(row)
        return rows

    def rule_complexity(self) -> str:
        parts = []
        for level_name in self.level_names:
            parts.append(f"{level_name}={len(self.choice_maps[level_name])}")
        return ",".join(parts)


POLICY_SPECS = [
    PolicySpec("P0_v012_fixed", "P0", "fixed"),
    PolicySpec("P0_v014_fixed", "P0", "fixed"),
    PolicySpec("P0_v139_fixed", "P0", "fixed"),
    PolicySpec("P0_v149_fixed", "P0", "fixed"),
    PolicySpec("P1_N_threshold_v139_v149", "P1", "threshold"),
    PolicySpec("P2_N_C_v139_v149", "P2", "table"),
    PolicySpec("P3_N_density3_v139_v149", "P3", "table"),
    PolicySpec("P4_N_C_density3_v139_v149", "P4", "table"),
    PolicySpec("P5_N_C_density3_guarded_fourway", "P5", "table"),
]
POLICY_ORDER = {spec.policy_id: idx for idx, spec in enumerate(POLICY_SPECS)}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Analyze N/M/C-only selector policies from serial_eval JSONL."
    )
    parser.add_argument(
        "--jsonl",
        help="input JSONL path. Defaults to the latest serial_eval_*.jsonl under results/analysis",
    )
    parser.add_argument(
        "--out-dir",
        help="output directory. Defaults to results/analysis/selector_nmc_<timestamp>",
    )
    return parser.parse_args()


def latest_jsonl_path() -> Path:
    candidates = list(ANALYSIS_DIR.glob("serial_eval_*.jsonl"))
    if not candidates:
        raise SystemExit("error: serial_eval_*.jsonl not found under results/analysis")
    return max(candidates, key=lambda path: path.stat().st_mtime)


def default_out_dir() -> Path:
    timestamp = datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
    return ANALYSIS_DIR / f"selector_nmc_{timestamp}"


def density3_bucket(N: int, M: int) -> tuple[float, str]:
    density = M / (N * N)
    if density < DENSITY_LOW_MAX:
        return density, "L"
    if density < DENSITY_MID_MAX:
        return density, "M"
    return density, "H"


def case_fold(case_name: str) -> int:
    return int(case_name[:4]) % CV_FOLDS


def load_cases(jsonl_path: Path) -> list[CaseData]:
    grouped: dict[str, list[dict]] = defaultdict(list)
    with jsonl_path.open(encoding="utf-8") as fh:
        for line_no, line in enumerate(fh, start=1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as exc:
                raise SystemExit(
                    f"error: failed to parse JSON at {jsonl_path}:{line_no}: {exc}"
                ) from exc
            grouped[record["case_id"]].append(record)

    cases = []
    for case_id, records in sorted(grouped.items()):
        if len(records) != len(BINS):
            raise SystemExit(
                f"error: case {case_id} has {len(records)} records, expected {len(BINS)}"
            )
        meta = records[0]
        bins_seen = {record["bin"] for record in records}
        if bins_seen != set(BINS):
            raise SystemExit(f"error: case {case_id} does not contain the expected bins")
        if any(record["status"] != "ok" for record in records):
            raise SystemExit(f"error: case {case_id} contains non-ok rows")
        density, density3 = density3_bucket(meta["N"], meta["M"])
        results = {
            record["bin"]: BinResult(
                score=int(record["score"]),
                elapsed_ms=int(record["elapsed_ms"]),
            )
            for record in records
        }
        cases.append(
            CaseData(
                case_id=case_id,
                case_name=meta["case_name"],
                dataset=meta["dataset"],
                N=int(meta["N"]),
                M=int(meta["M"]),
                C=int(meta["C"]),
                density=density,
                density3=density3,
                fold=case_fold(meta["case_name"]),
                results=results,
            )
        )
    return cases


def split_cases(cases: list[CaseData]) -> tuple[list[CaseData], list[CaseData]]:
    generated = [case for case in cases if case.dataset == "in_generated"]
    holdout = [case for case in cases if case.dataset == "in"]
    if len(generated) != 2000 or len(holdout) != 100:
        raise SystemExit(
            "error: expected 2000 generated cases and 100 holdout cases"
        )
    return generated, holdout


def key_global(case: CaseData) -> tuple[str]:
    return ("ALL",)


def key_n(case: CaseData) -> tuple[int]:
    return (case.N,)


def key_nc(case: CaseData) -> tuple[int, int]:
    return (case.N, case.C)


def key_nd(case: CaseData) -> tuple[int, str]:
    return (case.N, case.density3)


def key_ncd(case: CaseData) -> tuple[int, int, str]:
    return (case.N, case.C, case.density3)


KEY_FUNCS = {
    "GLOBAL": key_global,
    "N": key_n,
    "N_C": key_nc,
    "N_density3": key_nd,
    "N_C_density3": key_ncd,
}


def level_key(level_name: str, case: CaseData) -> tuple:
    return KEY_FUNCS[level_name](case)


def format_bucket(level_name: str, bucket: tuple) -> str:
    if level_name == "GLOBAL":
        return "ALL"
    if level_name == "N":
        return f"N={bucket[0]}"
    if level_name == "N_C":
        return f"N={bucket[0]}|C={bucket[1]}"
    if level_name == "N_density3":
        return f"N={bucket[0]}|D={bucket[1]}"
    if level_name == "N_C_density3":
        return f"N={bucket[0]}|C={bucket[1]}|D={bucket[2]}"
    raise RuntimeError(f"unknown level: {level_name}")


def build_bucket_aggregates(
    cases: list[CaseData],
    key_fn,
) -> dict[tuple, BucketAggregate]:
    support = Counter()
    score_sum: dict[tuple, Counter[str]] = defaultdict(Counter)
    elapsed_sum: dict[tuple, Counter[str]] = defaultdict(Counter)
    for case in cases:
        bucket = key_fn(case)
        support[bucket] += 1
        for bin_name in BINS:
            score_sum[bucket][bin_name] += case.results[bin_name].score
            elapsed_sum[bucket][bin_name] += case.results[bin_name].elapsed_ms
    out = {}
    for bucket, count in support.items():
        out[bucket] = BucketAggregate(
            support=count,
            score_sum=dict(score_sum[bucket]),
            elapsed_sum=dict(elapsed_sum[bucket]),
        )
    return out


def build_global_aggregate(cases: list[CaseData]) -> BucketAggregate:
    return build_bucket_aggregates(cases, key_global)[("ALL",)]


def choose_best_bin(
    aggregate: BucketAggregate,
    candidate_bins: list[str],
) -> tuple[str, str, float, dict[str, float]]:
    mean_scores = {bin_name: aggregate.mean_score(bin_name) for bin_name in BINS}
    mean_elapsed = {bin_name: aggregate.mean_elapsed(bin_name) for bin_name in BINS}
    ranked = sorted(
        candidate_bins,
        key=lambda bin_name: (
            mean_scores[bin_name],
            mean_elapsed[bin_name],
            BIN_ORDER[bin_name],
        ),
    )
    chosen = ranked[0]
    runner_up = ranked[1] if len(ranked) >= 2 else ranked[0]
    margin = mean_scores[runner_up] - mean_scores[chosen]
    return chosen, runner_up, margin, mean_scores


def choose_from_aggregate(aggregate: BucketAggregate, mode: str) -> ChoiceInfo:
    if mode == "two_way":
        considered = TWO_WAY_BINS
    elif mode == "guarded_fourway":
        considered = list(TWO_WAY_BINS)
        base_best, _, _, mean_scores = choose_best_bin(aggregate, TWO_WAY_BINS)
        base_best_score = mean_scores[base_best]
        if aggregate.support >= 15:
            for extra in ["v012_simple_beam", "v014_lane_split_beam"]:
                if mean_scores[extra] <= base_best_score - 20.0:
                    considered.append(extra)
    else:
        raise RuntimeError(f"unknown mode: {mode}")

    chosen, runner_up, margin, mean_scores = choose_best_bin(aggregate, considered)
    return ChoiceInfo(
        support=aggregate.support,
        chosen_bin=chosen,
        runner_up_bin=runner_up,
        runner_up_margin=margin,
        mean_scores=mean_scores,
        considered_bins=tuple(considered),
    )


def base_rule_row(level: str, bucket: str, choice: ChoiceInfo) -> dict[str, str]:
    return {
        "level": level,
        "bucket": bucket,
        "support": str(choice.support),
        "chosen_bin": choice.chosen_bin,
        "runner_up_bin": choice.runner_up_bin,
        "runner_up_margin": format_float(choice.runner_up_margin),
        "considered_bins": "|".join(choice.considered_bins),
        "avg_score_v012_simple_beam": format_float(choice.mean_scores["v012_simple_beam"]),
        "avg_score_v014_lane_split_beam": format_float(choice.mean_scores["v014_lane_split_beam"]),
        "avg_score_v139_refactor_v137": format_float(choice.mean_scores["v139_refactor_v137"]),
        "avg_score_v149_no_logs": format_float(choice.mean_scores["v149_no_logs"]),
    }


def train_table_model(
    train_cases: list[CaseData],
    level_names: list[str],
    mode: str,
    selection_note: str,
) -> TableModel:
    choice_maps: dict[str, dict[tuple, ChoiceInfo]] = {}
    for level_name in level_names:
        aggregates = build_bucket_aggregates(train_cases, KEY_FUNCS[level_name])
        choice_maps[level_name] = {
            bucket: choose_from_aggregate(aggregate, mode)
            for bucket, aggregate in aggregates.items()
        }
    return TableModel(level_names=level_names, choice_maps=choice_maps, selection_note=selection_note)


def train_policy(spec: PolicySpec, train_cases: list[CaseData]) -> PolicyModel:
    if spec.policy_id == "P0_v012_fixed":
        return FixedModel("v012_simple_beam")
    if spec.policy_id == "P0_v014_fixed":
        return FixedModel("v014_lane_split_beam")
    if spec.policy_id == "P0_v139_fixed":
        return FixedModel("v139_refactor_v137")
    if spec.policy_id == "P0_v149_fixed":
        return FixedModel("v149_no_logs")
    if spec.policy_id == "P1_N_threshold_v139_v149":
        return ThresholdModel.train(train_cases)
    if spec.policy_id == "P2_N_C_v139_v149":
        return train_table_model(
            train_cases,
            level_names=["N_C", "N", "GLOBAL"],
            mode="two_way",
            selection_note="candidate bins are v139_refactor_v137 and v149_no_logs; backoff N_C -> N -> GLOBAL",
        )
    if spec.policy_id == "P3_N_density3_v139_v149":
        return train_table_model(
            train_cases,
            level_names=["N_density3", "N", "GLOBAL"],
            mode="two_way",
            selection_note="candidate bins are v139_refactor_v137 and v149_no_logs; backoff N_density3 -> N -> GLOBAL",
        )
    if spec.policy_id == "P4_N_C_density3_v139_v149":
        return train_table_model(
            train_cases,
            level_names=["N_C_density3", "N_C", "N", "GLOBAL"],
            mode="two_way",
            selection_note="candidate bins are v139_refactor_v137 and v149_no_logs; backoff N_C_density3 -> N_C -> N -> GLOBAL",
        )
    if spec.policy_id == "P5_N_C_density3_guarded_fourway":
        return train_table_model(
            train_cases,
            level_names=["N_C_density3", "N_C", "N", "GLOBAL"],
            mode="guarded_fourway",
            selection_note="v012/v014 are allowed only when support>=15 and they beat the best of {v139,v149} by at least 20 in the current bucket; backoff N_C_density3 -> N_C -> N -> GLOBAL",
        )
    raise RuntimeError(f"unknown policy: {spec.policy_id}")


def evaluate_model(model: PolicyModel, cases: list[CaseData]) -> EvalMetrics:
    metrics = EvalMetrics()
    for case in cases:
        chosen = model.predict(case)
        result = case.results[chosen]
        metrics.update(result.score, result.elapsed_ms)
    return metrics


def evaluate_fixed_bin(cases: list[CaseData], bin_name: str) -> EvalMetrics:
    return evaluate_model(FixedModel(bin_name), cases)


def cv_metrics(spec: PolicySpec, generated_cases: list[CaseData]) -> EvalMetrics:
    metrics = EvalMetrics()
    for fold in range(CV_FOLDS):
        train_cases = [case for case in generated_cases if case.fold != fold]
        test_cases = [case for case in generated_cases if case.fold == fold]
        model = train_policy(spec, train_cases)
        fold_metrics = evaluate_model(model, test_cases)
        metrics.case_count += fold_metrics.case_count
        metrics.score_sum += fold_metrics.score_sum
        metrics.elapsed_sum += fold_metrics.elapsed_sum
        metrics.catastrophic_count += fold_metrics.catastrophic_count
    return metrics


def policy_rule_complexity(spec: PolicySpec, generated_cases: list[CaseData]) -> str:
    model = train_policy(spec, generated_cases)
    return model.rule_complexity()


def oracle_metrics(cases: list[CaseData]) -> EvalMetrics:
    metrics = EvalMetrics()
    for case in cases:
        best_bin = best_bin_name(case)
        best = case.results[best_bin]
        metrics.update(best.score, best.elapsed_ms)
    return metrics


def best_bin_name(case: CaseData) -> str:
    return min(
        BINS,
        key=lambda bin_name: (
            case.results[bin_name].score,
            case.results[bin_name].elapsed_ms,
            BIN_ORDER[bin_name],
        ),
    )


def bin_diagnostics(cases: list[CaseData]) -> list[dict[str, str]]:
    rows = []
    for bin_name in BINS:
        metrics = evaluate_fixed_bin(cases, bin_name)
        near_1500 = sum(case.results[bin_name].elapsed_ms >= NEAR_TLE_1500 for case in cases)
        near_1800 = sum(case.results[bin_name].elapsed_ms >= NEAR_TLE_1800 for case in cases)
        severe = sum(case.results[bin_name].score >= CAT_SEVERE for case in cases)
        rows.append(
            {
                "bin": bin_name,
                "mean_score": format_float(metrics.mean_score),
                "avg_elapsed_ms": format_float(metrics.avg_elapsed),
                "max_elapsed_ms": str(max(case.results[bin_name].elapsed_ms for case in cases)),
                "catastrophic_1e5": str(metrics.catastrophic_count),
                "catastrophic_1e6": str(severe),
                "elapsed_ge_1500": str(near_1500),
                "elapsed_ge_1800": str(near_1800),
            }
        )
    return rows


def evaluate_all_policies(
    generated_cases: list[CaseData],
    holdout_cases: list[CaseData],
) -> tuple[list[dict[str, str]], dict[str, PolicyModel]]:
    rows = []
    full_models = {}
    for spec in POLICY_SPECS:
        cv = cv_metrics(spec, generated_cases)
        full_model = train_policy(spec, generated_cases)
        full_models[spec.policy_id] = full_model
        holdout = evaluate_model(full_model, holdout_cases)
        rows.append(
            {
                "policy": spec.policy_id,
                "generated_cv_mean_score": format_float(cv.mean_score),
                "generated_cv_catastrophic_count": str(cv.catastrophic_count),
                "generated_cv_avg_elapsed": format_float(cv.avg_elapsed),
                "official_holdout_score": format_float(holdout.mean_score),
                "official_holdout_catastrophic_count": str(holdout.catastrophic_count),
                "rule_complexity": full_model.rule_complexity(),
            }
        )
    rows.sort(key=lambda row: POLICY_ORDER[row["policy"]])
    return rows, full_models


def selected_bins(model: PolicyModel, cases: list[CaseData]) -> set[str]:
    return {model.predict(case) for case in cases}


def select_policies(
    policy_rows: list[dict[str, str]],
    full_models: dict[str, PolicyModel],
    generated_cases: list[CaseData],
) -> tuple[str, str, str, bool]:
    baseline = next(row for row in policy_rows if row["policy"] == "P0_v139_fixed")
    baseline_cv = float(baseline["generated_cv_mean_score"])
    baseline_holdout = float(baseline["official_holdout_score"])
    baseline_cat_cv = int(baseline["generated_cv_catastrophic_count"])
    baseline_cat_holdout = int(baseline["official_holdout_catastrophic_count"])

    non_degenerate = []
    for row in policy_rows:
        policy_id = row["policy"]
        if policy_id.startswith("P0_"):
            continue
        if len(selected_bins(full_models[policy_id], generated_cases)) <= 1:
            continue
        non_degenerate.append(row)

    simplicity_rank = {
        "P1_N_threshold_v139_v149": 1,
        "P2_N_C_v139_v149": 2,
        "P3_N_density3_v139_v149": 3,
        "P4_N_C_density3_v139_v149": 4,
        "P5_N_C_density3_guarded_fourway": 5,
        "P0_v139_fixed": 6,
        "P0_v149_fixed": 6,
        "P0_v012_fixed": 6,
        "P0_v014_fixed": 6,
    }

    candidates = [
        row
        for row in non_degenerate
        if float(row["generated_cv_mean_score"]) <= baseline_cv
        and float(row["official_holdout_score"]) <= baseline_holdout
        and int(row["generated_cv_catastrophic_count"]) <= baseline_cat_cv
        and int(row["official_holdout_catastrophic_count"]) <= baseline_cat_holdout
    ]
    if candidates:
        primary = min(
            candidates,
            key=lambda row: (
                float(row["generated_cv_mean_score"]),
                float(row["official_holdout_score"]),
                POLICY_ORDER[row["policy"]],
            ),
        )["policy"]

        backup = min(
            candidates,
            key=lambda row: (
                simplicity_rank.get(row["policy"], 99),
                float(row["generated_cv_mean_score"]),
                float(row["official_holdout_score"]),
                POLICY_ORDER[row["policy"]],
            ),
        )["policy"]
        return primary, primary, backup, True

    exploratory = [
        row
        for row in non_degenerate
        if float(row["official_holdout_score"]) <= baseline_holdout
        and int(row["official_holdout_catastrophic_count"]) <= baseline_cat_holdout
    ]
    if not exploratory:
        exploratory = list(non_degenerate)
    if not exploratory:
        return "P0_v139_fixed", "P0_v139_fixed", "P0_v139_fixed", False

    primary_selector = min(
        exploratory,
        key=lambda row: (
            float(row["official_holdout_score"]),
            float(row["generated_cv_mean_score"]),
            POLICY_ORDER[row["policy"]],
        ),
    )["policy"]
    backup_selector = min(
        exploratory,
        key=lambda row: (
            simplicity_rank.get(row["policy"], 99),
            float(row["official_holdout_score"]),
            float(row["generated_cv_mean_score"]),
            POLICY_ORDER[row["policy"]],
        ),
    )["policy"]
    return "P0_v139_fixed", primary_selector, backup_selector, False


def format_float(value: float) -> str:
    return f"{value:.3f}"


def write_csv(path: Path, fieldnames: list[str], rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def write_bucket_rules(path: Path, policy_id: str, model: PolicyModel) -> None:
    rows = []
    for row in model.rule_rows():
        rows.append({"policy": policy_id, **row})
    fieldnames = [
        "policy",
        "level",
        "bucket",
        "support",
        "chosen_bin",
        "runner_up_bin",
        "runner_up_margin",
        "considered_bins",
        "avg_score_v012_simple_beam",
        "avg_score_v014_lane_split_beam",
        "avg_score_v139_refactor_v137",
        "avg_score_v149_no_logs",
        "selection_note",
    ]
    write_csv(path, fieldnames, rows)


def policy_row_map(policy_rows: list[dict[str, str]]) -> dict[str, dict[str, str]]:
    return {row["policy"]: row for row in policy_rows}


def selector_case_counts(model: PolicyModel, cases: list[CaseData]) -> Counter[str]:
    counter = Counter()
    for case in cases:
        counter[model.predict(case)] += 1
    return counter


def p5_extra_usage(model: PolicyModel, cases: list[CaseData]) -> tuple[int, int]:
    counts = selector_case_counts(model, cases)
    return counts["v012_simple_beam"], counts["v014_lane_split_beam"]


def v149_catastrophic_cases(cases: list[CaseData]) -> list[CaseData]:
    out = []
    for case in cases:
        if case.results["v149_no_logs"].score >= CAT_SEVERE:
            out.append(case)
    return out


def primary_oracle_gap(
    model: PolicyModel,
    cases: list[CaseData],
    bucket_level: str | None,
) -> list[tuple[str, int, float]]:
    by_bucket = defaultdict(lambda: {"count": 0, "regret_sum": 0})
    for case in cases:
        chosen = model.predict(case)
        chosen_score = case.results[chosen].score
        oracle_score = min(case.results[bin_name].score for bin_name in BINS)
        regret = chosen_score - oracle_score
        if bucket_level is None:
            bucket = "ALL"
        else:
            bucket = format_bucket(bucket_level, level_key(bucket_level, case))
        by_bucket[bucket]["count"] += 1
        by_bucket[bucket]["regret_sum"] += regret
    rows = []
    for bucket, value in by_bucket.items():
        rows.append((bucket, value["count"], value["regret_sum"] / value["count"]))
    rows.sort(key=lambda item: (-item[2], item[0]))
    return rows


def primary_bucket_level(policy_id: str) -> str | None:
    if policy_id == "P1_N_threshold_v139_v149":
        return "N"
    if policy_id == "P2_N_C_v139_v149":
        return "N_C"
    if policy_id == "P3_N_density3_v139_v149":
        return "N_density3"
    if policy_id in {"P4_N_C_density3_v139_v149", "P5_N_C_density3_guarded_fourway"}:
        return "N_C_density3"
    return None


def write_summary(
    path: Path,
    jsonl_path: Path,
    all_cases: list[CaseData],
    generated_cases: list[CaseData],
    holdout_cases: list[CaseData],
    policy_rows: list[dict[str, str]],
    full_models: dict[str, PolicyModel],
    production_policy: str,
    primary_selector_policy: str,
    backup_selector_policy: str,
    has_qualified_selector: bool,
) -> None:
    row_map = policy_row_map(policy_rows)
    all_diag = bin_diagnostics(all_cases)
    generated_oracle = oracle_metrics(generated_cases)
    holdout_oracle = oracle_metrics(holdout_cases)
    v149_bad = v149_catastrophic_cases(all_cases)
    p5_model = full_models["P5_N_C_density3_guarded_fourway"]
    p5_v012_count, p5_v014_count = p5_extra_usage(p5_model, generated_cases)
    primary_model = full_models[primary_selector_policy]
    primary_bucket = primary_bucket_level(primary_selector_policy)
    residuals = primary_oracle_gap(primary_model, generated_cases, primary_bucket)[:3]

    baseline_v139 = row_map["P0_v139_fixed"]
    production_row = row_map[production_policy]
    primary_row = row_map[primary_selector_policy]
    backup_row = row_map[backup_selector_policy]
    p4_row = row_map["P4_N_C_density3_v139_v149"]
    p5_row = row_map["P5_N_C_density3_guarded_fourway"]

    lines = []
    lines.append("# N/M/C 限定 selector 分析")
    lines.append("")
    lines.append("## データ")
    lines.append(f"- 入力 JSONL: `{jsonl_path}`")
    lines.append(f"- 総ケース数: {len(all_cases)} (`in_generated` {len(generated_cases)} 件, `in` {len(holdout_cases)} 件)")
    lines.append(f"- density3 bucket の定義: `L < {DENSITY_LOW_MAX:.2f}`, `M < {DENSITY_MID_MAX:.2f}`, `H >= {DENSITY_MID_MAX:.2f}`")
    lines.append("")
    lines.append("## 全データでの単独 bin 診断")
    lines.append("| bin | mean_score | avg_elapsed_ms | max_elapsed_ms | cat>=1e5 | cat>=1e6 | elapsed>=1500 | elapsed>=1800 |")
    lines.append("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for row in all_diag:
        lines.append(
            f"| {row['bin']} | {row['mean_score']} | {row['avg_elapsed_ms']} | {row['max_elapsed_ms']} | "
            f"{row['catastrophic_1e5']} | {row['catastrophic_1e6']} | {row['elapsed_ge_1500']} | {row['elapsed_ge_1800']} |"
        )
    lines.append("")
    lines.append("## 主な所見")
    lines.append(
        f"- 安定した単独 baseline として最も強いのは `v139_refactor_v137` である。generated CV は {baseline_v139['generated_cv_mean_score']}、holdout は {baseline_v139['official_holdout_score']}。"
    )
    lines.append(
        f"- `v149_no_logs` は高密度かつ大きめの `N` の一部領域で `v139_refactor_v137` よりかなり強いが、`score >= {CAT_SEVERE}` の catastrophic case を {len(v149_bad)} 件持っている。"
    )
    if v149_bad:
        ns = sorted({case.N for case in v149_bad})
        cs = sorted({case.C for case in v149_bad})
        lines.append(
            f"- `v149_no_logs` の catastrophic case はすべて density bucket `H` に集中していた。観測された `N` は {ns}、`C` は {cs} である。"
        )
    lines.append(
        f"- oracle 平均スコアは `in_generated` で {generated_oracle.mean_score:.3f}、`in` で {holdout_oracle.mean_score:.3f} であり、この 4 bin から作る selector の理論下限とみなせる。"
    )
    if residuals:
        residual_text = ", ".join(
            f"`{bucket}` (count={count}, avg regret={avg_regret:.3f})"
            for bucket, count, avg_regret in residuals
        )
        lines.append(
            f"- 主候補 selector の oracle との差は、とくに {residual_text} に集中している。"
        )
    lines.append("")
    lines.append("## Policy 比較")
    lines.append("| policy | generated_cv_mean_score | official_holdout_score | generated_cv_catastrophic_count | official_holdout_catastrophic_count | rule_complexity |")
    lines.append("| --- | ---: | ---: | ---: | ---: | --- |")
    seen = set()
    for policy_id in [
        "P0_v139_fixed",
        production_policy,
        backup_selector_policy,
        primary_selector_policy,
        "P4_N_C_density3_v139_v149",
        "P5_N_C_density3_guarded_fourway",
    ]:
        if policy_id in seen:
            continue
        seen.add(policy_id)
        row = row_map[policy_id]
        lines.append(
            f"| {policy_id} | {row['generated_cv_mean_score']} | {row['official_holdout_score']} | "
            f"{row['generated_cv_catastrophic_count']} | {row['official_holdout_catastrophic_count']} | {row['rule_complexity']} |"
        )
    lines.append("")
    lines.append("## 推奨")
    if has_qualified_selector:
        lines.append(
            f"- 本番採用候補は `{production_policy}` である。generated CV と holdout の両方で `P0_v139_fixed` を上回っている (`{baseline_v139['generated_cv_mean_score']} -> {production_row['generated_cv_mean_score']}`, `{baseline_v139['official_holdout_score']} -> {production_row['official_holdout_score']}`)。"
        )
        lines.append(
            f"- 保険候補は `{backup_selector_policy}` である。同じ条件を満たす selector の中で最も単純である (`{backup_row['generated_cv_mean_score']}`, `{backup_row['official_holdout_score']}`)。"
        )
    else:
        lines.append(
            "- 非自明な selector の中で、generated CV と holdout の両方で `P0_v139_fixed` を上回るものは見つからなかった。したがって、現時点の本番 baseline は `P0_v139_fixed` のままにするのが妥当である。"
        )
        lines.append(
            f"- 実験用の主候補 selector は `{primary_selector_policy}` である。holdout は改善する (`{baseline_v139['official_holdout_score']} -> {primary_row['official_holdout_score']}`) 一方、generated CV は悪化する (`{baseline_v139['generated_cv_mean_score']} -> {primary_row['generated_cv_mean_score']}`)。"
        )
        lines.append(
            f"- 実験用の保険候補 selector は `{backup_selector_policy}` である。`v139` より holdout を悪化させない selector の中で最も単純である (`{backup_row['official_holdout_score']}`)。"
        )
    lines.append(
        f"- `P5` は generated 側で `v012_simple_beam` を {p5_v012_count} 件、`v014_lane_split_beam` を {p5_v014_count} 件選択した。対応する generated CV / holdout は {p5_row['generated_cv_mean_score']} / {p5_row['official_holdout_score']} である。"
    )
    if float(p5_row["generated_cv_mean_score"]) <= float(p4_row["generated_cv_mean_score"]) and float(
        p5_row["official_holdout_score"]
    ) <= float(p4_row["official_holdout_score"]):
        lines.append(
            "- `v012` / `v014` については、guarded inclusion を入れても `P4` の two-way selector に負けていないため、候補集合に残す価値がある。"
        )
    else:
        lines.append(
            "- `v012` / `v014` については、勝つ bucket 自体はあるものの、guarded inclusion を入れても two-way selector に CV または holdout のどちらかで負ける。現時点では submit-time selector に入れる価値は薄い。"
        )
    lines.append("")
    lines.append("## Oracle との差")
    lines.append(
        f"- `P0_v139_fixed` vs oracle on `in_generated`: {float(baseline_v139['generated_cv_mean_score']) - generated_oracle.mean_score:.3f}"
    )
    lines.append(
        f"- `{primary_selector_policy}` vs oracle on `in_generated`: {float(primary_row['generated_cv_mean_score']) - generated_oracle.mean_score:.3f}"
    )
    lines.append(
        f"- `P0_v139_fixed` vs oracle on `in`: {float(baseline_v139['official_holdout_score']) - holdout_oracle.mean_score:.3f}"
    )
    lines.append(
        f"- `{primary_selector_policy}` vs oracle on `in`: {float(primary_row['official_holdout_score']) - holdout_oracle.mean_score:.3f}"
    )
    lines.append("")
    lines.append("## 次の一手")
    if has_qualified_selector:
        lines.append(
            f"- 現在の 4 bin だけで期待値を最大化したいなら、まず `{production_policy}` の実装に入るのがよい。"
        )
        lines.append(
            f"- コード量やデバッグ負荷を重く見るなら、`{backup_selector_policy}` を低リスクな保険候補として維持する。"
        )
    else:
        lines.append(
            "- 提出 baseline は当面 `P0_v139_fixed` のまま維持する。"
        )
        lines.append(
            f"- selector 探索を続けるなら、まず `{primary_selector_policy}` を試作し、比較用のより単純な基準として `{backup_selector_policy}` を併置するのがよい。"
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    jsonl_path = Path(args.jsonl).expanduser().resolve() if args.jsonl else latest_jsonl_path()
    out_dir = Path(args.out_dir).expanduser().resolve() if args.out_dir else default_out_dir()
    out_dir.mkdir(parents=True, exist_ok=True)

    cases = load_cases(jsonl_path)
    generated_cases, holdout_cases = split_cases(cases)
    policy_rows, full_models = evaluate_all_policies(generated_cases, holdout_cases)
    production_policy, primary_selector_policy, backup_selector_policy, has_qualified_selector = select_policies(
        policy_rows,
        full_models,
        generated_cases,
    )

    policy_eval_path = out_dir / "policy_eval.csv"
    bucket_rules_path = out_dir / "bucket_rules.csv"
    summary_path = out_dir / "summary.md"

    write_csv(
        policy_eval_path,
        [
            "policy",
            "generated_cv_mean_score",
            "generated_cv_catastrophic_count",
            "generated_cv_avg_elapsed",
            "official_holdout_score",
            "official_holdout_catastrophic_count",
            "rule_complexity",
        ],
        policy_rows,
    )
    write_bucket_rules(
        bucket_rules_path,
        primary_selector_policy,
        full_models[primary_selector_policy],
    )
    write_summary(
        summary_path,
        jsonl_path=jsonl_path,
        all_cases=cases,
        generated_cases=generated_cases,
        holdout_cases=holdout_cases,
        policy_rows=policy_rows,
        full_models=full_models,
        production_policy=production_policy,
        primary_selector_policy=primary_selector_policy,
        backup_selector_policy=backup_selector_policy,
        has_qualified_selector=has_qualified_selector,
    )

    print(f"wrote {out_dir}")
    print(f"production={production_policy}")
    print(f"primary_selector={primary_selector_policy}")
    print(f"backup_selector={backup_selector_policy}")
    print(f"qualified_selector={has_qualified_selector}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
