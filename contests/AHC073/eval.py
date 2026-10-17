#!/usr/bin/env python3
"""PyPy で解を実行し、公式ツールの Score を集計する。標準ライブラリのみ。"""

import argparse
import ast
from concurrent.futures import ThreadPoolExecutor, as_completed
import csv
from datetime import datetime
from decimal import Decimal
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
SCORE = re.compile(r"^\s*Score\s*=\s*([+-]?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)\s*$", re.M)
SUMMARY_FIELDS = ["run", "source", "cases", "failed", "average", "total", "minimum", "maximum",
                  "max_seconds", "mode", "jobs", "comparison_key"]


def run_process(command, input_path, output_path, log_path, cwd, timeout):
    """時間超過時は tester が起動した解プログラムもまとめて止める。"""
    started = time.perf_counter()
    with open(input_path, "rb") as stdin, open(output_path, "wb") as stdout, open(log_path, "wb") as stderr:
        process = subprocess.Popen(command, stdin=stdin, stdout=stdout, stderr=stderr,
                                   cwd=cwd, start_new_session=True)
        try:
            code = process.wait(timeout=timeout)
            status = "OK" if code == 0 else "RE"
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            status = "TIMEOUT"
    return status, time.perf_counter() - started


def evaluate_one(input_path, run_dir, python, config, timeout):
    case_dir = run_dir / "cases" / input_path.stem
    case_dir.mkdir(parents=True)
    output = case_dir / "out.txt"
    solver_log = case_dir / "solver.log"
    solver = [python, "-u", str(run_dir / "main.py")]
    tool = ROOT / "tools/target/release" / config["mode"]
    command = [str(tool), *config["scorer_args"], *solver] if config["mode"] == "tester" else solver
    status, elapsed = run_process(command, input_path, output, solver_log, case_dir, timeout)
    row = {"case": input_path.name, "status": status, "score": "", "seconds": f"{elapsed:.4f}"}
    if status != "OK":
        return row
    if config["mode"] == "tester":
        text = solver_log.read_text(errors="replace")
    else:
        score_log = case_dir / "scorer.log"
        command = [str(tool), str(input_path), str(output), *config["scorer_args"]]
        status, _ = run_process(command, os.devnull, score_log, case_dir / "scorer.err", case_dir, timeout)
        if status != "OK":
            row["status"] = "SCORER_" + status
            return row
        text = score_log.read_text(errors="replace")
    matches = list(SCORE.finditer(text))
    if not matches:
        row["status"] = "NO_SCORE"
        return row
    row["score"] = matches[-1].group(1)
    if config["mode"] == "vis" and SCORE.sub("", text).strip():
        # 公式 vis は不正解でも exit 0 + 説明文 + Score = 0 を返す。
        row["status"] = "INVALID"
    return row


def find_pypy(requested):
    path = requested
    if not path:
        local = ROOT / ".venv/bin/python"
        path = str(local) if local.exists() else shutil.which("pypy3")
    if not path:
        raise ValueError("PyPy が見つからない。.venv/bin/python を準備するか --python /path/to/pypy3 を指定する。")
    if os.sep in path:
        path = str(Path(path).expanduser().absolute())
    probe = subprocess.run([path, "-c", "import sys; print(sys.implementation.name); print(sys.version)"],
                           text=True, capture_output=True, check=True)
    if probe.stdout.splitlines()[0] != "pypy":
        raise ValueError(f"指定された処理系は PyPy ではない: {path}")
    return path, probe.stdout.strip()


