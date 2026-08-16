#!/usr/bin/env python3
"""B-100: 確定占有を控除した残余容量価格を保存済みv081軌跡で検証する。

未来既知基準は、各退去区間で実際に未到着だったgroupの占有量を使い、確定占有を
capacityから先に控除して求める。予測future_areaだけを同じ残余価格式へ入れた値と、
B-087のknown+future合算価格を比較する。奇数smooth caseで、turn価格MAEとprice-only
受否不一致率がともに合算価格より小さく、case別MAEでも過半数勝てばpremise pass。
solverは起動せず、保存済み入出力だけを読む補助probeである。
"""

from __future__ import annotations

import argparse
import functools
import math
from dataclasses import dataclass
from pathlib import Path

from probe_b087_time_price import (
    EXPECTED_P,
    HORIZON,
    Group,
    boundary_load_factor,
    effective_capacity,
    event_points,
    parse_accepts,
    parse_input,
    posterior_theta,
    predicted_future,
)


@dataclass
class PriceStats:
    turns: int = 0
    abs_error: float = 0.0
    sq_error: float = 0.0
    threshold_sum: float = 0.0
    decision_mismatch: int = 0

    def add(self, predicted: float, oracle: float, quality: float) -> None:
        error = predicted - oracle
        self.turns += 1
        self.abs_error += abs(error)
        self.sq_error += error * error
        self.threshold_sum += predicted
        self.decision_mismatch += (quality >= predicted) != (quality >= oracle)

    def merge(self, other: "PriceStats") -> None:
        self.turns += other.turns
        self.abs_error += other.abs_error
        self.sq_error += other.sq_error
        self.threshold_sum += other.threshold_sum
        self.decision_mismatch += other.decision_mismatch

    @property
    def mae(self) -> float:
        return self.abs_error / self.turns

    @property
    def rmse(self) -> float:
        return math.sqrt(self.sq_error / self.turns)

    @property
    def mean_threshold(self) -> float:
        return self.threshold_sum / self.turns

    @property
    def mismatch_rate(self) -> float:
        return self.decision_mismatch / self.turns


def minimum_perimeter(P: int) -> int:
    return 2 * math.ceil(2.0 * math.sqrt(P) - 1e-12)


def compactness(P: int, perimeter: int) -> float:
    return 4.0 * math.sqrt(P) / perimeter


def compactness_bar() -> float:
    lo_x = 2.0
    hi_x = math.sqrt(150.0)
    width = hi_x - lo_x
    weighted = 0.0
    expected = 0.0
    for P in range(4, 151):
        lo = max(lo_x, math.sqrt(max(P - 0.5, 0.0)))
        hi = min(hi_x, math.sqrt(P + 0.5))
        probability = max(hi - lo, 0.0) / width
        expected += probability * P
        weighted += probability * P * compactness(P, minimum_perimeter(P))
    assert abs(expected - EXPECTED_P) < 1e-8
    return weighted / expected


def accepted_load_fraction(q_threshold: float) -> float:
    steps = 160
    end = 16.0
    width = end / steps
    sigma = 0.8 * math.log(2.0)
    total = 0.0
    for k in range(steps + 1):
        x = width * k
        if x > 0.0:
            threshold = q_threshold * x**0.1
            survival = 0.5 * math.erfc(
                math.log(threshold) / (sigma * math.sqrt(2.0))
            ) if threshold > 0.0 else 1.0
            value = x * math.exp(-x) * survival
        else:
            value = 0.0
        coefficient = 1 if k in (0, steps) else 4 if k % 2 == 1 else 2
        total += coefficient * value
    return total * width / 3.0


@functools.lru_cache(maxsize=1_000)
def q_threshold_for_key(key: int) -> float:
    if key >= 1_000:
        return 0.0
    target = key / 1_000.0
    low = 0.0
    high = 16.0
    while accepted_load_fraction(high) > target:
        high *= 2.0
    for _ in range(34):
        mid = 0.5 * (low + high)
        if accepted_load_fraction(mid) > target:
            low = mid
        else:
            high = mid
    return 0.5 * (low + high)


