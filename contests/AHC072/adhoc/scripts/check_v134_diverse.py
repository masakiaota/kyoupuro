from concurrent.futures import ThreadPoolExecutor
import json,subprocess,difflib
from build_v134_diverse import ROOT,RUN,BASE,SOURCE,build
from check_v113_integrated import block,normalize
from check_v089_board import compile_binary,trace
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save,sha,status
from v090_data import RUN as BC_RUN

def decode(path,text):
    lines=text.splitlines();rows=[];at=0
    while at<len(lines):
        header=lines[at].split();assert header[0]=='START'
        sym,diverse,T,E,steps,reorders,last=map(int,header[1:]);at+=1
        output='\n'.join(lines[at:at+T])+'\n';at+=T
        checked=validated_output(path,output);assert checked['T']==T and checked['E']==E
        assert steps<=32 and last<=32 and reorders<=steps
        if not diverse:assert steps==reorders==last==0
        rows.append(dict(sym=sym,diverse=diverse,output=output,steps=steps,reorders=reorders,last_depth=last,**checked))
    return rows

def check():
    where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True)
    marker=where/'result.json'
    if marker.exists():
        result=json.loads(marker.read_text());assert result['source_sha256']==sha(SOURCE);return result
    status(RUN/'pipeline','mechanism_builds')
    for local in (True,False):
        mode='local' if local else 'judge'
        compile_binary(SOURCE.stem,where/('solver_'+mode),local)
        parent=normalize(preprocess(BASE,where/('base_'+mode+'.ii'),local))
        current=normalize(preprocess(SOURCE,where/(mode+'.ii'),local))
        (where/(mode+'.patch')).write_text(''.join(difflib.unified_diff(parent.splitlines(True),current.splitlines(True))))
        aligned=current
        for key in ('struct SearchResult','SearchResult search(', 'static void neural_extra_starts('):
            aligned=aligned.replace(block(aligned,key),block(parent,key),1)
        assert ' '.join(aligned.split())==' '.join(parent.split()),mode
    for label,name in [('base','check_v134_parent'),('diverse','check_v134_prefix')]:
        compile_binary(name,where/('probe_'+label),True)
    cases=[c for c in json.loads((BC_RUN/'input_manifest.json').read_text()) if c['role']=='train'][:4]
    status(RUN/'pipeline','mechanism_replay')
    def one(case):
        path=BC_RUN/case['path'];assert sha(path)==case['sha256']
        decoded={}
        for label in ('base','diverse'):
            proc=subprocess.run([where/('probe_'+label)],input=path.read_text(),text=True,capture_output=True,check=True,timeout=30)
            (where/(str(case['index'])+'_'+label+'.paths')).write_text(proc.stdout)
            decoded[label]=decode(path,proc.stdout)
        original={r['sym']:r for r in decoded['base']}
        for r in decoded['diverse']:
            if not r['diverse']:assert r['output']==original[r['sym']]['output']
        proc=subprocess.run([where/'solver_local'],input=path.read_text(),text=True,capture_output=True,check=True,timeout=8)
        result=validated_output(path,proc.stdout);assert result['E']==0
        result['trace']=trace(proc.stderr);result['case']=case['index']
        assert result['trace'].get('state_pool_free_at_end')==4
        for k in ('lns_errors','construction_errors','lns_invalid_candidates','baseline_recovery','final_recovery'):assert result['trace'].get(k,0)==0
        (where/(str(case['index'])+'.out')).write_text(proc.stdout)
        (where/(str(case['index'])+'.err')).write_text(proc.stderr)
        result['sampled']=[{k:v for k,v in r.items() if k!='output'}|{'changed_from_greedy':r['output']!=original[r['sym']]['output']} for r in decoded['diverse'] if r['diverse']]
        return result
    with ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,cases))
    assert sum(r['trace'].get('diverse_prefix_reorders',0) for r in rows)>0
    result=dict(source_sha256=sha(SOURCE),all_legal=True,rows=rows,preprocessing_checked=True,
                greedy_path_matches=len(cases)*4,independently_replayed_paths=len(cases)*11)
    save(marker,result);return result
