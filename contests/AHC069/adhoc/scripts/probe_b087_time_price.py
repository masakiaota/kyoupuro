#!/usr/bin/env python3
"""B-087: v081 の固定軌跡上で退去イベント別の期待負荷曲線を検証する。

偶数 case で候補解析式を pressure MAE により一つ選び、奇数 case は選択に
使わない検証集合とする。検証集合で load RMSE と pressure MAE がともに現行
global curve より小さく、case 単位の pressure MAE でも過半数勝てば premise pass。
これは solver を起動せず、保存済み入出力だけを読む補助 probe である。
"""

from __future__ import annotations

import argparse
import math
from dataclasses import dataclass
from pathlib import Path


HORIZON = 100_000
EXPECTED_P = 59.4974499956
X = (0.112_701_665_379_3, 0.5, 0.887_298_334_620_7)
W = (5.0 / 18.0, 8.0 / 18.0, 5.0 / 18.0)
MODELS = (
    "event_fixed",
    "event_fixed_right",
    "event_remaining",
    "event_remaining_right",
)


@dataclass(frozen=True)
class Group:
    S: int
    T: int
    P: int
    V: int


@dataclass
class Stats:
    weight: float = 0.0
    sq_error: float = 0.0
    abs_pressure_error: float = 0.0

    def add(self, predicted: float, oracle: float, capacity: float, weight: float) -> None:
        error = predicted - oracle
        self.weight += weight
        self.sq_error += weight * error * error
        predicted_pressure = max(predicted / capacity - 1.0, 0.0)
        oracle_pressure = max(oracle / capacity - 1.0, 0.0)
        self.abs_pressure_error += weight * abs(predicted_pressure - oracle_pressure)

    @property
    def load_rmse(self) -> float:
        return math.sqrt(self.sq_error / self.weight)

    @property
    def pressure_mae(self) -> float:
        return self.abs_pressure_error / self.weight

    def merge(self, other: "Stats") -> None:
        self.weight += other.weight
        self.sq_error += other.sq_error
        self.abs_pressure_error += other.abs_pressure_error


def parse_input(path: Path) -> tuple[list[str], list[Group]]:
    tokens = path.read_text().split()
    pos = 0
    N = int(tokens[pos])
    M = int(tokens[pos + 1])
    pos += 3
    board = tokens[pos : pos + N]
    pos += N
    groups: list[Group] = []
    for expected_id in range(M):
        group_id = int(tokens[pos])
        S = int(tokens[pos + 1])
        T = int(tokens[pos + 2])
        P = int(tokens[pos + 3])
        V = int(tokens[pos + 4])
        pos += 5
        assert group_id == expected_id
        groups.append(Group(S, T, P, V))
    assert pos == len(tokens)
    return board, groups


def parse_accepts(path: Path, groups: list[Group]) -> list[bool]:
    tokens = path.read_text().split()
    pos = 0
    accepted: list[bool] = []
    for turn, group in enumerate(groups):
        moved = int(tokens[pos])
        pos += 1
        for _ in range(moved):
            group_id = int(tokens[pos])
            pos += 1 + 2 * groups[group_id].P
        decision = tokens[pos]
        pos += 1
        assert decision in ("Yes", "No"), (path, turn, decision)
        is_accepted = decision == "Yes"
        accepted.append(is_accepted)
        if is_accepted:
            pos += 2 * group.P
    assert pos == len(tokens), (path, pos, len(tokens))
    return accepted


def effective_capacity(board: list[str]) -> float:
    N = len(board)
    seen = [[False] * N for _ in range(N)]
    component_sizes: list[int] = []
    for sr in range(N):
        for sc in range(N):
            if board[sr][sc] != "." or seen[sr][sc]:
                continue
            stack = [(sr, sc)]
            seen[sr][sc] = True
            size = 0
            while stack:
                r, c = stack.pop()
                size += 1
                for dr, dc in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                    nr, nc = r + dr, c + dc
                    if (
                        0 <= nr < N
                        and 0 <= nc < N
                        and board[nr][nc] == "."
                        and not seen[nr][nc]
                    ):
                        seen[nr][nc] = True
                        stack.append((nr, nc))
            component_sizes.append(size)
    usable = sum(size for size in component_sizes if size >= 4)
    component_count = sum(size >= 4 for size in component_sizes)
    grass_count = sum(row.count(".") for row in board)
    pond_count = N * N - grass_count
    pond_factor = min(max(pond_count / 900.0, 0.0), 1.0)
    split_factor = min(max((component_count - 1.0) / 8.0, 0.0), 1.0)
    packing_efficiency = min(max(0.89 - 0.055 * pond_factor - 0.015 * split_factor, 0.80), 0.89)
    return max(packing_efficiency * usable, 1.0)


def posterior_theta(duration_sum: float, duration_count: int) -> float:
    log_weights = []
    for k in range(121):
        theta = 2_000.0 + 50.0 * k
        log_weights.append(-duration_count * math.log(theta) - duration_sum / theta)
    maximum = max(log_weights)
    weights = [math.exp(value - maximum) for value in log_weights]
    return sum((2_000.0 + 50.0 * k) * weight for k, weight in enumerate(weights)) / sum(weights)


