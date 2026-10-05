#!/usr/bin/env python3
"""Generate the evaluated standalone v303 source; never run or register a solver."""
from pathlib import Path
import argparse, hashlib, subprocess, tempfile

PARENT_SHA256 = '4be4bd07bee152643fe4620f50573764a5a74167a81d5f9c280f0655789feaed'
SOURCE_SHA256 = 'c8fee70e2245c03b3cd25a838bb0f51d9d83e3e48717d900d9963322f3fa08f4'

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    root = parser.parse_args().root.resolve()
    parent = root / 'src/bin/v210_collection_color_quotient.cpp'
    patch = root / 'adhoc/v303/v303_vs_v210.patch'
    target = root / 'src/bin/v303_palindromic_transport.cpp'
    if hashlib.sha256(parent.read_bytes()).hexdigest() != PARENT_SHA256:
        raise SystemExit('Parent does not match the evaluated v210; refusing to apply.')
    with tempfile.TemporaryDirectory(prefix='v303-') as directory:
        output = Path(directory) / 'v303.cpp'
        subprocess.run(['patch', '--batch', '--silent', '--fuzz=0', '--output', str(output),
                        str(parent), str(patch)], check=True)
        data = output.read_bytes()
    if hashlib.sha256(data).hexdigest() != SOURCE_SHA256:
        raise SystemExit('Generated source hash mismatch.')
    if target.exists() and target.read_bytes() != data:
        raise SystemExit('Existing v303 differs; refusing to overwrite.')
    target.write_bytes(data)
    print(target)
    print('SHA256', SOURCE_SHA256)

if __name__ == '__main__':
    main()
