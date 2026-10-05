#!/usr/bin/env python3
"""Materialize the exact evaluated standalone v301 from the committed v210 and patch.

Usage (from any directory): python3 adhoc/scripts/materialize_v301.py
This script does not run a solver, register an evaluation, or modify v210.
Requires Python 3 and the standard `patch` executable.
"""
from __future__ import annotations
import hashlib
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
PARENT = ROOT / 'src/bin/v210_collection_color_quotient.cpp'
PATCH = ROOT / 'adhoc/v301/v301_vs_v210.patch'
OUTPUT = ROOT / 'src/bin/v301_delivery_order.cpp'
PARENT_SHA256 = '4be4bd07bee152643fe4620f50573764a5a74167a81d5f9c280f0655789feaed'
OUTPUT_SHA256 = '4e0fe019b62ec9da12c00abdd335a5dac59c865875285301e36c63fbd109da85'
PATCH_GIT_BLOB = '75e19ffabdfa5fd7767c4702988bbad07ad4f4f0'


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    parent = PARENT.read_bytes()
    delta = PATCH.read_bytes()
    if sha256(parent) != PARENT_SHA256:
        raise SystemExit('Parent differs from the pinned experiment. Use the experiment branch; do not apply the patch to a different parent.')
    blob = hashlib.sha1(b'blob ' + str(len(delta)).encode('ascii') + b'\0' + delta).hexdigest()
    if blob != PATCH_GIT_BLOB:
        raise SystemExit('Experiment patch hash mismatch.')
    patch_tool = shutil.which('patch')
    if patch_tool is None:
        raise SystemExit('The patch executable is required.')
    if OUTPUT.exists():
        if sha256(OUTPUT.read_bytes()) == OUTPUT_SHA256:
            print(f'Already materialized: {OUTPUT.relative_to(ROOT)}')
            return
        raise SystemExit(f'Refusing to replace an existing different file: {OUTPUT}')
    with tempfile.TemporaryDirectory(prefix='ahc072-v301-') as temporary:
        destination = Path(temporary) / 'v301_delivery_order.cpp'
        result = subprocess.run(
            [patch_tool, '--batch', '--output', str(destination), str(PARENT), str(PATCH)],
            capture_output=True, text=True, check=False,
        )
        if result.returncode != 0:
            raise SystemExit(result.stdout + result.stderr)
        content = destination.read_bytes()
        if sha256(content) != OUTPUT_SHA256:
            raise SystemExit('Generated solver does not match the evaluated source.')
        OUTPUT.write_bytes(content)
    print(f'Materialized: {OUTPUT.relative_to(ROOT)}')
    print(f'SHA256: {OUTPUT_SHA256}')


if __name__ == '__main__':
    main()