def boundary_load_factor(time: float, theta: float) -> float:
    clipped = min(max(time, 0.0), HORIZON)
    left = 1.0 - math.exp(-clipped / theta)
    right = 1.0 - math.exp(-(HORIZON - clipped) / theta)
    return max(left * right, 0.0)


def predicted_future(
    model: str,
    time: float,
    now: int,
    theta: float,
    remaining: int,
) -> float:
    if model.startswith("event_remaining"):
        rate = remaining / max(HORIZON - now, 1)
    else:
        rate = 1_000.0 / HORIZON
    future = rate * EXPECTED_P * theta * (1.0 - math.exp(-(time - now) / theta))
    if model.endswith("_right"):
        future *= 1.0 - math.exp(-(HORIZON - time) / theta)
    return future


def event_points(group: Group, active_ids: list[int], groups: list[Group]) -> list[tuple[float, float]]:
    boundaries = [group.S, group.T]
    boundaries.extend(groups[j].T for j in active_ids if group.S < groups[j].T < group.T)
    boundaries = sorted(set(boundaries))
    duration = group.T - group.S
    return [
        ((left + right) * 0.5, (right - left) / duration)
        for left, right in zip(boundaries, boundaries[1:])
    ]


def evaluate_case(groups: list[Group], accepted: list[bool], capacity: float) -> dict[str, Stats]:
    result = {name: Stats() for name in ("old_global", *MODELS)}
    active_ids: list[int] = []
    duration_sum = 0.0
    for i, group in enumerate(groups):
        active_ids = [j for j in active_ids if groups[j].T > group.S]
        duration_sum += group.T - group.S
        theta = posterior_theta(duration_sum, i + 1)
        offered_area = (1_000.0 / HORIZON) * EXPECTED_P * theta
        for time, weight in event_points(group, active_ids, groups):
            known = sum(groups[j].P for j in active_ids if groups[j].T > time)
            future = 0
            for k in range(i + 1, len(groups)):
                candidate = groups[k]
                if candidate.S > time:
                    break
                if candidate.T > time:
                    future += candidate.P
            oracle = known + future
            old = offered_area * boundary_load_factor(time, theta)
            result["old_global"].add(old, oracle, capacity, weight)
            remaining = len(groups) - i - 1
            for model in MODELS:
                predicted = known + predicted_future(model, time, group.S, theta, remaining)
                result[model].add(predicted, oracle, capacity, weight)
        if accepted[i]:
            active_ids.append(i)
    return result


def print_partition(name: str, case_stats: dict[int, dict[str, Stats]]) -> dict[str, Stats]:
    totals = {model: Stats() for model in ("old_global", *MODELS)}
    for stats in case_stats.values():
        for model, values in stats.items():
            totals[model].merge(values)
    print(f"[{name}] cases={len(case_stats)}")
    for model, values in totals.items():
        print(
            f"  {model:21s} load_rmse={values.load_rmse:9.3f} "
            f"pressure_mae={values.pressure_mae:.6f}"
        )
    return totals


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", type=Path, default=Path("tools/in"))
    parser.add_argument(
        "--outputs", type=Path, default=Path("results/out/v081_deep_terminal_hybrid")
    )
    args = parser.parse_args()

    per_case: dict[int, dict[str, Stats]] = {}
    for input_path in sorted(args.inputs.glob("*.txt")):
        case_id = int(input_path.stem)
        error_path = args.outputs / f"{input_path.name}.err"
        if "route_smooth=1" not in error_path.read_text():
            continue
        board, groups = parse_input(input_path)
        accepted = parse_accepts(args.outputs / input_path.name, groups)
        per_case[case_id] = evaluate_case(groups, accepted, effective_capacity(board))

    development = {case: stats for case, stats in per_case.items() if case % 2 == 0}
    validation = {case: stats for case, stats in per_case.items() if case % 2 == 1}
    dev_totals = print_partition("development-even", development)
    validation_totals = print_partition("validation-odd", validation)

    selected = min(MODELS, key=lambda model: dev_totals[model].pressure_mae)
    old = validation_totals["old_global"]
    chosen = validation_totals[selected]
    case_wins = sum(
        stats[selected].pressure_mae < stats["old_global"].pressure_mae
        for stats in validation.values()
    )
    ties = sum(
        stats[selected].pressure_mae == stats["old_global"].pressure_mae
        for stats in validation.values()
    )
    premise_pass = (
        chosen.load_rmse < old.load_rmse
        and chosen.pressure_mae < old.pressure_mae
        and case_wins > len(validation) / 2
    )
    print(
        f"selected={selected} validation_pressure_cases={case_wins}W/{ties}T/"
        f"{len(validation) - case_wins - ties}L premise={'PASS' if premise_pass else 'FAIL'}"
    )


if __name__ == "__main__":
    main()
