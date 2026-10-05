#!/usr/bin/env python3
"""One standard eval invocation plus read-only /proc VmHWM sampling."""
from pathlib import Path
import subprocess,time,json,os
r=Path(__file__).resolve().parents[2];o=r/'results/analysis/v402'; binary=r/'target/release/v402_initial_nn_repair';seen={}
with (o/'eval.log').open('w') as log:
 p=subprocess.Popen(['python3','scripts/eval.py','v402_initial_nn_repair','tools/in','-j','8','--label','cloud_v402_gcc15.3_LOCAL_search1.900_core1.870466_j8_20261004','-v'],cwd=r,stdout=log,stderr=subprocess.STDOUT)
 while p.poll() is None:
  for f in Path('/proc').glob('[0-9]*'):
   try:
    if os.readlink(f/'exe')!=str(binary):continue
    status=(f/'status').read_text(); vals={line.split(':',1)[0]:line.split(':',1)[1].strip() for line in status.splitlines()}
    key=f.name;v=int(vals.get('VmHWM','0 kB').split()[0]);seen[key]=max(seen.get(key,0),v)
   except (OSError,ValueError):pass
  time.sleep(.1)
(o/'memory.json').write_text(json.dumps({'method':'/proc VmHWM sampled every 100ms; includes warmup, observed lower bound, not exact exit peak','processes':seen,'max_observed_kib':max(seen.values(),default=0),'returncode':p.returncode},indent=2))
raise SystemExit(p.returncode)
