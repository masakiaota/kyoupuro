#!/usr/bin/env python3
"""移植のビルドと完全前処理を照合する。solverは実行しない。"""
import difflib
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
for version in ("v313", "v401"):
    manifest = json.loads((root / f"adhoc/imports/{version}/manifest.json").read_text())
    source = root / manifest["source"]
    original = Path(manifest["original_name"]) if version == "v313" else root / "adhoc/imports/v401/original.cpp.txt"
    assert hashlib.sha256(source.read_bytes()).hexdigest() == manifest["source_sha256"]
    assert hashlib.sha256(original.read_bytes()).hexdigest() == manifest["original_sha256"]
    entry = {"source": manifest["source"], "sha256": manifest["source_sha256"], "modes": []}
    for mode, macros in (("local", ["-DLOCAL"]),
                         ("judge", ["-DATCODER", "-DONLINE_JUDGE", "-DNOMINMAX"])):
        prefix = out / f"{source.stem}_{mode}"
        with prefix.with_suffix(".build.log").open("wb") as log:
            subprocess.run([cxx, *flags, *macros, str(source), "-o", str(prefix)],
                           env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        expanded = []
        for label, path in (("original", original), ("placed", source)):
            text = subprocess.check_output([cxx, *flags, *macros, "-x", "c++", "-E", "-P", str(path)], env=env)
            text = text.replace(str(path).encode(), b"SOURCE.cpp")
            text = text.replace(('"' + path.name + '"').encode(), b'"SOURCE.cpp"')
            (out / f"{source.stem}_{mode}_{label}.ii").write_bytes(text)
            expanded.append(text)
        expected = expanded[0]
        if version == "v401" and mode == "local":
            old = b"constexpr double LOCAL_TIME_RATIO = 1.90 / 1.930;"
            assert expected.count(old) == 1
            expected = expected.replace(old, b"constexpr double LOCAL_TIME_RATIO = 0.80;")
        diff = "".join(difflib.unified_diff(expanded[0].decode().splitlines(True),
                                         expanded[1].decode().splitlines(True),
                                         fromfile="original", tofile="placed"))
        prefix.with_suffix(".diff").write_text(diff)
        assert expected == expanded[1], (source.name, mode, "unexpected preprocessing difference")
        entry["modes"].append({"mode": mode, "full_preprocessing_equal": expanded[0] == expanded[1],
                               "only_expected_local_ratio_difference": expected == expanded[1],
                               "normalized_difference": "source path and basename in diagnostic literals"})
        print(source.name, mode, "build and preprocessing passed", flush=True)
    result["files"].append(entry)
(out / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
