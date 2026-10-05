#!/usr/bin/env python3
"""v105の追加validation評価を実行し、再開用の状態を残す。"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
run_dir = Path(sys.argv[1]).resolve()
manifest = json.loads((run_dir / "manifest.json").read_text())
source = root / "src/bin/v105_nn_lns.cpp"
assert hashlib.sha256(source.read_bytes()).hexdigest() == manifest["source_sha256"]
state_path = run_dir / "validation_state.json"
if state_path.exists():
    raise SystemExit("validation_state.json already exists; do not duplicate evaluation")
state = {"status": "running", "pid": os.getpid(),
         "started_at": datetime.datetime.now().astimezone().isoformat(),
         "label": manifest["validation_label"]}

def save_state():
    temporary = state_path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n")
    temporary.replace(state_path)

save_state()
code = 1
try:
    with (run_dir / "validation.log").open("w") as log:
        code = subprocess.call(
            [sys.executable, "scripts/eval.py", "v105_nn_lns", "tools/validation1",
             "-j", "1", "--wait-lock", "--label", manifest["validation_label"]],
            cwd=root, stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT)
finally:
    state.update(status="completed" if code == 0 else "failed", exit_code=code,
                 finished_at=datetime.datetime.now().astimezone().isoformat())
    save_state()
raise SystemExit(code)
