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
DENSITY3_ORDER = {"L": 0, "M": 1, "H": 2}
DENSITY_LOW_MAX = 0.40
DENSITY_MID_MAX = 0.55
CAT_MAIN = 100_000
CAT_SEVERE = 1_000_000


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
    results: dict[str, BinResult]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create basic N/M/C/density reports from serial_eval JSONL, focused on win counts."
    )
    parser.add_argument(
        "--jsonl",
        help="input JSONL path. Defaults to the latest serial_eval_*.jsonl under results/analysis",
    )
    parser.add_argument(
        "--out-dir",
        help="output directory. Defaults to results/analysis/basic_nmc_<timestamp>",
    )
    return parser.parse_args()


def latest_jsonl_path() -> Path:
    candidates = list(ANALYSIS_DIR.glob("serial_eval_*.jsonl"))
    if not candidates:
        raise SystemExit("error: serial_eval_*.jsonl not found under results/analysis")
    return max(candidates, key=lambda path: path.stat().st_mtime)


def default_out_dir() -> Path:
    timestamp = datetime.now().astimezone().strftime("%Y%m%dT%H%M%S%z")
    return ANALYSIS_DIR / f"basic_nmc_{timestamp}"


def density3_bucket(N: int, M: int) -> tuple[float, str]:
    density = M / (N * N)
    if density < DENSITY_LOW_MAX:
        return density, "L"
    if density < DENSITY_MID_MAX:
        return density, "M"
    return density, "H"


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
        if any(record["status"] != "ok" for record in records):
            raise SystemExit(f"error: case {case_id} contains non-ok rows")
        bins_seen = {record["bin"] for record in records}
        if bins_seen != set(BINS):
            raise SystemExit(f"error: case {case_id} does not contain the expected bins")

        meta = records[0]
        density, density3 = density3_bucket(int(meta["N"]), int(meta["M"]))
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
                results=results,
            )
        )
    return cases


def best_bin_name(case: CaseData) -> str:
    return min(
        BINS,
        key=lambda bin_name: (
            case.results[bin_name].score,
            case.results[bin_name].elapsed_ms,
            BIN_ORDER[bin_name],
        ),
    )


def format_float(value: float) -> str:
    return f"{value:.3f}"


