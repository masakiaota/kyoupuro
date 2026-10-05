#!/usr/bin/env python3
"""確定済みの選択規則だけをC++へ転記し、親との差分と機構を確認する。"""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import difflib,json,os,re,subprocess
from v201_fresh import ROOT,WORK,PARENTS,BIN,read,save,sha,unchanged
from analyze_v201_fresh import FEATURES,predict
from check_v037_results import ERRORS,log_values,verify_output
AUDIT=ROOT/'adhoc/v201_floor_audit'
FLAGS=['-std=gnu++23','-O2','-Wall','-Wextra','-march=native','-pthread','-ftrivial-auto-var-init=zero','-fopenmp']
MODES={'local':['-DLOCAL'],'production':['-DATCODER','-DONLINE_JUDGE','-DNOMINMAX']}
def run(args,**kw):
 p=subprocess.run(args,cwd=ROOT,text=True,capture_output=True,check=True,**kw)
 if p.stderr: print(p.stderr,flush=True)
 return p.stdout

def selected_names(tree):
 if 'feature' not in tree:return set()
 return {tree['feature']}|selected_names(tree['left'])|selected_names(tree['right'])
def expression(tree,names):
 if 'feature' not in tree:return 'true' if tree['use_nn'] else 'false'
 i=names.index(tree['feature']); t=repr(tree['threshold'])
 return f'(f[{i}]<={t} ? {expression(tree["left"],names)} : {expression(tree["right"],names)})'

def functions(tree):
 names=[f for f in FEATURES if f in selected_names(tree)]
 variables=dict(M='b.M',N='b.N',K='b.K',floors='b.cell_count',wall_fraction='1.-double(b.cell_count)/(b.N*b.N)',
  density='double(b.M)/b.cell_count',mean_per_color='double(b.M)/b.K',max_color_fraction='double(maximum_color)/b.M',
  color_concentration='double(squared_counts)/(b.M*b.M)',rare_fraction='double(rare_slimes)/b.M',home_mean='double(home_sum)/b.M',
  home_max='home_maximum',detour_mean='detour_sum/b.M',same_nearest_mean='same_count ? double(same_sum)/same_count : 0.',
  any_nearest_mean='double(any_sum)/b.M',dead_end_fraction='double(dead_ends)/b.cell_count',
  straight4_fraction='double(straight4)/b.cell_count',stepping2_fraction='double(stepping2)/b.M')
 lines=[f'static array<double,{len(names)}> nn_selector_features() {{','    [[maybe_unused]] const Board &b=board_info;']
 needed=set(names)
 if needed & {'max_color_fraction','color_concentration','rare_fraction'}:
  lines += ['    int counts[13]{};','    for(int p=0;p<b.cell_count;p++) if(b.initial[p]) counts[b.initial[p]]++;']
  if 'max_color_fraction' in needed: lines+=['    const int maximum_color=*max_element(counts+1,counts+b.K+1);']
  if 'color_concentration' in needed: lines+=['    int squared_counts=0;','    for(int c=1;c<=b.K;c++) squared_counts+=counts[c]*counts[c];']
  if 'rare_fraction' in needed: lines+=['    int rare_slimes=0;','    for(int c=1;c<=b.K;c++) if(counts[c]<=3) rare_slimes+=counts[c];']
 if needed & {'home_mean','home_max','detour_mean'}:
  if 'home_mean' in needed: lines+=['    int home_sum=0;']
  if 'home_max' in needed: lines+=['    int home_maximum=0;']
  if 'detour_mean' in needed: lines+=['    double detour_sum=0.;']
  lines+=['    for(int p=0;p<b.cell_count;p++) if(b.initial[p]) {','        const int q=b.nest_pos_by_code[b.initial[p]];','        const int distance=b.floor_dist[p][q];']
  if 'home_mean' in needed:lines+=['        home_sum+=distance;']
  if 'home_max' in needed:lines+=['        home_maximum=max(home_maximum,distance);']
  if 'detour_mean' in needed:lines+=['        detour_sum+=double(distance)/(abs(b.row[p]-b.row[q])+abs(b.col[p]-b.col[q]));']
  lines+=['    }']
 if needed & {'same_nearest_mean','any_nearest_mean'}:
  if 'same_nearest_mean' in needed:lines+=['    int same_sum=0,same_count=0;']
  if 'any_nearest_mean' in needed:lines+=['    int any_sum=0;']
  lines+=['    for(int p=0;p<b.cell_count;p++) if(b.initial[p]) {']
  if 'same_nearest_mean' in needed:lines+=['        int same_nearest=infinite_cost;']
  if 'any_nearest_mean' in needed:lines+=['        int any_nearest=infinite_cost;']
  lines+=['        for(int q=0;q<b.cell_count;q++) if(q!=p&&b.initial[q]) {']
  if 'same_nearest_mean' in needed:lines+=['            if(b.initial[p]==b.initial[q]) same_nearest=min(same_nearest,int(b.floor_dist[p][q]));']
  if 'any_nearest_mean' in needed:lines+=['            any_nearest=min(any_nearest,int(b.floor_dist[p][q]));']
  lines+=['        }']
  if 'same_nearest_mean' in needed:lines+=['        if(same_nearest<infinite_cost) { same_sum+=same_nearest; same_count++; }']
  if 'any_nearest_mean' in needed:lines+=['        any_sum+=any_nearest;']
  lines+=['    }']
 if 'dead_end_fraction' in needed:lines+=['    int dead_ends=0;','    for(int p=0;p<b.cell_count;p++) {','        int degree=0; for(int d=0;d<4;d++) degree+=b.adj[p][d]>=0;','        dead_ends+=degree<=1;','    }']
 if 'straight4_fraction' in needed:lines+=['    int straight4=0;','    for(int p=0;p<b.cell_count;p++) {','        bool found=false; for(int d=0;d<4;d++) found|=b.ray_count[p][d]>=4;','        straight4+=found;','    }']
 if 'stepping2_fraction' in needed:lines+=['    int stepping2=0;','    for(int p=0;p<b.cell_count;p++) if(b.initial[p]) {','        bool found=false; for(int d=0;d<4;d++) {','            const int q=b.ray[p][d][2]; if(q>=0&&b.initial[q]) found=true;','        }','        stepping2+=found;','    }']
 vals=', '.join('double('+variables[n]+')' for n in names)
 lines+=['    return {'+vals+'};','}',f'static bool nn_selector_choice([[maybe_unused]] const array<double,{len(names)}> &f) {{','    return '+expression(tree,names)+';','}','']
 return names,'\n'.join(lines)

