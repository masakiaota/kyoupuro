#!/usr/bin/env python3
"""指示された評価を1段階だけ実行し、重複起動を拒否する。"""
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
assert manifest.get("first_push_commit"), "initial push must finish before evaluation"
assert json.loads((run / "import_check/result.json").read_text())["solver_executions"] == 0
if config["previous_stage"]:
    reported = json.loads((run / f"{config['previous_stage']}_reported.json").read_text())
    assert time.time() >= reported["epoch"] + manifest["cooldown_seconds"], "cooldown not finished"
    if config.get("require_previous_push"):
        pushed = json.loads((run / f"{config['previous_stage']}_pushed.json").read_text())
        assert pushed["commit"] == pushed["verified_remote_sha"], "previous stage push must finish"
elif manifest.get("prerequisite_run"):
    prior = Path(manifest["prerequisite_run"])
    prior_manifest = json.loads((prior / "manifest.json").read_text())
    assert prior_manifest.get("final_push_commit"), "previous pipeline push must finish"
    assert prior_manifest["final_push_commit"] == prior_manifest.get("final_push_verified_remote_sha")
    reported = json.loads((prior / f"{manifest['prerequisite_stage']}_reported.json").read_text())
    assert time.time() >= reported["epoch"] + manifest["cooldown_after_prerequisite_report_seconds"], "prior cooldown not finished"
# 排他的作成により、同じ段階を二重に開始しない。
state = {"status": "running", "pid": os.getpid(), "stage": stage,
         "started_at": datetime.datetime.now().astimezone().isoformat(), "label": config["label"]}
with state_path.open("x") as f:
    json.dump(state, f, ensure_ascii=False, indent=2)
(run / f"{stage}_pid.txt").write_text(str(os.getpid()) + "\n")

def save_state():
    temporary = state_path.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n")
    temporary.replace(state_path)

code = 1
try:
    source = root / "src/bin" / f"{config['bin']}.cpp"
    assert hashlib.sha256(source.read_bytes()).hexdigest() == config["source_sha256"]
    dataset = config["input_dir"].split("/")[-1]
    files = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
             for p in (root / config["input_dir"]).glob("*.txt")}
    assert files == manifest["input_sha256"][dataset], "input set changed"
    with (run / f"{stage}.log").open("w") as log:
        code = subprocess.call([sys.executable, "scripts/eval.py", config["bin"],
                                config["input_dir"], "-j", str(config["jobs"]), "--no-local", "--wait-lock",
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
