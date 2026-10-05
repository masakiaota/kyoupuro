#!/usr/bin/env python3
"""Generate the exact standalone v302 source; never run solver/eval registration."""
from pathlib import Path
import hashlib,os,subprocess,tempfile
ROOT=Path(__file__).resolve().parents[2]
PARENT=ROOT/'src/bin/v210_collection_color_quotient.cpp'
PATCH=ROOT/'adhoc/v302/v302_vs_v210.patch'
OUTPUT=ROOT/'src/bin/v302_representative_gaps.cpp'
PARENT_SHA='4be4bd07bee152643fe4620f50573764a5a74167a81d5f9c280f0655789feaed'
OUTPUT_SHA='921fab3b645f180ce1840f2dc67c1837697be1883b555a97c78b4d199497d8d2'
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    if digest(PARENT)!=PARENT_SHA:raise SystemExit('Parent source differs from the evaluated v210')
    if OUTPUT.exists():
        if digest(OUTPUT)!=OUTPUT_SHA:raise SystemExit('Refusing to overwrite a different existing v302')
        print(OUTPUT);return
    with tempfile.TemporaryDirectory() as directory:
        target=Path(directory)/'v302.cpp'
        p=subprocess.run(['patch','--batch','--forward','--output',str(target),str(PARENT),str(PATCH)],capture_output=True,text=True)
        if p.returncode:raise SystemExit(p.stdout+p.stderr)
        if digest(target)!=OUTPUT_SHA:raise SystemExit('Generated source digest mismatch')
        OUTPUT.write_bytes(target.read_bytes())
    print(OUTPUT)
if __name__=='__main__':main()