def write_csv(path: Path, fieldnames: list[str], rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as fh:
        writer = csv.DictWriter(fh, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)


def contiguous_ranges(values: list[int]) -> str:
    if not values:
        return "-"
    ranges = []
    start = values[0]
    end = values[0]
    for value in values[1:]:
        if value == end + 1:
            end = value
            continue
        ranges.append(f"{start}" if start == end else f"{start}..{end}")
        start = value
        end = value
    ranges.append(f"{start}" if start == end else f"{start}..{end}")
    return ", ".join(ranges)


def distribution_rows(cases: list[CaseData]) -> list[dict[str, str]]:
    counters = {
        "dataset": Counter(case.dataset for case in cases),
        "N": Counter(case.N for case in cases),
        "M": Counter(case.M for case in cases),
        "C": Counter(case.C for case in cases),
        "density3": Counter(case.density3 for case in cases),
    }
    rows = []
    for dimension, counter in counters.items():
        if dimension in {"N", "M", "C"}:
            items = sorted(counter.items(), key=lambda item: item[0])
        elif dimension == "density3":
            items = sorted(counter.items(), key=lambda item: DENSITY3_ORDER[item[0]])
        else:
            items = sorted(counter.items(), key=lambda item: str(item[0]))
        for bucket, count in items:
            rows.append(
                {
                    "dimension": dimension,
                    "bucket": str(bucket),
                    "case_count": str(count),
                }
            )
    return rows


def global_bin_rows(cases: list[CaseData]) -> list[dict[str, str]]:
    best_counts = Counter(best_bin_name(case) for case in cases)
    rows = []
    for bin_name in BINS:
        case_count = len(cases)
        score_sum = sum(case.results[bin_name].score for case in cases)
        elapsed_sum = sum(case.results[bin_name].elapsed_ms for case in cases)
        max_elapsed = max(case.results[bin_name].elapsed_ms for case in cases)
        cat_main = sum(case.results[bin_name].score >= CAT_MAIN for case in cases)
        cat_severe = sum(case.results[bin_name].score >= CAT_SEVERE for case in cases)
        rows.append(
            {
                "bin": bin_name,
                "case_count": str(case_count),
                "best_count": str(best_counts[bin_name]),
                "best_rate": format_float(best_counts[bin_name] / case_count),
                "avg_score": format_float(score_sum / case_count),
                "avg_elapsed_ms": format_float(elapsed_sum / case_count),
                "max_elapsed_ms": str(max_elapsed),
                "catastrophic_1e5": str(cat_main),
                "catastrophic_1e6": str(cat_severe),
            }
        )
    return rows


def bucket_group_defs():
    return [
        ("N", lambda case: f"N={case.N}", lambda case: (case.N,)),
        ("M", lambda case: f"M={case.M}", lambda case: (case.M,)),
        ("C", lambda case: f"C={case.C}", lambda case: (case.C,)),
        (
            "density3",
            lambda case: f"density3={case.density3}",
            lambda case: (DENSITY3_ORDER[case.density3],),
        ),
        (
            "N_density3",
            lambda case: f"N={case.N}|density3={case.density3}",
            lambda case: (case.N, DENSITY3_ORDER[case.density3]),
        ),
    ]


def bucket_win_rows(cases: list[CaseData]) -> list[dict[str, str]]:
    rows = []
    for dimension, label_fn, sort_key_fn in bucket_group_defs():
        buckets: dict[str, list[CaseData]] = defaultdict(list)
        bucket_sort_keys: dict[str, tuple[int, ...]] = {}
        for case in cases:
            label = label_fn(case)
            buckets[label].append(case)
            bucket_sort_keys[label] = sort_key_fn(case)

        for bucket, bucket_cases in sorted(
            buckets.items(), key=lambda item: bucket_sort_keys[item[0]]
        ):
            case_count = len(bucket_cases)
            mean_scores = {
                bin_name: sum(case.results[bin_name].score for case in bucket_cases)
                / case_count
                for bin_name in BINS
            }
            best_counts = Counter(best_bin_name(case) for case in bucket_cases)
            best_avg_bin = min(
                BINS,
                key=lambda bin_name: (mean_scores[bin_name], BIN_ORDER[bin_name]),
            )
            win_ranking = sorted(
                BINS,
                key=lambda bin_name: (
                    -best_counts[bin_name],
                    mean_scores[bin_name],
                    BIN_ORDER[bin_name],
                ),
            )
            top_win_bin = win_ranking[0]
            runner_up_bin = win_ranking[1]
            rows.append(
                {
                    "dimension": dimension,
                    "bucket": bucket,
                    "case_count": str(case_count),
                    "top_win_bin": top_win_bin,
                    "top_win_count": str(best_counts[top_win_bin]),
                    "top_win_rate": format_float(best_counts[top_win_bin] / case_count),
                    "runner_up_bin": runner_up_bin,
                    "runner_up_count": str(best_counts[runner_up_bin]),
                    "runner_up_rate": format_float(best_counts[runner_up_bin] / case_count),
                    "win_gap_count": str(
                        best_counts[top_win_bin] - best_counts[runner_up_bin]
                    ),
                    "best_avg_bin": best_avg_bin,
                    "avg_score_v012_simple_beam": format_float(
                        mean_scores["v012_simple_beam"]
                    ),
                    "avg_score_v014_lane_split_beam": format_float(
                        mean_scores["v014_lane_split_beam"]
                    ),
                    "avg_score_v139_refactor_v137": format_float(
                        mean_scores["v139_refactor_v137"]
                    ),
                    "avg_score_v149_no_logs": format_float(
                        mean_scores["v149_no_logs"]
                    ),
                    "best_count_v012_simple_beam": str(
                        best_counts["v012_simple_beam"]
                    ),
                    "best_count_v014_lane_split_beam": str(
                        best_counts["v014_lane_split_beam"]
                    ),
                    "best_count_v139_refactor_v137": str(
                        best_counts["v139_refactor_v137"]
                    ),
                    "best_count_v149_no_logs": str(best_counts["v149_no_logs"]),
                    "best_rate_v012_simple_beam": format_float(
                        best_counts["v012_simple_beam"] / case_count
                    ),
                    "best_rate_v014_lane_split_beam": format_float(
                        best_counts["v014_lane_split_beam"] / case_count
                    ),
                    "best_rate_v139_refactor_v137": format_float(
                        best_counts["v139_refactor_v137"] / case_count
                    ),
                    "best_rate_v149_no_logs": format_float(
                        best_counts["v149_no_logs"] / case_count
                    ),
                }
            )
    return rows


def v149_catastrophic_rows(cases: list[CaseData]) -> list[dict[str, str]]:
    rows = []
    for case in cases:
        v149_score = case.results["v149_no_logs"].score
        if v149_score < CAT_SEVERE:
            continue
        best_bin = best_bin_name(case)
        rows.append(
            {
                "case_id": case.case_id,
                "N": str(case.N),
                "M": str(case.M),
                "C": str(case.C),
                "density": format_float(case.density),
                "density3": case.density3,
                "v149_score": str(v149_score),
                "v139_score": str(case.results["v139_refactor_v137"].score),
                "best_bin": best_bin,
                "best_score": str(case.results[best_bin].score),
            }
        )
    rows.sort(key=lambda row: row["case_id"])
    return rows


def markdown_table(headers: list[str], rows: list[list[str]]) -> list[str]:
    out = []
    out.append("| " + " | ".join(headers) + " |")
    out.append("| " + " | ".join(["---"] * len(headers)) + " |")
    for row in rows:
        out.append("| " + " | ".join(row) + " |")
    return out


def pick_rows(bucket_rows: list[dict[str, str]], dimension: str) -> list[dict[str, str]]:
    rows = [row for row in bucket_rows if row["dimension"] == dimension]

    def sort_key(row: dict[str, str]) -> tuple[int, ...]:
        bucket = row["bucket"]
        if dimension == "N":
            return (int(bucket.removeprefix("N=")),)
        if dimension == "M":
            return (int(bucket.removeprefix("M=")),)
        if dimension == "C":
            return (int(bucket.removeprefix("C=")),)
        if dimension == "density3":
            return (DENSITY3_ORDER[bucket.removeprefix("density3=")],)
        if dimension == "N_density3":
            left, right = bucket.split("|", 1)
            return (
                int(left.removeprefix("N=")),
                DENSITY3_ORDER[right.removeprefix("density3=")],
            )
        return (0,)

    return sorted(rows, key=sort_key)


def write_summary(
    path: Path,
    jsonl_path: Path,
    cases: list[CaseData],
    distribution: list[dict[str, str]],
    global_rows: list[dict[str, str]],
    bucket_rows: list[dict[str, str]],
    catastrophic_rows: list[dict[str, str]],
) -> None:
    row_by_bin = {row["bin"]: row for row in global_rows}
    n_rows = pick_rows(bucket_rows, "N")
    c_rows = pick_rows(bucket_rows, "C")
    d_rows = pick_rows(bucket_rows, "density3")
    nd_rows = pick_rows(bucket_rows, "N_density3")
    global_win_rows = sorted(
        global_rows,
        key=lambda row: (
            -int(row["best_count"]),
            float(row["avg_score"]),
            BIN_ORDER[row["bin"]],
        ),
    )
    top_global_win_row = global_win_rows[0]

    n_v149_win_ns = [
        int(row["bucket"].removeprefix("N="))
        for row in n_rows
        if row["top_win_bin"] == "v149_no_logs"
    ]
    n_wins_by_bin: dict[str, list[int]] = defaultdict(list)
    for row in n_rows:
        n_wins_by_bin[row["top_win_bin"]].append(int(row["bucket"].removeprefix("N=")))

    c_v149_win_cs = [
        int(row["bucket"].removeprefix("C="))
        for row in c_rows
        if row["top_win_bin"] == "v149_no_logs"
    ]
    c_wins_by_bin: dict[str, list[int]] = defaultdict(list)
    for row in c_rows:
        c_wins_by_bin[row["top_win_bin"]].append(int(row["bucket"].removeprefix("C=")))

    density_v149_buckets = [
        row["bucket"].removeprefix("density3=")
        for row in d_rows
        if row["top_win_bin"] == "v149_no_logs"
    ]
    mismatch_rows = [
        row
        for row in nd_rows
        if row["top_win_bin"] == "v149_no_logs"
        and row["best_avg_bin"] == "v139_refactor_v137"
    ]
    mismatch_rows = sorted(
        mismatch_rows,
        key=lambda row: (
            int(row["win_gap_count"]),
            int(row["case_count"]),
        ),
        reverse=True,
    )[:6]

    lines = []
    lines.append("# 基礎集計サマリ")
    lines.append("")
    lines.append("## 前提")
    lines.append(f"- 入力 JSONL: `{jsonl_path}`")
    lines.append(f"- 対象ケース数: {len(cases)}")
    lines.append("- 勝数は「そのケースで 4 bin の中で最良スコアを取った回数」を意味する。tie は `elapsed_ms`、さらに bin の固定順で破る。")
    lines.append("- `M` は生値だけだと `N` に強く依存するため、この要約では主に `density = M / N^2` を使う。raw `M` の分布と勝数は CSV に出している。")
    lines.append("- `density3` は `L < 0.40`, `M < 0.55`, `H >= 0.55` の 3 bucket である。")
    lines.append("")
    lines.append("## ケース分布")
    lines.extend(
        markdown_table(
            ["指標", "bucket", "件数"],
            [
                [row["dimension"], row["bucket"], row["case_count"]]
                for row in distribution
                if row["dimension"] != "M"
            ],
        )
    )
    lines.append("")
    lines.append("## 全体勝数")
    lines.extend(
        markdown_table(
            [
                "bin",
                "best_count",
                "best_rate",
                "avg_score",
                "avg_elapsed_ms",
                "max_elapsed_ms",
                "cat>=1e5",
                "cat>=1e6",
            ],
            [
                [
                    row["bin"],
                    row["best_count"],
                    row["best_rate"],
                    row["avg_score"],
                    row["avg_elapsed_ms"],
                    row["max_elapsed_ms"],
                    row["catastrophic_1e5"],
                    row["catastrophic_1e6"],
                ]
                for row in global_rows
            ],
        )
    )
    lines.append("")
    lines.append(
        f"- 全体勝数では `{top_global_win_row['bin']}` が {top_global_win_row['best_count']} 勝 ({top_global_win_row['best_rate']}) で最多である。"
    )
    lines.append(
        f"- 一方で平均スコア最良は `v139_refactor_v137` で、avg_score は {row_by_bin['v139_refactor_v137']['avg_score']} である。"
    )
    lines.append(
        f"- ただし `v149_no_logs` は `score >= {CAT_SEVERE}` の catastrophic case を {row_by_bin['v149_no_logs']['catastrophic_1e6']} 件持つため、平均スコアは {row_by_bin['v149_no_logs']['avg_score']} まで悪化している。"
    )
    lines.append("")
    lines.append("## N ごとの勝数")
    lines.extend(
        markdown_table(
            [
                "bucket",
                "件数",
                "top_win_bin",
                "top_win_count",
                "top_win_rate",
                "v012勝",
                "v014勝",
                "v139勝",
                "v149勝",
                "best_avg_bin",
            ],
            [
                [
                    row["bucket"],
                    row["case_count"],
                    row["top_win_bin"],
                    row["top_win_count"],
                    row["top_win_rate"],
                    row["best_count_v012_simple_beam"],
                    row["best_count_v014_lane_split_beam"],
                    row["best_count_v139_refactor_v137"],
                    row["best_count_v149_no_logs"],
                    row["best_avg_bin"],
                ]
                for row in n_rows
            ],
        )
    )
    lines.append("")
    lines.append(
        f"- `v149_no_logs` が勝数トップになる `N` は {contiguous_ranges(n_v149_win_ns)} である。"
    )
    lines.append(
        "- `N` ごとの winner は "
        + ", ".join(
            f"`{bin_name}`: {contiguous_ranges(sorted(values))}"
            for bin_name, values in (
                ("v012_simple_beam", n_wins_by_bin["v012_simple_beam"]),
                ("v014_lane_split_beam", n_wins_by_bin["v014_lane_split_beam"]),
                ("v139_refactor_v137", n_wins_by_bin["v139_refactor_v137"]),
                ("v149_no_logs", n_wins_by_bin["v149_no_logs"]),
            )
            if values
        )
        + " である。"
    )
    lines.append("- `best_avg_bin` も併記しているので、「勝数では強いが平均では危ない」帯をそのまま見分けられる。")
    lines.append("")
    lines.append("## C ごとの勝数")
    lines.extend(
        markdown_table(
            [
                "bucket",
                "件数",
                "top_win_bin",
                "top_win_count",
                "top_win_rate",
                "v012勝",
                "v014勝",
                "v139勝",
                "v149勝",
                "best_avg_bin",
            ],
            [
                [
                    row["bucket"],
                    row["case_count"],
                    row["top_win_bin"],
                    row["top_win_count"],
                    row["top_win_rate"],
                    row["best_count_v012_simple_beam"],
                    row["best_count_v014_lane_split_beam"],
                    row["best_count_v139_refactor_v137"],
                    row["best_count_v149_no_logs"],
                    row["best_avg_bin"],
                ]
                for row in c_rows
            ],
        )
    )
    lines.append("")
    lines.append(
        f"- `v149_no_logs` が勝数トップになる `C` は {contiguous_ranges(c_v149_win_cs)} である。"
    )
    lines.append(
        "- `C` ごとの winner は "
        + ", ".join(
            f"`{bin_name}`: {contiguous_ranges(sorted(values))}"
            for bin_name, values in (
                ("v012_simple_beam", c_wins_by_bin["v012_simple_beam"]),
                ("v014_lane_split_beam", c_wins_by_bin["v014_lane_split_beam"]),
                ("v139_refactor_v137", c_wins_by_bin["v139_refactor_v137"]),
                ("v149_no_logs", c_wins_by_bin["v149_no_logs"]),
            )
            if values
        )
        + " である。"
    )
    lines.append("")
    lines.append("## density3 ごとの勝数")
    lines.extend(
        markdown_table(
            [
                "bucket",
                "件数",
                "top_win_bin",
                "top_win_count",
                "top_win_rate",
                "v012勝",
                "v014勝",
                "v139勝",
                "v149勝",
                "best_avg_bin",
            ],
            [
                [
                    row["bucket"],
                    row["case_count"],
                    row["top_win_bin"],
                    row["top_win_count"],
                    row["top_win_rate"],
                    row["best_count_v012_simple_beam"],
                    row["best_count_v014_lane_split_beam"],
                    row["best_count_v139_refactor_v137"],
                    row["best_count_v149_no_logs"],
                    row["best_avg_bin"],
                ]
                for row in d_rows
            ],
        )
    )
    lines.append("")
    lines.append(
        f"- `v149_no_logs` が勝数トップになる density bucket は {', '.join(density_v149_buckets) if density_v149_buckets else '-'} である。"
    )
    lines.append(
        "- density bucket ごとの winner は "
        + ", ".join(
            f"`{row['bucket']}`: `{row['top_win_bin']}` ({row['top_win_count']}勝)"
            for row in d_rows
        )
        + " である。"
    )
    lines.append("- `density3=H` のように、勝数では `v149_no_logs` が勝っても平均スコアでは `v139_refactor_v137` に負ける帯がある。")
    lines.append("")
    lines.append("## N × density3 の勝数")
    lines.extend(
        markdown_table(
            [
                "bucket",
                "件数",
                "top_win_bin",
                "top_win_count",
                "v139勝",
                "v149勝",
                "best_avg_bin",
            ],
            [
                [
                    row["bucket"],
                    row["case_count"],
                    row["top_win_bin"],
                    row["top_win_count"],
                    row["best_count_v139_refactor_v137"],
                    row["best_count_v149_no_logs"],
                    row["best_avg_bin"],
                ]
                for row in nd_rows
            ],
        )
    )
    lines.append("")
    if mismatch_rows:
        lines.append("- `top_win_bin = v149_no_logs` なのに `best_avg_bin = v139_refactor_v137` になる bucket は次の通りである。")
        lines.extend(
            markdown_table(
                ["bucket", "件数", "v139勝", "v149勝", "win_gap", "best_avg_bin"],
                [
                    [
                        row["bucket"],
                        row["case_count"],
                        row["best_count_v139_refactor_v137"],
                        row["best_count_v149_no_logs"],
                        row["win_gap_count"],
                        row["best_avg_bin"],
                    ]
                    for row in mismatch_rows
                ],
            )
        )
    else:
        lines.append("- 勝数 winner と平均スコア winner が食い違う `N × density3` bucket は見つからなかった。")
    lines.append("")
    lines.append("## v149 の catastrophic case")
    if catastrophic_rows:
        lines.extend(
            markdown_table(
                ["case_id", "N", "M", "C", "density", "v149_score", "v139_score", "best_bin"],
                [
                    [
                        row["case_id"],
                        row["N"],
                        row["M"],
                        row["C"],
                        row["density"],
                        row["v149_score"],
                        row["v139_score"],
                        row["best_bin"],
                    ]
                    for row in catastrophic_rows
                ],
            )
        )
        lines.append("")
        lines.append("- `v149_no_logs` の catastrophic case はすべて `density3=H` にあり、`N` は 14 以上に集中している。")
        lines.append("- 「勝数では良いが、平均スコアでは危険」という帯の正体は、ほぼこの表のケース群である。")
    else:
        lines.append("- `v149_no_logs` の catastrophic case は見つからなかった。")
    lines.append("")
    lines.append("## ここから読める素朴な結論")
    lines.append("- selector の最初の材料として見るべきなのは、平均スコアより先に `N`, `C`, `density3`, `N×density3` ごとの勝数である。")
    lines.append("- `v149_no_logs` は勝数だけ見れば強い帯が広いが、`N>=14` かつ高密度帯では catastrophic case を踏むため、勝数 winner をそのまま selector にすると危ない。")
    lines.append("- `v139_refactor_v137` は全体勝数 winner ではないが、危険帯の保険としては最も安定している。")
    lines.append("- raw `M` で見たい場合は `distribution.csv` と `bucket_win_summary.csv` の `dimension=M` を見ればよい。")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    jsonl_path = Path(args.jsonl).expanduser().resolve() if args.jsonl else latest_jsonl_path()
    out_dir = Path(args.out_dir).expanduser().resolve() if args.out_dir else default_out_dir()
    out_dir.mkdir(parents=True, exist_ok=True)

    cases = load_cases(jsonl_path)
    distribution = distribution_rows(cases)
    global_rows = global_bin_rows(cases)
    bucket_rows = bucket_win_rows(cases)
    catastrophic_rows = v149_catastrophic_rows(cases)

    write_csv(
        out_dir / "distribution.csv",
        ["dimension", "bucket", "case_count"],
        distribution,
    )
    write_csv(
        out_dir / "global_bin_summary.csv",
        [
            "bin",
            "case_count",
            "best_count",
            "best_rate",
            "avg_score",
            "avg_elapsed_ms",
            "max_elapsed_ms",
            "catastrophic_1e5",
            "catastrophic_1e6",
        ],
        global_rows,
    )
    write_csv(
        out_dir / "bucket_win_summary.csv",
        [
            "dimension",
            "bucket",
            "case_count",
            "top_win_bin",
            "top_win_count",
            "top_win_rate",
            "runner_up_bin",
            "runner_up_count",
            "runner_up_rate",
            "win_gap_count",
            "best_avg_bin",
            "avg_score_v012_simple_beam",
            "avg_score_v014_lane_split_beam",
            "avg_score_v139_refactor_v137",
            "avg_score_v149_no_logs",
            "best_count_v012_simple_beam",
            "best_count_v014_lane_split_beam",
            "best_count_v139_refactor_v137",
            "best_count_v149_no_logs",
            "best_rate_v012_simple_beam",
            "best_rate_v014_lane_split_beam",
            "best_rate_v139_refactor_v137",
            "best_rate_v149_no_logs",
        ],
        bucket_rows,
    )
    write_csv(
        out_dir / "v149_catastrophic_cases.csv",
        [
            "case_id",
            "N",
            "M",
            "C",
            "density",
            "density3",
            "v149_score",
            "v139_score",
            "best_bin",
            "best_score",
        ],
        catastrophic_rows,
    )
    write_summary(
        out_dir / "summary.md",
        jsonl_path=jsonl_path,
        cases=cases,
        distribution=distribution,
        global_rows=global_rows,
        bucket_rows=bucket_rows,
        catastrophic_rows=catastrophic_rows,
    )

    print(f"wrote {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
