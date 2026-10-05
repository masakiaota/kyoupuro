#!/usr/bin/env python3
"""v201再作成用の入力準備と固定条件の評価。閾値の選定は別の静的分析で行う。"""
from pathlib import Path
import hashlib,json,os,secrets,shutil,subprocess,sys,time
from run_v077_overnight import prior_inputs
from check_v037_results import ERRORS,log_values,verify_output
ROOT=Path(__file__).resolve().parents[2]
WORK=ROOT/'results/analysis/v201_fresh'
PARENTS=['v076_relative_tuned','v079_nn_immediate']
BIN='v201_floor_nn_selector'
def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v):
 p.parent.mkdir(parents=True,exist_ok=True); p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def read(p): return json.loads(p.read_text())
def records():
 with (ROOT/'results/eval_records.jsonl').open() as f: return [json.loads(l) for l in f]
def prepare():
 assert not WORK.exists(); WORK.mkdir(parents=True)
 seeds,hashes=prior_inputs(WORK)
 for p in ROOT.glob('results/**/inputs.jsonl'):
  with p.open() as f:
   for line in f:
    r=json.loads(line)
    if 'seed' in r: seeds.add(r['seed'])
    if 'sha256' in r: hashes.add(r['sha256'])
 for p in ROOT.glob('results/**/generated.jsonl'):
  with p.open() as f:
   for line in f:
    r=json.loads(line)
    if 'seed' in r: seeds.add(r['seed'])
    if 'sha256' in r: hashes.add(r['sha256'])
 save(WORK/'excluded.json',dict(seeds=sorted(seeds),hashes=sorted(hashes)))
 fresh=[]
 while len(fresh)<700:
  value=secrets.randbits(63)|(1<<63)
  if value not in seeds: seeds.add(value); fresh.append(value)
 (WORK/'seeds.txt').write_text(''.join(f'{s}\n' for s in fresh))
 subprocess.run(['sh','scripts/gen_tools.sh',str(WORK/'seeds.txt'),'--dir',str(WORK/'generated')],cwd=ROOT,check=True)
 manifest=[]
 for i,s in enumerate(fresh):
  p=WORK/'generated'/f'{i:04d}.txt'; h=sha(p); assert h not in hashes; hashes.add(h)
  role='selection500' if i<500 else 'holdout200'; name=f'{i:04d}.txt'
  dest=WORK/role/name; dest.parent.mkdir(exist_ok=True); shutil.move(p,dest)
  manifest.append(dict(case=name,seed=s,sha256=h,role=role,path=str(dest.relative_to(ROOT))))
 (WORK/'generated').rmdir()
 # 分割は入力ハッシュだけから作り、スコアを参照しない。
 order=sorted((r for r in manifest if r['role']=='selection500'),key=lambda r:r['sha256'])
 for i,r in enumerate(order): r['fold']=i%5
 save(WORK/'manifest.json',manifest)
 protected=[ROOT/f'src/bin/{b}.cpp' for b in PARENTS]+[ROOT/'notes/experiments/v201.md',ROOT/'scripts/eval.py',ROOT/'scripts/build_solver.sh',ROOT/'tools/target/release/gen',WORK/'manifest.json']
 save(WORK/'preflight.json',dict(sha256={str(p.relative_to(ROOT)):sha(p) for p in protected},selection_cases=500,holdout_cases=200,local=True,jobs=2,time_ratio=.80))
 for b in PARENTS:
  scratch=ROOT/'results/out'/b
  if scratch.exists(): shutil.copytree(scratch,WORK/'previous_outputs'/b)
 print(f'prepared 500 selection + 200 holdout; excluded {len(hashes)-700} existing hashes',flush=True)
def unchanged():
 for name,h in read(WORK/'preflight.json')['sha256'].items():
  if name!='notes/experiments/v201.md': assert sha(ROOT/name)==h,name
 for r in read(WORK/'manifest.json')+read(WORK/'composite_manifest.json'): assert sha(ROOT/r['path'])==r['sha256']
def evaluate(b,role):
 unchanged(); label=f'v201_fresh_{role}_{b.split("_")[0]}'
 assert not any(r['label']==label for r in records()),'do not repeat evaluation'
 dest=WORK/'runs'/label; dest.mkdir(parents=True,exist_ok=False)
 if b==BIN or role=='holdout_composite200':
  frozen=read(WORK/'solver_frozen.json'); assert sha(ROOT/f'src/bin/{BIN}.cpp')==frozen['source_sha256']
  assert sha(WORK/'selection.json')==frozen['selection_sha256']
  assert read(WORK/'diagnostic.json')['passed']
 folder=ROOT/'tools/in' if role=='tools_in100' else WORK/role
 cmd=[sys.executable,'scripts/eval.py','-v','--label',label,b,str(folder)]
 print(f'START {label}',flush=True)
 p=subprocess.Popen(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,bufsize=1)
 last=time.monotonic()
 for line in p.stdout:
  if time.monotonic()-last>=30 or line.startswith(('eval:','summary:','error:','bin=')):
   print(line.rstrip(),flush=True); last=time.monotonic()
 assert p.wait()==0, f'eval failed: {label}'
 rows=[r for r in records() if r['label']==label]
 n=100 if role=='tools_in100' else (500 if role=='selection500' else 200)
 assert len(rows)==n and len({r['run_id'] for r in rows})==1
 shutil.copytree(ROOT/'results/out'/b,dest/'out')
 for r in rows:
  path=dest/'out'/r['case_name']; T=verify_output(str(folder/r['case_name']),path)
  c,t,errors=log_values(Path(str(path)+'.err'))
  assert r['status']=='ok' and r['local'] and T==r['score']==c['T']==c['final_ops']==c['validated_moves']
  assert c['E']==0 and not errors and all(c[k]==0 for k in ERRORS)
  assert c['state_pool_free_at_end']==c['state_slots']==4 and T<=100000
  r.update(saved_output=str(path.relative_to(ROOT)),counts=c,times_ms=t)
 save(dest/'cases.json',rows)
 summary=dict(run_id=rows[0]['run_id'],label=label,bin=b,role=role,cases=n,total=sum(r['score'] for r in rows),max_elapsed_ms=max(r['elapsed'] for r in rows),verified=True)
 save(dest/'summary.json',summary); unchanged(); print(json.dumps(summary),flush=True)
def main():
 action=sys.argv[1]
 if action=='prepare': prepare()
 elif action=='parents':
  for b in PARENTS:
   subprocess.run(['sh','scripts/build_solver.sh','--no-local',b],cwd=ROOT,check=True)
   evaluate(b,'selection500')
 elif action=='final':
  evaluate(BIN,'holdout200')
  evaluate(BIN,'tools_in100')
 else: raise ValueError(action)
if __name__=='__main__': main()
