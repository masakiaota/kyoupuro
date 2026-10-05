#!/usr/bin/env python3
"""Reconstruct the exact evaluated standalone v306. Does not run or register eval."""
from pathlib import Path
import hashlib
import re

ROOT = Path(__file__).resolve().parents[2]
PARENT_SHA = '4be4bd07bee152643fe4620f50573764a5a74167a81d5f9c280f0655789feaed'
RESULT_SHA = 'dcccc76ad4f6985678f697d9d83d8be2975b88535b14d8bd3b515e030115cd61'

def build(root: Path) -> Path:
    parent = root / 'src/bin/v210_collection_color_quotient.cpp'
    data = parent.read_bytes()
    if hashlib.sha256(data).hexdigest() != PARENT_SHA:
        raise RuntimeError('Parent v210 hash differs; refusing to apply the patch')
    source = data.decode('utf-8').splitlines(keepends=True)
    patch = (root / 'adhoc/v306/v306_vs_v210.patch').read_text().splitlines(keepends=True)
    output: list[str] = []
    cursor = 0
    i = 2
    while i < len(patch):
        match = re.fullmatch(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@\n?', patch[i])
        if match is None:
            raise RuntimeError(f'Invalid hunk header: {patch[i]!r}')
        old_count = int(match[2] or 1)
        new_count = int(match[4] or 1)
        start = int(match[1]) - (old_count != 0)
        if start < cursor:
            raise RuntimeError('Overlapping patch hunks')
        output.extend(source[cursor:start])
        i += 1
        consumed: list[str] = []
        added: list[str] = []
        while i < len(patch) and not patch[i].startswith('@@ '):
            line = patch[i]
            if line.startswith('-'):
                consumed.append(line[1:])
            elif line.startswith('+'):
                added.append(line[1:])
            elif line.startswith(' '):
                consumed.append(line[1:])
                added.append(line[1:])
            else:
                raise RuntimeError(f'Invalid patch line: {line!r}')
            i += 1
        if len(consumed) != old_count or len(added) != new_count:
            raise RuntimeError('Hunk line count mismatch')
        if source[start:start + old_count] != consumed:
            raise RuntimeError('Hunk source mismatch')
        output.extend(added)
        cursor = start + old_count
    output.extend(source[cursor:])
    result = ''.join(output).encode('utf-8')
    if hashlib.sha256(result).hexdigest() != RESULT_SHA:
        raise RuntimeError('Reconstructed v306 hash differs')
    target = root / 'src/bin/v306_anonymous_extraction.cpp'
    if target.exists() and target.read_bytes() != result:
        raise RuntimeError('Refusing to overwrite a different v306 file')
    target.write_bytes(result)
    return target

if __name__ == '__main__':
    print(build(ROOT))
