#!/usr/bin/env python3
"""長時間教師の保存経路を独立再生し、入力単位の品質を判定する。"""
import csv
from datetime import datetime, timezone
import gzip
import hashlib
import json
from pathlib import Path

from run_v047_long_search import Problem

ROOT = Path(__file__).resolve().parents[2]


def now():
    return datetime.now(timezone.utc).isoformat()


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def save(path, value):
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n")
    temp.replace(path)


def status(run, stage, **values):
    save(run / "status.json", {"stage": stage, "updated_at": now(), **values})
    print(json.dumps({"stage": stage, **values}, ensure_ascii=False), flush=True)


def move_hash(moves):
    h = 1469598103934665603
    for p, k, d, length in moves:
        assert all(type(v) is int for v in (p, k, d, length))
        assert 0 <= p < 400 and 0 <= k < 8 and 0 <= d < 4 and 1 <= length <= 8
        packed = p | (k << 16) | (d << 24) | (length << 32)
        h = ((h ^ packed) * 1099511628211) & ((1 << 64) - 1)
    return str(h)


def packed_text(problem, moves):
    cells = [(i, j) for i, row in enumerate(problem.C) for j, c in enumerate(row) if c != "#"]
    lines = []
    for p, k, d, length in moves:
        i, j = cells[p]
        lines.append(f"{i} {j} {k} {'UDLR'[d]} {length}\n")
    return "".join(lines)


