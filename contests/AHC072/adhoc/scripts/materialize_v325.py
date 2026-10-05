#!/usr/bin/env python3
"""Materialize an already frozen C++ file; never execute or evaluate a solver."""
from pathlib import Path
import hashlib
import re

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'src/bin/v136_exact_speed.cpp'
PATCH = ROOT / 'adhoc/v325/source_min.patch'
DEST = ROOT / 'src/bin/v325_exact_hotpath.cpp'
BASE_SHA = '56fca55348f129aee8055ee8272f507d6131662aef8eebd5838f2b18efab02fe'
PATCH_SHA = '59332c4ca3488e46fddfcb10f41c19d9500dc8d6d09f04fd2fc4770ec7b12409'
DEST_SHA = '1dce487ed293ba3e8cd44cf93ecddc8ed0e8b5cb3ac6f42d48ff15fd50769141'

def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()

def main() -> None:
    source = BASE.read_bytes()
    patch = PATCH.read_bytes()
    if sha(source) != BASE_SHA or sha(patch) != PATCH_SHA:
        raise RuntimeError('Frozen parent or patch checksum mismatch')
    lines = source.decode('utf-8').splitlines(keepends=True)
    diff = patch.decode('utf-8').splitlines(keepends=True)
    result = []
    cursor = 0
    index = 2
    while index < len(diff):
        match = re.fullmatch(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@\n', diff[index])
        if not match:
            raise RuntimeError(f'Invalid hunk header at {index + 1}')
        start, count = int(match[1]), int(match[2] or '1')
        expected_new = int(match[4] or '1')
        position = start - 1 if count else start
        if not cursor <= position <= len(lines):
            raise RuntimeError('Overlapping or out-of-range hunk')
        result.extend(lines[cursor:position])
        cursor = position
        consumed = added = 0
        index += 1
        while index < len(diff) and not diff[index].startswith('@@ '):
            entry = diff[index]
            if entry.startswith('-'):
                if cursor >= len(lines) or lines[cursor] != entry[1:]:
                    raise RuntimeError(f'Parent context differs at line {cursor + 1}')
                cursor += 1
                consumed += 1
            elif entry.startswith('+'):
                result.append(entry[1:])
                added += 1
            else:
                raise RuntimeError('Only exact zero-context hunks are permitted')
            index += 1
        if consumed != count or added != expected_new:
            raise RuntimeError('Hunk size mismatch')
    result.extend(lines[cursor:])
    data = ''.join(result).encode('utf-8')
    if sha(data) != DEST_SHA:
        raise RuntimeError('Materialized source checksum mismatch')
    if DEST.exists() and DEST.read_bytes() != data:
        raise RuntimeError('Refusing to overwrite a different source')
    DEST.write_bytes(data)
    print(f'{DEST.relative_to(ROOT)} {len(data)} bytes SHA256={DEST_SHA}')

if __name__ == '__main__':
    main()
