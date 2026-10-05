#!/usr/bin/env python3
"""持ち込みソースの同一性と両ビルドを確認する。solverは実行しない。"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
run = Path(sys.argv[1]).resolve()
out = run / "import_check"
out.mkdir(exist_ok=True)
env = os.environ.copy()
if sys.platform == "darwin":
    env.setdefault("SDKROOT", subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip())
    env.setdefault("MACOSX_DEPLOYMENT_TARGET", "15.0")
cxx = shutil.which(env.get("CXX", "g++-15"))
assert cxx, "GCC 15 compiler not found"
flags = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
result = {"compiler": subprocess.check_output([cxx, "--version"], text=True).splitlines()[0],
          "solver_executions": 0, "files": []}
for v in ("v311", "v312"):
    manifest = json.loads((root / f"adhoc/imports/{v}/manifest.json").read_text())
    for item in manifest["files"]:
        if not item["original_name"].endswith(".cpp"):
            continue
        source = root / item["stored_path"]
        original = Path("/Users/masaki/Downloads") / item["original_name"]
        assert hashlib.sha256(source.read_bytes()).hexdigest() == item["sha256"]
        assert source.read_bytes() == original.read_bytes()
        entry = {"source": item["stored_path"], "sha256": item["sha256"], "modes": []}
        for mode, macros in (("local", ["-DLOCAL"]),
                             ("judge", ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"])):
            prefix = out / f"{source.stem}_{mode}"
            with prefix.with_suffix(".build.log").open("wb") as log:
                subprocess.run([cxx, *flags, *macros, str(source), "-o", str(prefix)],
                               env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
            expanded = []
            for path in (original, source):
                text = subprocess.check_output([cxx, *flags, *macros, "-E", "-P", str(path)], env=env)
                expanded.append(text.replace(str(path).encode(), b"SOURCE.cpp"))
            assert expanded[0] == expanded[1], (source.name, mode)
            entry["modes"].append({"mode": mode, "full_preprocessing_equal": True,
                                   "normalized_difference": "source path in diagnostic literals"})
            print(source.name, mode, "build and full preprocessing passed", flush=True)
        result["files"].append(entry)
(out / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