def comparison_key(inputs, config, python_info, jobs, timeout):
    digest = hashlib.sha256()
    for path in inputs:
        digest.update(path.name.encode() + b"\0" + path.read_bytes() + b"\0")
    tool_root = ROOT / "tools"
    for path in sorted((tool_root / "src").rglob("*.rs")):
        digest.update(str(path.relative_to(tool_root)).encode() + path.read_bytes())
    for name in ("Cargo.toml", "Cargo.lock"):
        path = tool_root / name
        if path.exists():
            digest.update(name.encode() + path.read_bytes())
    digest.update(json.dumps([config, python_info, jobs, timeout], sort_keys=True).encode())
    return digest.hexdigest()[:20]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", nargs="?", default="main.py", help="評価する単体提出ファイル")
    parser.add_argument("--quick", action="store_true", help="先頭10ケースだけ評価する")
    parser.add_argument("--case", help="1ケースだけ評価する（例: 0000、0000.txt）")
    parser.add_argument("-j", "--jobs", type=int, default=25, help="並列数（既定25、時間の最終確認は1）")
    parser.add_argument("--timeout", type=float, help="1ケースの強制終了秒数（既定10秒、公式TLとは別）")
    parser.add_argument("--python", help="PyPy の実行ファイル。通常は指定不要")
    args = parser.parse_args()
    config_file = ROOT / "eval_config.json"
    if not config_file.is_file():
        raise ValueError("公式ツールの準備が必要だ: python3 setup.py ~/Downloads/tools.zip")
    config = json.loads(config_file.read_text())
    timeout = args.timeout if args.timeout is not None else config["timeout"]
    if args.jobs < 1 or timeout <= 0:
        raise ValueError("並列数と時間は正の値を指定する。")
    source = Path(args.source).expanduser()
    if not source.is_absolute():
        source = ROOT / source
    source = source.resolve()
    if not source.is_file():
        raise ValueError(f"提出ファイルがない: {source}")
    # 雛形の未記入箇所を、全ケースで同じ例外が出る前に一覧で示す。
    try:
        tree = ast.parse(source.read_text())
    except SyntaxError as error:
        raise ValueError(f"構文エラー: {error}") from error
    todos = []
    for node in ast.walk(tree):
        if not isinstance(node, ast.Raise) or not isinstance(node.exc, ast.Call):
            continue
        call = node.exc
        if (isinstance(call.func, ast.Name) and call.func.id == "NotImplementedError" and call.args
                and isinstance(call.args[0], ast.Constant) and isinstance(call.args[0].value, str)
                and call.args[0].value.startswith("TODO:")):
            todos.append(call.args[0].value)
    if todos:
        raise ValueError("main.py に未記入の関数がある。\n  " + "\n  ".join(todos))
    inputs = sorted((ROOT / "tools/in").glob("*.txt"))
    if args.case:
        inputs = [p for p in inputs if p.stem == Path(args.case).stem]
    elif args.quick:
        inputs = inputs[:10]
    if not inputs:
        raise ValueError("対象の入力がない。tools/in と --case を確認する。")
    tool = ROOT / "tools/target/release" / config["mode"]
    if not tool.is_file():
        raise ValueError("公式採点器がない: python3 setup.py を実行する。")
    python, python_info = find_pypy(args.python)
    check = subprocess.run([python, "-m", "py_compile", str(source)], text=True, capture_output=True)
    if check.returncode:
        raise ValueError("構文エラー:\n" + check.stderr)
    key = comparison_key(inputs, config, python_info, args.jobs, timeout)
    results_dir = ROOT / "results"
    results_dir.mkdir(exist_ok=True)
    run_id = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    run_dir = results_dir / run_id
    run_dir.mkdir()
    shutil.copyfile(source, run_dir / "main.py")
    meta = {"source": str(source), "python": python, "python_version": python_info,
            "config": config, "timeout": timeout, "jobs": args.jobs,
            "inputs": [p.name for p in inputs], "comparison_key": key}
    (run_dir / "settings.json").write_text(json.dumps(meta, ensure_ascii=False, indent=2) + "\n")
    print(f"PyPy / {config['mode']} / {len(inputs)}ケース / 並列{args.jobs}")
    print(f"保存先: {run_dir}", flush=True)
    rows = []
    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(evaluate_one, p, run_dir, python, config, timeout): p for p in inputs}
        for future in as_completed(futures):
            row = future.result()
            rows.append(row)
            detail = f"score={row['score']}" if row["score"] else "得点なし"
            print(f"[{len(rows):3}/{len(inputs)}] {row['case']} {row['status']} {detail} {row['seconds']}秒", flush=True)
            if row["status"] != "OK":
                case_dir = run_dir / "cases" / Path(row["case"]).stem
                print(f"  確認先: {case_dir}", flush=True)
                log = case_dir / ("scorer.log" if row["status"] == "INVALID" else "solver.log")
                if log.exists():
                    print("  " + "\n  ".join(log.read_text(errors="replace").splitlines()[-8:]), flush=True)
    rows.sort(key=lambda row: row["case"])
    with (run_dir / "scores.csv").open("w", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=["case", "status", "score", "seconds"])
        writer.writeheader()
        writer.writerows(rows)
    failed = sum(row["status"] != "OK" for row in rows)
    summary = {field: "" for field in SUMMARY_FIELDS}
    summary.update(run=run_id, source=source.name, cases=len(rows), failed=failed,
                   max_seconds=max(Decimal(row["seconds"]) for row in rows),
                   mode=config["mode"], jobs=args.jobs, comparison_key=key)
    if not failed:
        scores = [Decimal(row["score"]) for row in rows]
        total = sum(scores)
        summary.update(average=total / len(scores), total=total, minimum=min(scores), maximum=max(scores))
    summary_path = results_dir / "summary.csv"
    with summary_path.open("a+", newline="") as file:
        fcntl.flock(file, fcntl.LOCK_EX)
        file.seek(0)
        previous = [row for row in csv.DictReader(file)
                    if row["comparison_key"] == key and row["failed"] == "0"]
        file.seek(0, os.SEEK_END)
        writer = csv.DictWriter(file, fieldnames=SUMMARY_FIELDS)
        if file.tell() == 0:
            writer.writeheader()
        writer.writerow(summary)
    print(f"\n完了: 成功={len(rows) - failed}, 失敗={failed}, 最大時間={summary['max_seconds']}秒")
    if not failed:
        print(f"平均={summary['average']:.3f}, 合計={summary['total']}, 最小={summary['minimum']}, 最大={summary['maximum']}")
        if previous:
            difference = summary["average"] - Decimal(previous[-1]["average"])
            print(f"同条件の前回との差={difference:+.3f}（符号は生の得点差）")
    else:
        print("失敗を含むため平均と合計は確定しない。ケース別の得点とログを確認する。")
    print(f"一覧: {summary_path}\n提出コード: {run_dir / 'main.py'}\nケース別: {run_dir / 'scores.csv'}")
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"評価失敗: {error}", file=sys.stderr)
        sys.exit(1)
