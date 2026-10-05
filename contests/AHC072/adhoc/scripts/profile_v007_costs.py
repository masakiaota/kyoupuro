#!/usr/bin/env python3
"""既存v006/v007を同条件でサンプリングする。solver変更・採点・eval追記は行わない。"""

import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

from analyze_approach_outputs import trace


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v007_cost_profile"
BINS = ["v006_branch_reconnect", "v007_incremental"]
CASES = ["0001.txt", "0019.txt"]


def main():
    OUT.mkdir(exist_ok=True)
    records = []
    with (ROOT / "results/.eval.lock").open("a+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        for bin_name in BINS:
            subprocess.run([str(ROOT / "scripts/build_solver.sh"), bin_name], cwd=ROOT, check=True)
        for case in CASES:
            for bin_name in BINS:
                stem = OUT / (bin_name + "_" + Path(case).stem)
                source = ROOT / "src/bin" / (bin_name + ".cpp")
                log_path = stem.with_suffix(".err")
                profile_path = stem.with_suffix(".sample.txt")
                with (ROOT / "tools/in" / case).open() as input_file, stem.with_suffix(".out").open("w") as output_file, log_path.open("w") as log_file:
                    start = time.perf_counter()
                    solver = subprocess.Popen([str(ROOT / "target/release" / bin_name)], stdin=input_file, stdout=output_file, stderr=log_file, cwd=ROOT)
                    sampler = subprocess.Popen(["/usr/bin/sample", str(solver.pid), "2", "1", "-mayDie", "-file", str(profile_path)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                    _, status, usage = os.wait4(solver.pid, 0)
                    elapsed = time.perf_counter() - start
                    solver.returncode = os.waitstatus_to_exitcode(status)
                    sample_out, sample_err = sampler.communicate()
                stem.with_suffix(".sample.log").write_text(sample_out + sample_err)
                if solver.returncode or sampler.returncode:
                    raise RuntimeError(f"{bin_name} {case}: solver={solver.returncode}, sampler={sampler.returncode}; {sample_err}")
                record = {
                    "bin": bin_name, "case": case,
                    "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                    "wall_seconds": elapsed,
                    "user_seconds": usage.ru_utime,
                    "system_seconds": usage.ru_stime,
                    "voluntary_switches": usage.ru_nvcsw,
                    "involuntary_switches": usage.ru_nivcsw,
                    "profile": str(profile_path.relative_to(ROOT)),
                    "trace": trace(log_path),
                }
                records.append(record)
                print(json.dumps({key: value for key, value in record.items() if key != "trace"}, ensure_ascii=False), flush=True)
    (OUT / "records.json").write_text(json.dumps(records, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