def validate_episodes(folder, input_path):
    info = json.loads((folder / "teacher_stats.json").read_text())
    assert info["ring_limit"] == info["episode_limit"] == 64
    assert 0 <= info["episodes_saved"] <= min(64, info["episodes_seen"])
    problem = Problem.read(input_path)
    selected = {0, info["episodes_saved"] // 2, info["episodes_saved"] - 1}
    path = folder / "episodes.jsonl"
    opener = path.open
    if not path.exists():
        path = folder / "episodes.jsonl.gz"
        opener = lambda mode: gzip.open(path, mode)
    seen, checked = {}, set()
    counts = dict(episodes=0, transitions=0, unique_transitions=0, repeated_transitions=0,
                  neutral=0, downhill=0, uphill=0, independently_replayed_plans=0,
                  transitions_with_ids=0)

    def replay(moves, digest):
        if digest not in checked:
            result = problem.replay(packed_text(problem, moves))
            assert result["T"] == len(moves) and result["E"] == 0
            checked.add(digest)
            counts["independently_replayed_plans"] += 1

    with opener("rt") as stream:
        for index, line in enumerate(stream):
            episode = json.loads(line)
            assert episode["schema_version"] == 1
            transitions = episode["transitions"]
            assert 1 <= len(transitions) <= 64
            start = episode["start"]
            prior_hash, prior_T = move_hash(start), len(start)
            if index in selected:
                replay(start, prior_hash)
            for row in transitions:
                assert row["before_T"] == prior_T and row["before_hash"] == prior_hash
                assert row["after_T"] == len(row["after"])
                assert row["after_hash"] == move_hash(row["after"])
                assert row["after_hash"] != prior_hash
                signature = hashlib.sha256(json.dumps(row, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
                key = row["transition_id"]
                if key in seen:
                    assert seen[key] == signature, f"inconsistent duplicate transition: {key}"
                    counts["repeated_transitions"] += 1
                else:
                    seen[key] = signature
                    counts["unique_transitions"] += 1
                    delta = row["after_T"] - prior_T
                    counts["neutral" if delta == 0 else "downhill" if delta < 0 else "uphill"] += 1
                    counts["transitions_with_ids"] += bool(row.get("selection", {}).get("ids"))
                if index in selected:
                    replay(row["after"], row["after_hash"])
                prior_hash, prior_T = row["after_hash"], row["after_T"]
                counts["transitions"] += 1
            assert prior_T == episode["best_after_T"] < episode["best_before_T"]
            counts["episodes"] += 1
    assert counts["episodes"] == info["episodes_saved"]
    counts["passed"] = True
    return counts


def compress_episodes(folder):
    """検証済みの原文を可逆圧縮し、読み戻しの一致後に原文を除く。"""
    source, target = folder / "episodes.jsonl", folder / "episodes.jsonl.gz"
    if not source.exists():
        assert target.exists()
        return
    assert not target.exists()
    expected = sha(source)
    with source.open("rb") as inp, gzip.open(target, "wb", compresslevel=3) as out:
        for block in iter(lambda: inp.read(1024 * 1024), b""):
            out.write(block)
    h = hashlib.sha256()
    with gzip.open(target, "rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    assert h.hexdigest() == expected
    save(folder / "episodes_compression.json", {"original_sha256": expected,
         "compressed_sha256": sha(target), "original_bytes": source.stat().st_size,
         "compressed_bytes": target.stat().st_size})
    source.unlink()


def quality(run, count):
    import numpy as np
    manifest = json.loads((run / "input_manifest.json").read_text())[:count]
    rows = []
    for item in manifest:
        folder = run / "cases" / f"{item['index']:06d}"
        row = json.loads((folder / "complete.json").read_text())
        assert row["input_sha256"] == item["sha256"] and sha(folder / "best.txt") == row["best_sha256"]
        for name, digest in row["reports"].items():
            assert sha(folder / name) == digest
        assert row["T"] <= row["baseline_T"]
        rows.append({"index": item["index"], "seed": item["seed"], "role": item["role"], "M": item["M"],
                     "baseline_T": row["baseline_T"], "v800_T": row["v800_T"], "v801_T": row["v801_T"],
                     "teacher_T": row["T"], "saved": row["baseline_T"]-row["T"], "source": row["source"],
                     "episodes": row["episodes"], "unique_transitions": row["unique_transitions"]})
    saved = np.array([r["saved"] for r in rows], dtype=np.float64)
    rng = np.random.default_rng(85002)
    boot = np.concatenate([saved[rng.integers(0, count, size=(100, count))].mean(axis=1) for _ in range(60)])
    ci = np.quantile(boot, [.025, .975]).tolist()

    def summary(subset):
        if not subset:
            return {"inputs": 0}
        values = np.array([r["saved"] for r in subset])
        return {"inputs": len(subset), "mean_saved": float(values.mean()), "median_saved": float(np.median(values)),
                "at_least_5_rate": float((values >= 5).mean()), "at_least_10_rate": float((values >= 10).mean())}

    result = {"inputs": count, **summary(rows), "ci95": ci, "bootstrap_seed": 85002,
              "materials": {"episodes": sum(r["episodes"] for r in rows),
                            "unique_transitions": sum(r["unique_transitions"] for r in rows),
                            "terminal_teacher_moves": sum(r["teacher_T"] for r in rows)},
              "bootstrap_replicates": 6000, "M_lt_80": summary([r for r in rows if r["M"] < 80]),
              "M_ge_80": summary([r for r in rows if r["M"] >= 80]),
              "v801_better_than_v800_inputs": sum(r["v801_T"] < r["v800_T"] for r in rows),
              "v800_better_than_v801_inputs": sum(r["v800_T"] < r["v801_T"] for r in rows),
              "v801_minus_v800_mean_T": float(np.mean([r["v801_T"]-r["v800_T"] for r in rows])),
              "gate": {"mean_at_least_10": bool(saved.mean() >= 10), "ci_lower_at_least_5": ci[0] >= 5,
                       "all_legal_nondegrading": True, "passed": bool(saved.mean() >= 10 and ci[0] >= 5)},
              "completed_at": now()}
    save(run / f"quality_{count}.json", result)
    with (run / f"cases_{count}.csv").open("w") as out:
        writer = csv.DictWriter(out, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)
    return result
