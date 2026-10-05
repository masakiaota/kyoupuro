#!/usr/bin/env python3
"""Materialize the frozen v324 source without modifying its parent or running it."""
from pathlib import Path
import hashlib,re
root=Path(__file__).resolve().parents[2]
parent=root/'src/bin/v113_integrated_nn_lns.cpp'
original=parent.read_text()
assert hashlib.sha256(original.encode()).hexdigest()=='037f9fcca137552c097bc396428699850cf591cf58e513e874c9058309bfd258'
lines=original.splitlines(keepends=True)
patch=(root/'adhoc/v324/source.patch').read_text().splitlines(keepends=True)
result=[];cursor=0;i=2
while i<len(patch):
    match=re.fullmatch(r'@@ -(\d+),(\d+) \+(\d+),(\d+) @@\n',patch[i])
    assert match,repr(patch[i])
    start=int(match[1])-1
    assert start>=cursor
    result.extend(lines[cursor:start]);cursor=start;i+=1
    old_count=0;new_count=0
    while i<len(patch) and not patch[i].startswith('@@ '):
        kind,body=patch[i][0],patch[i][1:]
        assert kind in ' +-'
        if kind in ' -':
            assert lines[cursor]==body,(cursor,lines[cursor],body)
            cursor+=1;old_count+=1
        if kind in ' +':result.append(body);new_count+=1
        i+=1
    assert old_count==int(match[2]) and new_count==int(match[4])
result.extend(lines[cursor:]);source=''.join(result)
expected='914b966c5675db6ea1e8a236477fee573c4ba919a8e685ddc99e58dc0fba2518'
assert hashlib.sha256(source.encode()).hexdigest()==expected,'source transfer mismatch'
output=root/'src/bin/v324_repair_cost_tournament.cpp'
if output.exists():assert output.read_text()==source,'refuse to overwrite a different source'
else:output.write_text(source)
assert parent.read_text()==original
print(expected,output.relative_to(root))
