#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
source adhoc/scripts/cloud_toolchain_env.sh
out=results/analysis/v402
for mode in local nonlocal; do
 args=(); macros=(-DLOCAL)
 if [[ $mode == nonlocal ]];then args=(--no-local);macros=(-DATCODER -DONLINE_JUDGE -DNOMINMAX);fi
 scripts/build_solver.sh "${args[@]}" v402_initial_nn_repair >"$out/build_$mode.log" 2>&1
 cp target/release/v402_initial_nn_repair "$out/solver_$mode"
 for v in 106 402;do
  scripts/build_solver.sh "${args[@]}" v402_selector_reference_$v >"$out/build_reference_${v}_${mode}.log" 2>&1
  cp target/release/v402_selector_reference_$v "$out/probe_${v}_$mode"
 done
 "$CXX" -std=gnu++23 -O2 -march=native -fopenmp -E -P "${macros[@]}" src/bin/v402_initial_nn_repair.cpp >"$out/candidate_$mode.ii"
 "$CXX" -std=gnu++23 -O2 -march=native -fopenmp -E -P "${macros[@]}" src/bin/v401_incremental_cnn.cpp >"$out/parent_$mode.ii"
 diff -u "$out/parent_$mode.ii" "$out/candidate_$mode.ii" >"$out/preprocess_$mode.diff" || [[ $? == 1 ]]
done
python3 - <<'PY'
from pathlib import Path
import subprocess,json,hashlib,concurrent.futures
r=Path.cwd();o=r/'results/analysis/v402';p=r/'results/analysis/v401/outputs'
def check(inp):
 out=p/inp.name;row={'case':inp.name}
 for mode in ['local','nonlocal']:
  logs=[]
  for v in ['106','402']:
   c=subprocess.run([str(o/f'probe_{v}_{mode}'),str(out)],input=inp.read_bytes(),capture_output=True,check=True)
   logs.append(c.stdout)
  assert logs[0]==logs[1], (inp.name,mode)
  row[mode]={'scored':len(logs[0].splitlines())-1,'sha256':hashlib.sha256(logs[0]).hexdigest()}
  (o/'numeric').mkdir(exist_ok=True);(o/'numeric'/f'{inp.stem}_{mode}.txt').write_bytes(logs[0])
 assert row['local']==row['nonlocal']
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=8) as e:rows=list(e.map(check,sorted((r/'tools/in').glob('*.txt'))))
(o/'numeric_check.json').write_text(json.dumps({'passed':True,'cases':len(rows),'rows':rows},indent=2))
print('numeric passed',len(rows),'cases',sum(x['local']['scored'] for x in rows),'scores per mode')
PY
