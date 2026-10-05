from concurrent.futures import ThreadPoolExecutor
import json,subprocess,difflib
from build_v133_neutral import ROOT,RUN,BASE,SOURCE,build
from check_v113_integrated import block,normalize
from check_v089_board import compile_binary,trace
from run_v105_hybrid import preprocess
from run_v089_evaluation import validated_output
from v089_data import save,sha
from v090_data import RUN as BC_RUN

def check():
    where=RUN/'mechanism';where.mkdir(parents=True,exist_ok=True)
    for local in (True,False):
        mode='local' if local else 'judge'
        compile_binary(SOURCE.stem,where/('solver_'+mode),local)
        parent=normalize(preprocess(BASE,where/('base_'+mode+'.ii'),local))
        current=normalize(preprocess(SOURCE,where/(mode+'.ii'),local))
        (where/(mode+'.patch')).write_text(''.join(difflib.unified_diff(parent.splitlines(True),current.splitlines(True))))
        key='void optimize(vector<Move>& best'
        aligned=current.replace(block(current,key),block(parent,key),1)
        assert ' '.join(aligned.split())==' '.join(parent.split()),mode
    cases=[c for c in json.loads((BC_RUN/'input_manifest.json').read_text()) if c['role']=='train'][:4]
    def one(case):
        path=BC_RUN/case['path'];assert sha(path)==case['sha256']
        proc=subprocess.run([where/'solver_local'],input=path.read_text(),text=True,capture_output=True,check=True,timeout=8)
        result=validated_output(path,proc.stdout);assert result['E']==0
        result['trace']=trace(proc.stderr);result['case']=case['index']
        (where/(str(case['index'])+'.out')).write_text(proc.stdout)
        (where/(str(case['index'])+'.err')).write_text(proc.stderr)
        return result
    with ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(one,cases))
    assert sum(r['trace'].get('neutral_best_refresh',0) for r in rows)>0
    result=dict(source_sha256=sha(SOURCE),all_legal=True,rows=rows,preprocessing_checked=True)
    save(where/'result.json',result);return result
