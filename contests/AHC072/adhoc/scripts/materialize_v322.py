#!/usr/bin/env python3
"""Reconstruct frozen v322. Never execute a solver or register an evaluation."""
from pathlib import Path
import base64, hashlib, json, runpy, zlib
ROOT = Path(__file__).resolve().parents[2]
EXPECTED_PAYLOAD = '6039a619c8e7679ea104734b75ca66d180aef825df82ada7e1c6cb4f84800d29'
EXPECTED_SOURCE = 'abb46ec1d213e38840fd3780d9fdfc512f17383faa9b5234070d809bb9789fd9'
encoded = ''.join((ROOT / 'adhoc/v322/source_transfer.b64').read_text().split())
# Correct the documented single transport insertion, not the evaluated code.
encoded = encoded.replace('07VS6axtSD', '07VS6xtSD')
raw = zlib.decompress(base64.b64decode(encoded, validate=True))
assert hashlib.sha256(raw).hexdigest() == EXPECTED_PAYLOAD, 'Transport checksum mismatch'
for name, content in json.loads(raw).items():
    target = ROOT / name
    assert name in ('adhoc/v322/sparse_rewriter.inc', 'adhoc/scripts/build_v322_source.py')
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content)
runpy.run_path(str(ROOT / 'adhoc/scripts/build_v322_source.py'), run_name='__main__')
source = ROOT / 'src/bin/v322_sparse_plan_rewriting.cpp'
assert hashlib.sha256(source.read_bytes()).hexdigest() == EXPECTED_SOURCE
print('v322 source checksum verified; no solver runs')