def normalize(s):
 for b in [*PARENTS,BIN]:s=s.replace(str(ROOT/f'src/bin/{b}.cpp'),'solver.cpp').replace(f'"{b}.cpp"','"solver.cpp"')
 s=s.replace('"<stdin>"','"solver.cpp"')
 return re.sub(r'("solver.cpp",\s*)\d+',r'\g<1>0',s)

def main():
 unchanged(); assert not (WORK/'solver_frozen.json').exists(); AUDIT.mkdir(exist_ok=True)
 selection=read(WORK/'selection.json'); names,helper=functions(selection['model'])
 source=ROOT/f'src/bin/{BIN}.cpp'; parent=(ROOT/f'src/bin/{PARENTS[1]}.cpp').read_text()
 # 選定用の入力特徴だけを一度計算し、時間を分割せず片方の候補選択へ全時間を使う。
 comment='// 未使用500件の5分割比較で決めた入力条件。NNの重みと探索時間は親から保持する。\n'
 comment+='// 特徴の順序: '+', '.join(names)+'。最終200件の結果による条件調整はしない。\n'
 insert=comment+helper+'\n'
 marker='    void optimize('
 pos=parent.index(marker); brace=parent.index('{',pos)+1
 anchor=parent[pos:brace]
 gate='\n        const auto selector_values=nn_selector_features();\n        const bool use_nn=nn_selector_choice(selector_values);\n        LOCAL_NOTE(trace.count_by("selector_nn_enabled",use_nn); trace.count_by("selector_M",board_info.M);)'
 changes=[dict(old='// v079_nn_immediate.cpp',new=f'// {BIN}.cpp'),
  dict(old='struct ConstructionState {',new=insert+'struct ConstructionState {'),
  dict(old=anchor,new=anchor+gate),
  dict(old='const bool nn_eligible=!dependency&&mode>=3&&choice>=0;',new='const bool nn_eligible=use_nn&&!dependency&&mode>=3&&choice>=0;')]
 current=parent
 for c in changes:
  assert current.count(c['old'])==1,c['old']; current=current.replace(c['old'],c['new'],1)
 source.write_text(current);save(AUDIT/'registered_changes.json',changes)
 restored=current
 for c in reversed(changes):restored=restored.replace(c['new'],c['old'],1)
 assert restored==parent
 env=os.environ.copy()
 if sys.platform=='darwin':env.setdefault('SDKROOT',run(['xcrun','--show-sdk-path']).strip());env.setdefault('MACOSX_DEPLOYMENT_TARGET','15.0')
 compiler=env.get('CXX','g++-15')
 def mode_build(mode):
  flags=FLAGS+MODES[mode]
  def compile(args,**kw):return run([compiler,*flags,*args],env=env,**kw)
  compile([str(source),'-o',str(AUDIT/f'{BIN}_{mode}')])
  expanded={b:normalize(compile(['-E','-P',str(ROOT/f'src/bin/{b}.cpp')])) for b in [*PARENTS,BIN]}
  assert normalize(compile(['-E','-P','-x','c++','-'],input=restored))==expanded[PARENTS[1]]
  cleaned=expanded[BIN]
  start=cleaned.index(f'static array<double,{len(names)}> nn_selector_features()')
  end=cleaned.index('struct ConstructionState {',start); cleaned=cleaned[:start]+cleaned[end:]
  for part in ['const auto selector_values=nn_selector_features();','const bool use_nn=nn_selector_choice(selector_values);']:
   assert cleaned.count(part)==1;cleaned=cleaned.replace(part,'')
  cleaned=cleaned.replace('const bool nn_eligible=use_nn&&','const bool nn_eligible=')
  cleaned=re.sub(r'trace\.count_by\("selector_(?:nn_enabled|M)",[^;]*\);','',cleaned)
  assert cleaned.split()==expanded[PARENTS[1]].split(),'unexpected macro-expanded differences'
  for b in PARENTS:(AUDIT/f'{mode}_{b}.diff').write_text(''.join(difflib.unified_diff(expanded[b].splitlines(True),expanded[BIN].splitlines(True))))
  compile([f'-DAUDIT_SOURCE="{source}"',str(ROOT/'adhoc/bin/probe_v201_fixed_clock.cpp'),'-o',str(AUDIT/f'fixed_{BIN}_{mode}')])
  # 親の診断実行ファイルは前回のソース・実行ファイルハッシュを照合して再利用する。
  old=ROOT/'results/withdrawn/v201_m80_20261002/adhoc/v201_audit'; frozen=read(old/'frozen.json')['sha256']
  for b in PARENTS:
   assert sha(ROOT/f'src/bin/{b}.cpp')==frozen[f'src/bin/{b}.cpp']
   f=old/f'fixed_{b}_{mode}'; assert sha(f)==frozen[f'adhoc/v201_audit/{f.name}']; shutil.copy2(f,AUDIT/f.name)
  print(f'{mode}: build and preprocessing passed',flush=True)
 built=AUDIT/'build_passed.json'
 if built.exists():
  checked=read(built); assert checked['source_sha256']==sha(source)
  for name,h in checked['binaries'].items(): assert sha(ROOT/name)==h
 else:
  with ThreadPoolExecutor(max_workers=2) as pool:list(pool.map(mode_build,MODES))
  save(built,dict(source_sha256=sha(source),binaries={str(p.relative_to(ROOT)):sha(p) for p in AUDIT.iterdir() if p.is_file() and os.access(p,os.X_OK)}))
 # 実装内の特徴関数をそのまま呼ぶ補助入口。探索mainは呼ばず、入力計算だけを照合する。
 prefix=current[:current.rindex('int main(')]
 (AUDIT/'solver_prefix.hpp').write_text(prefix)
 probe=ROOT/'adhoc/bin/check_v201_features.cpp'
 probe.write_text('// check_v201_features.cpp\n#include "../v201_floor_audit/solver_prefix.hpp"\nint main(int argc,char **argv) {\n    cout<<setprecision(17);\n    for(int i=1;i<argc;i++) {\n        ifstream file(argv[i]); cin.rdbuf(file.rdbuf()); cin.clear();\n        board_info.~Board(); new (&board_info) Board{}; board_info.read();\n        const auto f=nn_selector_features();\n        cout<<argv[i]<<" "<<nn_selector_choice(f);\n        for(double x:f) cout<<" "<<x;\n        cout<<"\\n";\n    }\n}\n')
 run([compiler,*FLAGS,str(probe),'-o',str(AUDIT/'check_features')],env=env)
 rows=read(WORK/'selection_cases.json')
 output=run([str(AUDIT/'check_features'),*[str(ROOT/r['path']) for r in rows]])
 checked=[]
 for row,line in zip(rows,output.splitlines(),strict=True):
  fields=line.split();assert fields[0]==str(ROOT/row['path'])
  assert int(fields[1])==predict(selection['model'],row['features'])
  actual=list(map(float,fields[2:]));assert len(actual)==len(names)
  for n,v in zip(names,actual):assert abs(v-row['features'][n])<1e-10,(row['case'],n,v,row['features'][n])
  checked.append(dict(case=row['case'],use_nn=int(fields[1]),features=dict(zip(names,actual))))
 save(AUDIT/'feature_checks.json',checked)
 save(WORK/'solver_frozen.json',dict(source=str(source.relative_to(ROOT)),source_sha256=sha(source),selection_sha256=sha(WORK/'selection.json'),
  feature_names=names,registered_changes_sha256=sha(AUDIT/'registered_changes.json'),build_script_sha256=sha(Path(__file__)),
  binaries={str(p.relative_to(ROOT)):sha(p) for p in AUDIT.iterdir() if p.is_file() and os.access(p,os.X_OK)},features_checked=500))
 # 各葉の先頭1件を使う。最終確認用200件はこの診断にも使わない。
 examples={}
 for row in rows:
  tree=selection['model']; branch=''
  while 'feature' in tree:
   left=row['features'][tree['feature']]<=tree['threshold'];branch+='L' if left else 'R';tree=tree['left' if left else 'right']
  examples.setdefault(branch,row)
 comparisons=[]
 for mode in MODES:
  for branch,row in examples.items():
   enabled=predict(selection['model'],row['features']); chosen=PARENTS[enabled]; outputs={}
   for b in [chosen,BIN]:
    p=subprocess.run([str(AUDIT/f'fixed_{b}_{mode}')],cwd=ROOT,input=(ROOT/row['path']).read_text(),text=True,capture_output=True,check=True,timeout=60)
    out=AUDIT/f'{row["case"]}_{b}_{mode}.out';out.write_text(p.stdout);err=Path(str(out)+'.err');err.write_text(p.stderr)
    verify_output(str(ROOT/row['path']),out);c,t,errors=log_values(err);assert not errors
    audit=next(l for l in p.stderr.splitlines() if l.startswith('[selector.audit]'))
    if mode=='local':
     assert all(c[k]==0 for k in ERRORS)
     if b==BIN: assert c['selector_nn_enabled']==enabled and (c['nn_calls']>0)==bool(enabled)
    c={k:v for k,v in c.items() if not k.startswith('selector_') and (enabled or not k.startswith('nn_'))}
    t={k:v for k,v in t.items() if enabled or not k.startswith('nn_')}
    outputs[b]=(p.stdout,audit,c,t)
   assert outputs[chosen]==outputs[BIN],(mode,branch,row['case'])
   comparisons.append(dict(mode=mode,branch=branch,case=row['case'],parent=chosen,matched=True))
   print('fixed clock matched',mode,branch,row['case'],chosen,flush=True)
 save(WORK/'diagnostic.json',dict(passed=True,features=500,comparisons=comparisons))
 print('v201 source frozen; threshold unchanged from pre-holdout selection',flush=True)
if __name__=='__main__':
 import sys,shutil
 main()
