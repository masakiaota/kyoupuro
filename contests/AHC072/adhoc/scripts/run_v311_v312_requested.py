#!/usr/bin/env python3
"""指示された評価を1段階だけ実行し、終了状態と出力を保存する。"""
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

root = Path(__file__).resolve().parents[2]
run = Path(sys.argv[1]).resolve()
stage = sys.argv[2]
manifest = json.loads((run / "manifest.json").read_text())
config = manifest["stages"][stage]
state_path = run / f"{stage}_state.json"
if state_path.exists():
    raise SystemExit("state already exists; do not duplicate evaluation")
assert manifest.get("first_push_commit"), "initial push must finish before evaluation"
previous = {"v312_in": "v311_in", "v312_validation1": "v312_in"}.get(stage)
if previous:
    reported = json.loads((run / f"{previous}_reported.json").read_text())
    assert time.time() >= reported["epoch"] + manifest["cooldown_seconds"], "cooldown not finished"
state = {"status": "running", "pid": os.getpid(), "stage": stage,
         "started_at": datetime.datetime.now().astimezone().isoformat(), "label": config["label"]}

def save_state():
    temporary = state_path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n")
    temporary.replace(state_path)

save_state()
code = 1
try:
    source = root / "src/bin" / f"{config['bin']}.cpp"
    assert hashlib.sha256(source.read_bytes()).hexdigest() == config["source_sha256"]
    with (run / f"{stage}.log").open("w") as log:
        code = subprocess.call([sys.executable, "scripts/eval.py", config["bin"],
                                config["input_dir"], "-j", str(config["jobs"]), "--wait-lock",
                                "--label", config["label"]], cwd=root, stdin=subprocess.DEVNULL,
                               stdout=log, stderr=subprocess.STDOUT)
    if code == 0:
        shutil.copytree(root / "results/out" / config["bin"], run / f"{stage}_outputs")
except Exception as error:
    code = 1
    state["error"] = str(error)
    raise
finally:
    state.update(status="completed" if code == 0 else "failed", exit_code=code,
                 finished_at=datetime.datetime.now().astimezone().isoformat())
    save_state()
raise SystemExit(code)
