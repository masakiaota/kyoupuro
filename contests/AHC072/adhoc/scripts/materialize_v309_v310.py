#!/usr/bin/env python3
"""Reconstruct the exact evaluated v309/v310 standalone C++ files.
The payload contains only line edits against v210, not models or test answers.
This script does not execute solvers, train models, or register evaluations.
"""
from pathlib import Path
import base64
import difflib
import hashlib
import json
import zlib

ROOT = Path(__file__).resolve().parents[2]
PARENT_SHA256 = '4be4bd07bee152643fe4620f50573764a5a74167a81d5f9c280f0655789feaed'
PAYLOAD = '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'


def main() -> None:
    parent_path = ROOT / 'src/bin/v210_collection_color_quotient.cpp'
    data = parent_path.read_bytes()
    if hashlib.sha256(data).hexdigest() != PARENT_SHA256:
        raise SystemExit('Parent v210 hash mismatch; refusing to modify files.')
    source = data.decode('utf-8').splitlines(keepends=True)
    specs = json.loads(zlib.decompress(base64.b64decode(PAYLOAD)))
    prepared = []
    for name, spec in specs.items():
        lines = source.copy()
        for begin, end, replacement in reversed(spec['edits']):
            lines[begin:end] = replacement
        result = ''.join(lines).encode('utf-8')
        if hashlib.sha256(result).hexdigest() != spec['sha256']:
            raise SystemExit(f'{name}: generated source hash mismatch')
        target = ROOT / 'src/bin' / (name + '.cpp')
        if target.exists() and target.read_bytes() != result:
            raise SystemExit(f'{target}: refusing to overwrite a different file')
        prepared.append((name, target, result, lines))
    for name, target, result, lines in prepared:
        target.write_bytes(result)
        patch = ''.join(difflib.unified_diff(source, lines,
            fromfile='a/src/bin/v210_collection_color_quotient.cpp',
            tofile='b/src/bin/' + name + '.cpp'))
        patch_path = ROOT / 'adhoc/v309' / (name[:4] + '_vs_v210.patch')
        patch_path.parent.mkdir(parents=True, exist_ok=True)
        patch_path.write_text(patch, encoding='utf-8')
        print(target.relative_to(ROOT), hashlib.sha256(result).hexdigest())


if __name__ == '__main__':
    main()
