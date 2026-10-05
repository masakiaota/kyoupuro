#!/usr/bin/env python3
"""Package frozen results, without invoking a solver or changing the implementation."""
from pathlib import Path
import zipfile,hashlib,json,shutil
r=Path(__file__).resolve().parents[2];o=r/'results/analysis/v402';d=r/'adhoc/imports/v402';d.mkdir(parents=True,exist_ok=True)
source=r/'src/bin/v402_initial_nn_repair.cpp';assert hashlib.sha256(source.read_bytes()).hexdigest()=='fd4de2b5f0fcbc7ece772d1a619c0f63bcd4cf9ae0444f797f1feed346d5263d'
for name in ['result.json','stage_analysis.json','component_hashes.json']:
 shutil.copy2(o/name,d/name)
(d/'README.md').write_text('''# v402 frozen experiment evidence

Result: rejected. One In100 evaluation, GCC15.3/Rust1.89, LOCAL 1.90/1.930, j8. The implementation was not modified after evaluation. See notes/experiments/v402.md.

`reproduction.zip` contains official per-case records, independent replay summaries and per-case output hashes, numerical equality summaries, stage-level observations, preprocessing differences and hashes, the frozen source/note, and v402 scripts. Full preprocessing `.ii`, compiled binaries, duplicated extracted repair files and numeric text are omitted from this compact archive; their SHA256 manifest is retained and scripts reproduce preprocessing/probes. All raw output/stderr (including raw and accepted repair plans) and bulk evidence remain under results/analysis/v402 and the separately delivered full ZIP.

The recorded cloud toolchain helpers assume the original cloud installation under /tmp; on another environment provide GCC15.3 and Rust1.89. Standard scoring entry: `CXX=g++-15 python3 scripts/eval.py v402_initial_nn_repair tools/in -j 8 --label <new-authorized-label>`. Do not run again without a new explicit user instruction. Source materialization and numerical probes also require the frozen v401/v106 parent sources from this commit.

`run_v402_checks.sh` expects the original cloud helper path. Exact original helpers are archived under reproduction_helpers; they are evidence, not installed software. Existing repo scripts/build_solver.sh and scripts/eval.py remain unchanged.

No AtCoder submission or public repository upload occurred. The private main publication is coordinated separately.
''')
manifest={str(p.relative_to(r)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(o.rglob('*')) if p.is_file()}
(d/'evidence_manifest.json').write_text(json.dumps(manifest,indent=2))
base=[source,r/'notes/experiments/v402.md']+sorted((r/'adhoc/scripts').glob('*v402*'))+sorted((r/'adhoc/bin').glob('v402_selector_reference_*.cpp'))
compact=[p for p in o.rglob('*') if p.is_file() and p.suffix!='.ii' and p.name not in ['solver_local','solver_nonlocal'] and not p.name.startswith('probe_') and not any(x in p.parts for x in ['numeric','repair_outputs','outputs'])]
helpers=[r/'adhoc/scripts/cloud_toolchain_env.sh',r/'adhoc/scripts/cloud_gcc15.py']
with zipfile.ZipFile(d/'reproduction.zip','w',zipfile.ZIP_DEFLATED,9) as z:
 for p in sorted(set(base+compact+[d/'README.md',d/'evidence_manifest.json'])):z.write(p,str(p.relative_to(r)))
 for p in helpers:z.write(p,'reproduction_helpers/'+p.name)
 z.write(r/'results/analysis/cloud_baseline/fixed_reference.json','results/analysis/cloud_baseline/fixed_reference.json')
delivery=r.parent/'v402_delivery';delivery.mkdir(exist_ok=True)
shutil.copy2(source,delivery/source.name);shutil.copy2(r/'notes/experiments/v402.md',delivery/'v402.md')
with zipfile.ZipFile(delivery/'v402_complete_evidence.zip','w',zipfile.ZIP_DEFLATED,9) as z:
 for p in sorted(set(base+[p for p in o.rglob('*') if p.is_file() and not p.name.startswith('probe_') and p.name not in ['solver_local','solver_nonlocal']])):z.write(p,str(p.relative_to(r)))
 for p in helpers:z.write(p,'reproduction_helpers/'+p.name)
 z.write(r/'results/analysis/cloud_baseline/fixed_reference.json','results/analysis/cloud_baseline/fixed_reference.json')
for p in [d/'reproduction.zip',delivery/'v402_complete_evidence.zip']:print(p, p.stat().st_size,hashlib.sha256(p.read_bytes()).hexdigest())
