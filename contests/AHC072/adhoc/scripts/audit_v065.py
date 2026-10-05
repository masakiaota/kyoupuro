#!/usr/bin/env python3
"""v065の凍結、ビルド照合、固定した機構確認。通常評価は起動しない。"""
from pathlib import Path
import argparse, csv, hashlib, json, os, shutil, subprocess
from replay_slime_output import replay

ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'adhoc/v065_audit/20260930_submit'
NAME='v065_bounded_interval_lns'
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
 for c in reversed(json.loads((ROOT/'adhoc/v065_audit/registered_changes.json').read_text())):
  assert restored.count(c['new'])==1
  restored=restored.replace(c['new'],c['old'],1)
 assert restored==parent.read_text()
 reconstructed=OUT/'parent_reconstructed.cpp';reconstructed.write_text(restored)
 # Both modes have already compiled using the standard build script. Preserve
 # the most recent --no-local output before eval.py rebuilds its LOCAL binary.
 shutil.copy2(ROOT/'target/release'/NAME,OUT/'production')
 run(['scripts/build_solver.sh','check_v065_bounded_interval'],'build_check')
 shutil.copy2(ROOT/'target/release/check_v065_bounded_interval',OUT/'check')
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
 files=[source,parent,ROOT/'adhoc/bin/check_v065_bounded_interval.cpp',ROOT/'adhoc/scripts/audit_v065.py',ROOT/'adhoc/scripts/build_v065_source.py',ROOT/'notes/experiments/v065.md',ROOT/'adhoc/v065_audit/registered_changes.json',ROOT/'adhoc/bin/v061_bounded_lns_probe.cpp',ROOT/'src/bin/v064_sparse_interval_lns.cpp']
 hashes={}
 for p in files:
  target=snapshots/p.name;shutil.copy2(p,target);hashes[str(p.relative_to(ROOT))]=sha(p)
 inputs={p.name:sha(p) for p in sorted((ROOT/'tools/in').glob('*.txt'))}
 assert len(inputs)==100
 parent_outputs={p.name:sha(p) for p in sorted(PARENT.glob('*.txt'))}
 assert len(parent_outputs)==100
 protected={name:sha(ROOT/name) for name in ['AGENTS.md','README.md','src/bin/v000_template.cpp','notes/notations.md','notes/important_properties.md','scripts/build_solver.sh','scripts/eval.py']}
 write(OUT/'manifest.json',dict(source_files=hashes,protected_files=protected,input_hashes=inputs,parent_output_hashes=parent_outputs,
       cases=CASES,attempts_per_case=64,production_sha256=sha(OUT/'production'),checker_sha256=sha(OUT/'check')))
 write(OUT/'build_verification.json',dict(status='passed',local_and_production_builds=True,
       registered_difference_reversal=True,preprocessed_reconstructed_parent_equal=['LOCAL','nonLOCAL'],solver_executions=0))
 print('v065 prepared: source frozen; both macro modes checked',flush=True)

def mechanism():
 assert not (OUT/'mechanism.json').exists(),'機構確認済みの実行は繰り返さない'
 manifest=json.loads((OUT/'manifest.json').read_text())
 for name,h in manifest['source_files'].items():assert sha(ROOT/name)==h,name
 fixture=OUT/'fixture.txt';fixture.write_text('12 4\nABCD........\nabcd........\n'+'............\n'*10)
 run([OUT/'check','--self-test',OUT/'self_test'],'self_test',fixture,30)
 test=json.loads((OUT/'self_test.out').read_text());assert test['self_tests']==7 and test['saved']==2 and test['uses_outside'] and test['deadline_restored']
 for p in (OUT/'self_test').glob('*.txt'):
  assert replay(fixture,p)['metrics']['E']==0
 reports=[];replayed=0
 for case in CASES:
  inp=ROOT/'tools/in'/f'{case}.txt';plan=PARENT/f'{case}.txt'
  assert sha(inp)==manifest['input_hashes'][inp.name] and sha(plan)==manifest['parent_output_hashes'][plan.name]
  output=OUT/'saved'/case
  run([OUT/'check',plan,output],f'saved_{case}',inp,30)
  report=json.loads((OUT/f'saved_{case}.out').read_text());assert report['attempts']==64
  original_T=len(plan.read_text().splitlines());savings=[]
  for p in output.glob('*.txt'):
   metrics=replay(inp,p)['metrics'];assert metrics['E']==0 and metrics['T']<=original_T;replayed+=1
   savings.append(original_T-metrics['T'])
  assert len(savings)==report['returned'] and sum(savings)==report['saved']
  assert sum(x>0 for x in savings)==report['shortened'] and max(savings,default=0)==report['best_saved']
  with (output/'calls.csv').open() as f:call_rows=list(csv.DictReader(f))
  assert len(call_rows)==64 and all(0<=int(r['span'])<=64 for r in call_rows)
  assert sum(int(r['returned']) for r in call_rows)==report['returned']
  assert abs(sum(float(r['elapsed_ms']) for r in call_rows)-report['elapsed_ms'])<0.001
  reports.append(dict(case=case,**report));print('mechanism',case,report,flush=True)
 passed=(sum(r['returned'] for r in reports)>0 and sum(r['saved'] for r in reports)>=1
         and all(r['elapsed_ms']<=r['time_gate_ms'] for r in reports))
 write(OUT/'mechanism.json',dict(status='passed' if passed else 'gate_not_met',self_test=test,cases=reports,independently_replayed_candidates=replayed,
       returned=sum(r['returned'] for r in reports),saved=sum(r['saved'] for r in reports),
       best_saved_sum=sum(r['best_saved'] for r in reports),
       time_gate=all(r['elapsed_ms']<=r['time_gate_ms'] for r in reports),solver_revisions_after_execution=0))
 if not passed:raise SystemExit('v065の通常評価への進行条件を満たさない')
 print('v065 mechanism passed; source and parameters unchanged',flush=True)

if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('stage',choices=['prepare','mechanism']);a=p.parse_args()
 (prepare if a.stage=='prepare' else mechanism)()
