#!/usr/bin/env python3
"""固定したGitソースと両モードの完全前処理を照合する。solverは実行しない。"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
run = Path(sys.argv[1]).resolve()
manifest = json.loads((run / "manifest.json").read_text())
out = run / "import_check"
out.mkdir(exist_ok=True)
original_dir = out / "original"
original_dir.mkdir(exist_ok=True)
env = os.environ.copy()
if sys.platform == "darwin":
    env.setdefault("SDKROOT", subprocess.check_output(["xcrun", "--show-sdk-path"], text=True).strip())
    env.setdefault("MACOSX_DEPLOYMENT_TARGET", "15.0")
cxx = shutil.which(env.get("CXX", "g++-15"))
assert cxx
flags = ["-std=gnu++23", "-O2", "-Wall", "-Wextra", "-march=native", "-pthread",
         "-ftrivial-auto-var-init=zero", "-fopenmp"]
result = {"compiler": subprocess.check_output([cxx, "--version"], text=True).splitlines()[0],
          "source_commit": manifest["source_commit"], "solver_executions": 0, "files": []}
for version in ("v314", "v315"):
    config = manifest["stages"][version + "_in"]
    path = f"src/bin/{config['bin']}.cpp"
    source = root / path
    original = original_dir / source.name
    original.write_bytes(subprocess.check_output(["git", "show", f"{manifest['source_commit']}:{path}"], cwd=root))
    assert source.read_bytes() == original.read_bytes()
    assert hashlib.sha256(source.read_bytes()).hexdigest() == config["source_sha256"]
    entry = {"source": path, "sha256": config["source_sha256"], "modes": []}
    for mode, macros in (("local", ["-DLOCAL"]),
                         ("judge", ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"])):
        prefix = out / f"{source.stem}_{mode}"
        with prefix.with_suffix(".build.log").open("wb") as log:
            subprocess.run([cxx, *flags, *macros, str(source), "-o", str(prefix)],
                           env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        expanded = []
        for label, p in (("original", original), ("placed", source)):
            text = subprocess.check_output([cxx, *flags, *macros, "-E", "-P", str(p)], env=env)
            text = text.replace(str(p).encode(), b"SOURCE.cpp")
            text = text.replace(('"' + p.name + '"').encode(), b'"SOURCE.cpp"')
            (out / f"{source.stem}_{mode}_{label}.ii").write_bytes(text)
            expanded.append(text)
        assert expanded[0] == expanded[1], (version, mode)
        entry["modes"].append({"mode": mode, "full_preprocessing_equal": True,
                               "normalized_difference": "source path and basename in diagnostic literals"})
        print(version, mode, "build and full preprocessing passed", flush=True)
    result["files"].append(entry)
(out / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
