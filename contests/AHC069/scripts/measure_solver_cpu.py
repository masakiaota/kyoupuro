#!/usr/bin/env python3
"""Run a solver with inherited stdio and record only the solver child's CPU time."""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--result-file", required=True)
    parser.add_argument("command")
    parser.add_argument("command_args", nargs=argparse.REMAINDER)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not hasattr(os, "fork") or not hasattr(os, "wait4"):
        print("measure_solver_cpu.py requires a Unix-like OS", file=sys.stderr)
        return 2

    pid = os.fork()
    if pid == 0:
        try:
            os.execvpe(args.command, [args.command, *args.command_args], os.environ)
        except OSError as error:
            print(f"failed to execute solver: {error}", file=sys.stderr)
            os._exit(127)

    while True:
        try:
            _, wait_status, usage = os.wait4(pid, 0)
            break
        except InterruptedError:
            continue

    cpu_elapsed_ns = round((usage.ru_utime + usage.ru_stime) * 1_000_000_000)
    if os.WIFEXITED(wait_status):
        exit_code = os.WEXITSTATUS(wait_status)
        term_signal = 0
    else:
        exit_code = -1
        term_signal = os.WTERMSIG(wait_status)

    Path(args.result_file).write_text(
        f"exit_code={exit_code}\n"
        f"term_signal={term_signal}\n"
        f"cpu_elapsed_ns={cpu_elapsed_ns}\n",
        encoding="utf-8",
    )

    if term_signal != 0:
        return 128 + term_signal
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
