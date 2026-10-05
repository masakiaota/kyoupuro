#!/usr/bin/env python3
"""Reconstruct the exact frozen v317 from its two repository parents.
No solvers, training or eval registration are run. Weights are not duplicated.
"""
from pathlib import Path
import base64,difflib,hashlib,json,re,zlib
ROOT=Path(__file__).resolve().parents[2]
PARENT='ceb38ea758212dbbd91f8faa63eb6684fd96aab4b3b60a82ea718150538d4618'
LEARNED='c3a8402fbeef98385ff209c3bf07d7350f82b2646409ca9dfcc700eb3f12b9db'
EXPECTED='5bd46f9b88d3e372ae1fb22d6f213a9188a1624ae0b54c3982773a861d57a39f'
EDITS='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'
parent_path=ROOT/'src/bin/v401_incremental_cnn.cpp'
learned_path=ROOT/'adhoc/bin/v109_assisted_learned.cpp'
parent=parent_path.read_text();learned=learned_path.read_text()
assert hashlib.sha256(parent.encode()).hexdigest()==PARENT,'Parent changed'
assert hashlib.sha256(learned.encode()).hexdigest()==LEARNED,'Learned checkpoint source changed'
lines=parent.splitlines(True)
for first,last,replacement in reversed(json.loads(zlib.decompress(base64.b64decode(EDITS)))):
    lines[first:last]=replacement
text=''.join(lines)
packed=re.search(r'static constexpr char nn_packed\[\] =\s*((?:"[A-Za-z0-9+/=]*"\s*)+);',learned).group(0)
assert text.count('/* V317_COMPLETION_PACKED */')==1
text=text.replace('/* V317_COMPLETION_PACKED */',packed.replace('nn_packed[]','nn_completion_packed[]'))
assert hashlib.sha256(text.encode()).hexdigest()==EXPECTED,'Reconstruction mismatch'
source=ROOT/'src/bin/v317_complementary_policies.cpp'
if source.exists():assert source.read_text()==text,'Refusing to replace different candidate'
else:source.write_text(text)
out=ROOT/'adhoc/v317';out.mkdir(parents=True,exist_ok=True)
(out/'v317_vs_v401.patch').write_text(''.join(difflib.unified_diff(parent.splitlines(True),text.splitlines(True),fromfile='a/src/bin/v401_incremental_cnn.cpp',tofile='b/src/bin/v317_complementary_policies.cpp')))
print(EXPECTED,source)