def q_threshold_for_fraction(fraction: float) -> float:
    if fraction >= 0.9995:
        return 0.0
    key = min(max(math.floor(fraction * 1_000.0 + 0.5), 0), 999)
    return q_threshold_for_key(key)


def q_threshold_for_positive_fraction(fraction: float) -> float:
    """正の積分点を既存0.001刻みcacheのkey 0へ丸めない。"""
    if fraction >= 0.9995:
        return 0.0
    key = min(max(math.floor(fraction * 1_000.0 + 0.5), 1), 999)
    return q_threshold_for_key(key)


def pooled_bid(known_area: float, future_area: float, capacity: float, c_bar: float) -> float:
    offered = known_area + future_area
    if offered <= capacity:
        return 0.0
    return q_threshold_for_fraction(capacity / offered) * c_bar


def residual_bid(
    known_area: float,
    future_area: float,
    capacity: float,
    c_bar: float,
    incoming_P: int,
) -> float:
    if future_area <= 1e-12:
        return 0.0
    residual_before = capacity - known_area
    accepted_before = min(max(residual_before, 0.0), future_area)
    accepted_after = min(max(residual_before - incoming_P, 0.0), future_area)
    displaced = accepted_before - accepted_after
    if displaced <= 1e-12:
        return 0.0

    # q(fraction) は残余容量一点の微分価格で、fraction=0では非有界になる。
    # current groupが実際に消費するPセル幅で未来価値をGauss積分し、有限差分の
    # 1セル当たり平均機会費用として返す。積分点は端点0を含まない。
    points = (0.112_701_665_379_3, 0.5, 0.887_298_334_620_7)
    weights = (5.0 / 18.0, 8.0 / 18.0, 5.0 / 18.0)
    average_marginal = 0.0
    for x, weight in zip(points, weights):
        accepted_area = accepted_after + x * displaced
        fraction = accepted_area / future_area
        average_marginal += (
            weight * q_threshold_for_positive_fraction(fraction) * c_bar
        )
    return (displaced / incoming_P) * average_marginal


def legacy_threshold(
    group: Group,
    known_now: int,
    theta: float,
    capacity: float,
    c_bar: float,
) -> float:
    offered_area = (1_000.0 / HORIZON) * EXPECTED_P * theta
    points = (0.112_701_665_379_3, 0.5, 0.887_298_334_620_7)
    weights = (5.0 / 18.0, 8.0 / 18.0, 5.0 / 18.0)
    duration = group.T - group.S
    average = 0.0
    for x, weight in zip(points, weights):
        time = group.S + x * duration
        offered = offered_area * boundary_load_factor(time, theta)
        bid = 0.0 if offered <= capacity else q_threshold_for_fraction(capacity / offered) * c_bar
        average += weight * bid
    threshold = average * (duration / theta) ** 0.1
    current_offer = offered_area * boundary_load_factor(group.S, theta)
    target = min(capacity, current_offer)
    error = (known_now + 0.5 * group.P - target) / max(capacity, 1.0)
    multiplier = min(max(math.exp(0.70 * error), 0.82), 1.80)
    return threshold * multiplier


