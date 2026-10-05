#!/usr/bin/env python3
"""Read-only raw-plan numerical replay after the single evaluation; no search."""
from pathlib import Path
import subprocess,json,hashlib,concurrent.futures
r=Path(__file__).resolve().parents[2];o=r/'results/analysis/v402'
def check(path):
 case=path.name.split('_')[0]+'.txt';row={'raw':path.name,'case':case}
 for mode in ['local','nonlocal']:
  logs=[]
  for v in ['106','402']:
   c=subprocess.run([str(o/f'probe_{v}_{mode}'),str(path)],input=(r/'tools/in'/case).read_bytes(),capture_output=True,check=True)
   logs.append(c.stdout)
  assert logs[0]==logs[1],(path,mode)
  row[mode]={'scored':len(logs[0].splitlines())-1,'sha256':hashlib.sha256(logs[0]).hexdigest()}
 assert row['local']==row['nonlocal']
 return row
with concurrent.futures.ThreadPoolExecutor(max_workers=8) as e:rows=list(e.map(check,sorted((o/'repair_outputs').glob('*_raw.txt'))))
assert rows
(o/'raw_numeric_check.json').write_text(json.dumps({'passed':True,'raw_seeds':len(rows),'rows':rows},indent=2))
print('raw numeric passed',len(rows),'seeds',sum(x['local']['scored'] for x in rows),'scores per mode')
