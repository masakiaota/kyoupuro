#!/usr/bin/env python3
"""v064の凍結、ビルド照合、固定した機構確認。通常評価は起動しない。"""
from pathlib import Path
import argparse, csv, hashlib, json, os, shutil, subprocess
from replay_slime_output import replay

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'adhoc/v064_audit/20260930_submit'
NAME='v064_sparse_interval_lns'
CASES=['0015','0019','0034','0075','0081','0092']
PARENT=ROOT/'results/analysis/v059/20260929T235018_0cd76222/outputs'

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def write(path,data):path.write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n')
def env():
 e=os.environ.copy()
 if os.uname().sysname=='Darwin':
  e.setdefault('SDKROOT',subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip())
  e.setdefault('MACOSX_DEPLOYMENT_TARGET','15.0')
 for key in ['OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','VECLIB_MAXIMUM_THREADS']:e[key]='1'
 return e

def run(command,stem,input_path=None,timeout=180):
 with (OUT/(stem+'.out')).open('wb') as stdout,(OUT/(stem+'.err')).open('wb') as stderr:
  with (input_path or Path(os.devnull)).open('rb') as stdin:
   subprocess.run([str(x) for x in command],cwd=ROOT,env=env(),stdin=stdin,stdout=stdout,stderr=stderr,check=True,timeout=timeout)

def prepare():
 OUT.mkdir(parents=True,exist_ok=True)
 assert not (OUT/'mechanism.json').exists(),'機構確認済みの実行は繰り返さない'
 source=ROOT/'src/bin'/f'{NAME}.cpp';parent=ROOT/'src/bin/v059_repair_priority.cpp'
 restored=source.read_text()
 for c in reversed(json.loads((ROOT/'adhoc/v064_audit/registered_changes.json').read_text())):
  assert restored.count(c['new'])==1
  restored=restored.replace(c['new'],c['old'],1)
 assert restored==parent.read_text()
 reconstructed=OUT/'parent_reconstructed.cpp';reconstructed.write_text(restored)
 # Both modes have already compiled using the standard build script. Preserve
 # the most recent --no-local output before eval.py rebuilds its LOCAL binary.
 shutil.copy2(ROOT/'target/release'/NAME,OUT/'production')
 run(['scripts/build_solver.sh','check_v064_sparse_interval'],'build_check')
 shutil.copy2(ROOT/'target/release/check_v064_sparse_interval',OUT/'check')
 compiler=os.environ.get('CXX','g++-15')
 for mode,defines in [('local',['-DLOCAL']),('production',['-DATCODER','-DONLINE_JUDGE','-DNOMINMAX'])]:
  processed=[]
  for label,file in [('parent',parent),('reconstructed',reconstructed)]:
   output=OUT/f'{mode}_{label}.ii'
   with output.open('wb') as f:
    subprocess.run([compiler,'-std=gnu++23','-E','-P',*defines,str(file)],cwd=ROOT,env=env(),stdout=f,check=True)
   # Darwin's assert macro emits __FILE_NAME__, while other diagnostics use
   # the full path. Normalize only these source-file string literals.
   text=output.read_text().replace(json.dumps(str(file)),json.dumps('SOURCE.cpp')).replace(json.dumps(file.name),json.dumps('SOURCE.cpp'))
   processed.append(text)
  assert processed[0]==processed[1],f'{mode}: 登録差分以外が異なる'
 snapshots=OUT/'snapshot';snapshots.mkdir(exist_ok=True)
 files=[source,parent,ROOT/'adhoc/bin/check_v064_sparse_interval.cpp',ROOT/'adhoc/scripts/audit_v064.py',ROOT/'adhoc/scripts/build_v064_source.py',ROOT/'notes/experiments/v064.md',ROOT/'adhoc/v064_audit/registered_changes.json']
 hashes={}
 for p in files:
  target=snapshots/p.name;shutil.copy2(p,target);hashes[str(p.relative_to(ROOT))]=sha(p)
 inputs={p.name:sha(p) for p in sorted((ROOT/'tools/in').glob('*.txt'))}
 assert len(inputs)==100
 parent_outputs={p.name:sha(p) for p in sorted(PARENT.glob('*.txt'))}
 assert len(parent_outputs)==100
 write(OUT/'manifest.json',dict(source_files=hashes,input_hashes=inputs,parent_output_hashes=parent_outputs,
       cases=CASES,attempts_per_case=64,production_sha256=sha(OUT/'production'),checker_sha256=sha(OUT/'check')))
 write(OUT/'build_verification.json',dict(status='passed',local_and_production_builds=True,
       registered_difference_reversal=True,preprocessed_reconstructed_parent_equal=['LOCAL','nonLOCAL'],solver_executions=0))
 print('v064 prepared: source frozen; both macro modes checked',flush=True)

def mechanism():
 assert not (OUT/'mechanism.json').exists(),'機構確認済みの実行は繰り返さない'
 manifest=json.loads((OUT/'manifest.json').read_text())
 for name,h in manifest['source_files'].items():assert sha(ROOT/name)==h,name
 fixture=OUT/'fixture.txt';fixture.write_text('12 4\nABCD........\nabcd........\n'+'............\n'*10)
 run([OUT/'check','--self-test',OUT/'self_test'],'self_test',fixture,30)
 test=json.loads((OUT/'self_test.out').read_text());assert test['self_tests']==8 and test['saved']==2 and test['dense_sparse_equal']
 for p in (OUT/'self_test').glob('*.txt'):
  assert replay(fixture,p)['metrics']['E']==0
 reports=[];replayed=0
 for case in CASES:
  inp=ROOT/'tools/in'/f'{case}.txt';plan=PARENT/f'{case}.txt'
  assert sha(inp)==manifest['input_hashes'][inp.name] and sha(plan)==manifest['parent_output_hashes'][plan.name]
  output=OUT/'saved'/case
  run([OUT/'check',plan,output],f'saved_{case}',inp,30)
  report=json.loads((OUT/f'saved_{case}.out').read_text());assert report['attempts']==64
  for p in output.glob('*.txt'):
   metrics=replay(inp,p)['metrics'];assert metrics['E']==0 and metrics['T']<=len(plan.read_text().splitlines());replayed+=1
  reports.append(dict(case=case,**report));print('mechanism',case,report,flush=True)
 passed=sum(r['completed'] for r in reports)>0 and sum(r['skipped_layers'] for r in reports)>0
 write(OUT/'mechanism.json',dict(status='passed' if passed else 'gate_not_met',self_test=test,cases=reports,independently_replayed_candidates=replayed))
 if not passed:raise SystemExit('v064の通常評価への進行条件を満たさない')
 print('v064 mechanism passed; source and parameters unchanged',flush=True)

if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('stage',choices=['prepare','mechanism']);a=p.parse_args()
 (prepare if a.stage=='prepare' else mechanism)()
