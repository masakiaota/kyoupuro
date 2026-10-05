#!/usr/bin/env python3
"""Run each source once per selected input with the diagnostic clock."""
from pathlib import Path
import json
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
out = root / "adhoc/v015_port_audit"
env = dict(os.environ, AHC072_LOG="1")
records = []
# Preselected inputs: the handmade case and the first four generated cases.
for case in range(5):
    name = f"{case:04d}"
    pair = {}
    for variant in ("original", "ported"):
        with (root / f"tools/in/{name}.txt").open("rb") as stdin:
            result = subprocess.run(
                ["/usr/bin/time", "-l", str(out / f"{variant}_fixed_clock")],
                stdin=stdin, capture_output=True, env=env, check=True,
            )
        (out / f"{name}.{variant}.out").write_bytes(result.stdout)
        (out / f"{name}.{variant}.err").write_bytes(result.stderr)
        stderr = result.stderr.decode()
        audit = next(line for line in stderr.splitlines() if line.startswith("[audit]"))
        counters = dict(re.findall(r"([a-z_]+)=(\d+)", audit))
        rss = int(re.search(r"(\d+)\s+maximum resident set size", stderr)[1])
        pair[variant] = {"operations": len(result.stdout.splitlines()),
                         "audit": counters, "max_rss_bytes": rss}
    pair["same_output"] = (out / f"{name}.original.out").read_bytes() == (out / f"{name}.ported.out").read_bytes()
    pair["same_audit"] = pair["original"]["audit"] == pair["ported"]["audit"]
    record = {"case": name, **pair}
    records.append(record)
    print(json.dumps(record), flush=True)
(out / "fixed_clock_results.json").write_text(json.dumps(records, indent=2) + "\n")
