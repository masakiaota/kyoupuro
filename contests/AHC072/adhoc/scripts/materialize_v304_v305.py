#!/usr/bin/env python3
"""Recreate the two standalone, evaluated C++ files. Does not run or register them."""
from pathlib import Path
import hashlib,re
ROOT=Path(__file__).resolve().parents[2]
PARENT='4be4bd07bee152643fe4620f50573764a5a74167a81d5f9c280f0655789feaed'
TARGETS={
    'v304_open_construction':'18e996e11c5e7fa3584f2c04fd493701cadfb5110ce6fc2320b9942939c3c04e',
    'v305_mixed_construction':'10ae352ac937d371206b347274dbacb4580760d8d08539d5496e1a786157d8cf',
}
def digest(text):return hashlib.sha256(text.encode()).hexdigest()
def apply(original,patch):
    src=original.splitlines(True);lines=patch.splitlines(True);out=[];at=0;i=0
    while i<len(lines):
        if not lines[i].startswith('@@ '):i+=1;continue
        m=re.match(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@',lines[i])
        if not m:raise RuntimeError('Invalid hunk header')
        old_start,old_count,new_start,new_count=(int(m[1]),int(m[2] or 1),int(m[3]),int(m[4] or 1))
        out.extend(src[at:old_start-1]);at=old_start-1
        if len(out)!=new_start-1:raise RuntimeError('New offset mismatch')
        used=made=0;i+=1
        while i<len(lines) and not lines[i].startswith('@@ '):
            line=lines[i];i+=1
            if not line or line[0] not in ' +-':raise RuntimeError('Invalid patch line')
            if line[0] in ' -':
                if at>=len(src) or src[at]!=line[1:]:raise RuntimeError('Parent context mismatch')
                at+=1;used+=1
            if line[0] in ' +':out.append(line[1:]);made+=1
        if (used,made)!=(old_count,new_count):raise RuntimeError('Hunk length mismatch')
    out.extend(src[at:]);return ''.join(out)
def main():
    parent=(ROOT/'src/bin/v210_collection_color_quotient.cpp').read_text()
    if digest(parent)!=PARENT:raise RuntimeError('Parent v210 SHA256 differs')
    for name,expected in TARGETS.items():
        patch=(ROOT/f'adhoc/v304/{name[:4]}_vs_v210.patch').read_text()
        text=apply(parent,patch)
        if digest(text)!=expected:raise RuntimeError('Generated solver SHA256 differs')
        dest=ROOT/f'src/bin/{name}.cpp'
        if dest.exists() and dest.read_text()!=text:raise RuntimeError('Refusing to replace existing different source: '+str(dest))
        dest.write_text(text);print(name,expected)
if __name__=='__main__':main()
