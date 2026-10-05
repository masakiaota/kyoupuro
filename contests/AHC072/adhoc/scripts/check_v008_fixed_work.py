#!/usr/bin/env python3
"""同一仕事量でv006/v008を1回ずつ比較する。solver変更やスコア蓄積はしない。"""

import fcntl
import hashlib
import json
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "adhoc/v008_fixed_work"
BINS = {"v006": "check_v008_reference", "v008": "check_v008_fixed_work"}
CHECK_KEYS = ["verified", "build_digest", "reconnect_digest", "random_state", "build_calls",
              "reconnect_attempts", "dp_calls", "dp_terminals", "dp_cells", "phase_bytes"]


def main():
    OUT.mkdir(exist_ok=True)
    cases = []
    with (ROOT / "results/.eval.lock").open("a+") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        for index, case in enumerate(["0001.txt", "0019.txt"]):
            row = {"case": case}
            # 固定仕事量を各1回実行する。順番は2件で入れ替える。
            order = ["v006", "v008"] if index == 0 else ["v008", "v006"]
            for version in order:
                with (ROOT / "tools/in" / case).open() as input_file:
                    result = subprocess.run([str(ROOT / "target/release" / BINS[version])], stdin=input_file,
                                            capture_output=True, text=True, cwd=ROOT, check=True)
                (OUT / (version + "_" + Path(case).stem + ".json")).write_text(result.stdout)
                row[version] = json.loads(result.stdout)
            for key in CHECK_KEYS:
                assert row["v006"][key] == row["v008"][key], (case, key, row)
            row["checks_equal"] = True
            cases.append(row)
    totals = {
        version: {key: sum(row[version][key] for row in cases)
                  for key in ["build_cpu_ms", "build_wall_ms", "reconnect_cpu_ms", "reconnect_wall_ms"]}
        for version in BINS
    }
    for total in totals.values():
        total["total_cpu_ms"] = total["build_cpu_ms"] + total["reconnect_cpu_ms"]
    summary = {
        "source_sha256": hashlib.sha256((ROOT / "src/bin/v008_lightweight.cpp").read_bytes()).hexdigest(),
        "reference_sha256": hashlib.sha256((ROOT / "adhoc/bin/v008_reference.cpp").read_bytes()).hexdigest(),
        "cases": cases,
        "totals": totals,
        "cpu_speedup": totals["v006"]["total_cpu_ms"] / totals["v008"]["total_cpu_ms"],
    }
    (OUT / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
