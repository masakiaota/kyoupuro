"""評価画面に必要な個票と全件集計を、正本ログから事前に生成する。"""

from __future__ import annotations

from collections import defaultdict
from contextlib import contextmanager
import fcntl
import json
from pathlib import Path
from tempfile import NamedTemporaryFile

from eval_records import RECORDS_DIR, record_paths

VIEW_DIR = RECORDS_DIR.parent / "eval_view_data"
CASE_COLUMN_LIMIT = 100


@contextmanager
def eval_view_lock():
    VIEW_DIR.mkdir(parents=True, exist_ok=True)
    with (VIEW_DIR / ".lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


def project_dataset(runs):
    best_by_case = {}
    for run in runs:
        for name, score in run["caseScores"].items():
            best_by_case[name] = min(score, best_by_case.get(name, score))
    all_case_names = sorted(best_by_case)
    case_names = all_case_names[:CASE_COLUMN_LIMIT]
    for run in runs:
        scores = run["caseScores"]
        elapsed = run["caseElapsed"]
        run["totalAvg"] = sum(scores.values()) / len(scores)
        run["maxElapsed"] = max(elapsed.values())
        # 表示範囲の変更で、全件での相対評価や並べ替えが変わらないようにする。
        rel_sum = 0.0
        for name in all_case_names:
            if name in scores:
                best, score = best_by_case[name], scores[name]
                rel_sum += 1 if best == score else best / score
        run["rel"] = rel_sum / len(all_case_names)
        run["caseScores"] = {name: scores[name] for name in case_names if name in scores}
        run["caseElapsed"] = {name: elapsed[name] for name in case_names if name in elapsed}
    return {"caseNames": case_names, "runs": runs}


def build_snapshot(source):
    runs = {}
    failed = set()
    latest_by_bin = {}
    with source.open(encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            row = json.loads(line)
            if not isinstance(row, dict):
                continue
            bin_name, executed_at = row.get("bin", ""), row.get("executed_at", "")
            if isinstance(bin_name, str) and isinstance(executed_at, str):
                latest_by_bin[bin_name] = max(executed_at, latest_by_bin.get(bin_name, ""))
            run_id = row.get("run_id")
            if not isinstance(run_id, str) or not run_id or run_id in failed:
                continue
            name, score, elapsed = row.get("case_name"), row.get("score"), row.get("elapsed")
            if (row.get("status") != "ok" or not isinstance(name, str) or not name
                    or type(score) not in (int, float) or type(elapsed) not in (int, float)):
                failed.add(run_id)
                continue
            metadata = dict(zip(
                ("bin", "label", "executedAt", "evalSet"),
                (row.get(key, "") for key in ("bin", "label", "executed_at", "input_dir")),
            ))
            metadata = {key: value if isinstance(value, str) else "" for key, value in metadata.items()}
            run = runs.get(run_id)
            if run is None:
                run = runs[run_id] = dict(metadata, id=run_id, caseScores={}, caseElapsed={})
            elif any(run[key] != value for key, value in metadata.items()):
                failed.add(run_id)
                continue
            run["caseScores"][name] = score
            run["caseElapsed"][name] = elapsed

    datasets = defaultdict(list)
    for run_id, run in runs.items():
        if run_id not in failed:
            datasets[run.pop("evalSet")].append(run)
    return {
        "datasets": {name: project_dataset(dataset_runs) for name, dataset_runs in datasets.items()},
        "latestExecutedAtByBin": latest_by_bin,
    }


def sync_eval_view_file(source: Path) -> Path | None:
    relative = source.relative_to(RECORDS_DIR)
    if relative.parts[0] in {"adhoc", "adhoc.jsonl"}:
        return None
    destination = (VIEW_DIR / relative).with_suffix(".json")
    stat = source.stat()
    version = [stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns]
    if destination.is_file():
        with destination.open(encoding="utf-8") as handle:
            if json.load(handle)["sourceVersion"] == version:
                return destination

    snapshot = build_snapshot(source)
    snapshot["sourceVersion"] = version
    destination.parent.mkdir(parents=True, exist_ok=True)
    with NamedTemporaryFile(dir=destination.parent, mode="w", encoding="utf-8", delete=False) as handle:
        temporary = Path(handle.name)
        try:
            json.dump(snapshot, handle, ensure_ascii=False, separators=(",", ":"))
            handle.close()
            temporary.replace(destination)
        finally:
            temporary.unlink(missing_ok=True)
    return destination


def main():
    with eval_view_lock():
        current = set()
        for source in record_paths():
            destination = sync_eval_view_file(source)
            if destination is not None:
                current.add(destination)
        for stale in set(VIEW_DIR.rglob("*.json")) - current:
            stale.unlink()


if __name__ == "__main__":
    main()