def evaluate_case(
    groups: list[Group], accepted: list[bool], capacity: float, c_bar: float
) -> dict[str, PriceStats]:
    result = {name: PriceStats() for name in ("pooled", "residual", "legacy")}
    active_ids: list[int] = []
    duration_sum = 0.0
    for i, group in enumerate(groups):
        active_ids = [j for j in active_ids if groups[j].T > group.S]
        duration = group.T - group.S
        duration_sum += duration
        theta = posterior_theta(duration_sum, i + 1)
        predicted_average = {"pooled": 0.0, "residual": 0.0}
        oracle_average = 0.0
        for time, weight in event_points(group, active_ids, groups):
            known = sum(groups[j].P for j in active_ids if groups[j].T > time)
            actual_future = 0
            for k in range(i + 1, len(groups)):
                candidate = groups[k]
                if candidate.S > time:
                    break
                if candidate.T > time:
                    actual_future += candidate.P
            predicted = predicted_future(
                "event_fixed_right", time, group.S, theta, len(groups) - i - 1
            )
            predicted_average["pooled"] += weight * pooled_bid(
                known, predicted, capacity, c_bar
            )
            predicted_average["residual"] += weight * residual_bid(
                known, predicted, capacity, c_bar, group.P
            )
            oracle_average += weight * residual_bid(
                known, actual_future, capacity, c_bar, group.P
            )

        duration_factor = (duration / theta) ** 0.1
        oracle = oracle_average * duration_factor
        quality = (
            group.V / (group.P * duration**0.9)
        ) * compactness(group.P, minimum_perimeter(group.P))
        for name in ("pooled", "residual"):
            result[name].add(predicted_average[name] * duration_factor, oracle, quality)
        result["legacy"].add(
            legacy_threshold(group, sum(groups[j].P for j in active_ids), theta, capacity, c_bar),
            oracle,
            quality,
        )
        if accepted[i]:
            active_ids.append(i)
    return result


def aggregate(case_stats: dict[int, dict[str, PriceStats]]) -> dict[str, PriceStats]:
    totals = {name: PriceStats() for name in ("pooled", "residual", "legacy")}
    for stats in case_stats.values():
        for name, values in stats.items():
            totals[name].merge(values)
    return totals


def print_partition(name: str, case_stats: dict[int, dict[str, PriceStats]]) -> dict[str, PriceStats]:
    totals = aggregate(case_stats)
    print(f"[{name}] cases={len(case_stats)} turns={totals['residual'].turns}")
    for model, values in totals.items():
        print(
            f"  {model:8s} threshold_mean={values.mean_threshold:.6f} "
            f"mae={values.mae:.6f} rmse={values.rmse:.6f} "
            f"decision_mismatch={values.mismatch_rate:.6f}"
        )
    return totals


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", type=Path, default=Path("tools/in"))
    parser.add_argument(
        "--outputs", type=Path, default=Path("results/out/v081_deep_terminal_hybrid")
    )
    args = parser.parse_args()

    c_bar = compactness_bar()
    per_case: dict[int, dict[str, PriceStats]] = {}
    for input_path in sorted(args.inputs.glob("*.txt")):
        case_id = int(input_path.stem)
        error_path = args.outputs / f"{input_path.name}.err"
        if "route_smooth=1" not in error_path.read_text():
            continue
        board, groups = parse_input(input_path)
        accepted = parse_accepts(args.outputs / input_path.name, groups)
        per_case[case_id] = evaluate_case(
            groups, accepted, effective_capacity(board), c_bar
        )

    development = {case: stats for case, stats in per_case.items() if case % 2 == 0}
    validation = {case: stats for case, stats in per_case.items() if case % 2 == 1}
    print_partition("development-even", development)
    validation_totals = print_partition("validation-odd", validation)
    wins = sum(
        stats["residual"].mae < stats["pooled"].mae
        for stats in validation.values()
    )
    ties = sum(
        stats["residual"].mae == stats["pooled"].mae
        for stats in validation.values()
    )
    residual = validation_totals["residual"]
    pooled = validation_totals["pooled"]
    premise_pass = (
        residual.mae < pooled.mae
        and residual.mismatch_rate < pooled.mismatch_rate
        and wins > len(validation) / 2
    )
    print(
        f"validation_residual_vs_pooled_cases={wins}W/{ties}T/"
        f"{len(validation) - wins - ties}L premise={'PASS' if premise_pass else 'FAIL'}"
    )


if __name__ == "__main__":
    main()
