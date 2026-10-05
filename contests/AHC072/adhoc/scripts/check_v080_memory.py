#!/usr/bin/env python3
"""実験開始前にCPU閾値、GPU報告の加算、報告停止の検出を確認する。"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def save(path, value):
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(value))
    temp.replace(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--child", choices=("cpu", "gpu", "stale"))
    args = parser.parse_args()
    if args.child:
        if args.child == "cpu":
            memory = bytearray(400_000_000)
            assert len(memory) == 400_000_000
        else:
            save(args.output / "gpu.json", {"active": True, "pid": os.getpid(),
                 "driver_bytes": 800_000_000 if args.child == "gpu" else 0,
                 "updated_unix": time.time() - (31 if args.child == "stale" else 0)})
        time.sleep(20)
        return
    args.output.mkdir(parents=True, exist_ok=False)
    checks = {}
    for mode in ("cpu", "gpu", "stale"):
        directory = args.output / mode
        directory.mkdir()
        save(directory / "gpu.json", {"active": False, "driver_bytes": 0, "updated_unix": time.time()})
        command = [sys.executable, str(Path(__file__).with_name("memory_guard.py")),
                   "--log-dir", str(directory / "guard"), "--stop-gb", ".2" if mode == "cpu" else ".5",
                   "--limit-gb", "1", "--seconds", "10"]
        if mode != "cpu":
            command += ["--gpu-state", str(directory / "gpu.json")]
        command += ["--", sys.executable, str(Path(__file__).resolve()), "--output", str(directory), "--child", mode]
        result = subprocess.run(command, timeout=15, capture_output=True, text=True)
        state = json.loads((directory / "guard/exit.json").read_text())
        assert result.returncode == (74 if mode == "stale" else 73), (mode, result.stderr, state)
        assert state["reason"] == ("monitor_error: GPU memory report is stale" if mode == "stale" else "memory_stop"), state
        if mode == "gpu":
            samples = [json.loads(line) for line in (directory / "guard/samples.jsonl").read_text().splitlines()]
            assert any(r["gpu_bytes"] == 800_000_000 and r["cpu_bytes"] < 500_000_000 for r in samples)
        checks[mode] = state
    save(args.output / "passed.json", {"passed": True, "checks": checks})
    print(json.dumps({"passed": True, "checks": list(checks)}))


if __name__ == "__main__":
    main()
