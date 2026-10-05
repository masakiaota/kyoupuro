#!/usr/bin/env python3
"""専用プロセス群のメモリを監視する。閾値超過・監視失敗時は群ごと終了する。"""
import argparse
import ctypes
from datetime import datetime, timezone
import errno
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time


def save(path, value):
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(value, indent=2) + "\n")
    temp.replace(path)


def now():
    return datetime.now(timezone.utc).isoformat()


def gpu_memory(path, processes, group):
    if path is None:
        return 0
    state = json.loads(path.read_text())
    if not state["active"]:
        return 0
    pid = int(state["pid"])
    if pid not in {p["pid"] for p in processes}:
        try:
            actual_group = os.getpgid(pid)
        except ProcessLookupError:
            return 0
        # ps取得とGPU報告の間に起動した子も、同じ群なら次の標本でCPU側を計測する。
        if actual_group != group:
            raise RuntimeError("GPU reporter is outside the monitored process group")
    if time.time() - state["updated_unix"] > 30:
        raise RuntimeError("GPU memory report is stale")
    value = int(state["driver_bytes"])
    if value < 0:
        raise ValueError("negative GPU memory report")
    return value


class Memory:
    def __init__(self):
        self.library = None
        if sys.platform == "darwin":
            self.library = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
            self.library.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]
            self.library.proc_pid_rusage.restype = ctypes.c_int

    def footprint(self, pid, rss):
        if self.library is None:
            return rss
        # SDKのrusage_info_v4: UUID 16 bytes、その後uint64_tが35個。
        buffer = (ctypes.c_uint64 * 37)()
        result = self.library.proc_pid_rusage(pid, 4, ctypes.byref(buffer))
        if result:
            code = ctypes.get_errno()
            if code in (errno.ESRCH, errno.ENOENT):
                return 0
            raise OSError(code, f"proc_pid_rusage({pid}) failed")
        return max(rss, int(buffer[2 + 6]), int(buffer[2 + 7]))

    def sample(self, group):
        listing = subprocess.check_output(["ps", "-axo", "pid=,pgid=,rss="], text=True)
        processes = []
        for line in listing.splitlines():
            pid, pgid, rss = map(int, line.split())
            if pgid == group or pid == os.getpid():
                value = self.footprint(pid, rss * 1024)
                processes.append({"pid": pid, "bytes": value})
        if not any(p["pid"] == os.getpid() and p["bytes"] > 0 for p in processes):
            raise RuntimeError("memory guard could not measure itself")
        return sum(p["bytes"] for p in processes), processes


def stop(group):
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(group, sig)
        except OSError as error:
            # macOSでは終了済みの子だけが残る群へのkillpgがEPERMになる。
            # 生存プロセスがある場合の権限エラーは隠さない。
            listing = subprocess.check_output(["ps", "-axo", "pgid=,stat="], text=True)
            alive = any(int(row.split()[0]) == group and "Z" not in row.split()[1]
                        for row in listing.splitlines())
            if error.errno not in (errno.ESRCH, errno.EPERM) or alive:
                raise
            return
        if sig == signal.SIGTERM:
            time.sleep(.25)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--log-dir", type=Path, required=True)
    parser.add_argument("--stop-gb", type=float, default=56)
    parser.add_argument("--limit-gb", type=float, default=64)
    parser.add_argument("--seconds", type=int, default=3600)
    parser.add_argument("--gpu-state", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command or not 0 < args.stop_gb < args.limit_gb <= 64:
        parser.error("a command and 0 < stop-gb < limit-gb <= 64 are required")
    args.log_dir.mkdir(parents=True, exist_ok=False)
    memory = Memory()
    memory.sample(-1)  # 監視手段が動かない環境では、計算を起動しない。
    if args.gpu_state is not None:
        state = json.loads(args.gpu_state.read_text())
        if state["active"]:
            raise RuntimeError("GPU state must be inactive before launch")
    process = subprocess.Popen(command, stdin=subprocess.DEVNULL, start_new_session=True)
    started = time.monotonic()
    peak = samples = 0
    reason, code = "child_exit", None
    def interrupted(signum, frame):
        raise InterruptedError(f"signal {signum}")
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    save(args.log_dir / "launch.json", {"guard_pid": os.getpid(), "group": process.pid,
         "command": command, "started_at": now(), "stop_bytes": int(args.stop_gb * 1e9),
         "limit_bytes": int(args.limit_gb * 1e9), "interval_sec": .1,
         "gpu_used": args.gpu_state is not None, "gpu_state": str(args.gpu_state) if args.gpu_state else None})
    try:
        with (args.log_dir / "samples.jsonl").open("x") as log:
            while True:
                cpu, processes = memory.sample(process.pid)
                gpu = gpu_memory(args.gpu_state, processes, process.pid)
                # Appleの共有メモリは二重計上し得るが、過少に見積もらない。
                used = cpu + gpu
                samples += 1
                peak = max(peak, used)
                row = {"time": now(), "bytes": used, "cpu_bytes": cpu, "gpu_bytes": gpu,
                       "peak_bytes": peak, "processes": processes}
                if samples % 10 == 1:
                    log.write(json.dumps(row) + "\n");log.flush()
                    save(args.log_dir / "status.json", row)
                if used >= args.stop_gb * 1e9:
                    reason, code = "memory_stop", 73
                    log.write(json.dumps(row) + "\n");log.flush()
                    stop(process.pid)
                    break
                if time.monotonic() - started >= args.seconds:
                    reason, code = "time_limit", 75
                    stop(process.pid)
                    break
                result = process.poll()
                if result is not None:
                    code = result if result >= 0 else 128 - result
                    break
                time.sleep(.1)
    except BaseException as error:
        reason, code = "monitor_error: " + str(error), 74
        stop(process.pid)
    finally:
        process.wait()
        save(args.log_dir / "exit.json", {"reason": reason, "exit_code": code, "child_exit_code": process.returncode,
             "peak_observed_bytes": peak, "samples": samples, "finished_at": now(),
             "elapsed_sec": time.monotonic() - started})
    return code


if __name__ == "__main__":
    sys.exit(main())
