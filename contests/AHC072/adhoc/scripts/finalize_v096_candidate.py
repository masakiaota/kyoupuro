#!/usr/bin/env python3
"""固定した終了重みだけからNN候補を選び、通常100入力を一度評価する。"""
from pathlib import Path
import json
import shutil
from v096_selected import ROOT,RUN,PARENT
from v091_env import load
from v090_data import save,sha,now
from run_v092_evaluation import evaluate as evaluate_parent
from run_v096_evaluation import evaluate as evaluate_aux


def execute():
    directory=RUN/'final';directory.mkdir(parents=True,exist_ok=True);marker=directory/'result.json'
    if marker.exists():return load(marker)
    candidates=[]
    for name,root in [('v092',PARENT),('control',RUN/'control'),('mixed',RUN/'mixed')]:
        base=root/('evaluation/finish' if name=='v092' else 'evaluation')
        report=load(base/'comparison.json');validation=load(base/'validation/greedy/result.json');test=load(base/'test/greedy/result.json')
        if report['all_complete']:
            candidates.append((validation['metrics']['mean_S_all'],validation['metrics']['mean_elapsed_ms'],name,root))
    if not candidates:
        result=dict(selected=None,reason='no final checkpoint completed all fixed validation and holdout inputs',completed_at=now())
        save(marker,result);return result
    _,_,name,root=min(candidates);source=ROOT/('src/bin/v092_nn_fresh.cpp' if name=='v092' else 'adhoc/bin/v096_control.cpp' if name=='control' else 'src/bin/v096_nn_reverse_aux.cpp')
    selected=dict(name=name,source=str(source),source_sha256=sha(source),model_sha256=sha(root/'training/model.json'),rule='validation mean official score, then inference time; all validation and holdout completed')
    if (directory/'selected.json').exists():assert load(directory/'selected.json')==selected
    else:save(directory/'selected.json',selected)
    cases=[dict(index=i,filename=p.name,sha256=sha(p)) for i,p in enumerate(sorted((ROOT/'tools/in').glob('*.txt')))];assert len(cases)==100
    result=(evaluate_parent(root,'final_in','greedy',cases,'finish') if name=='v092' else evaluate_aux(root,'final_in','greedy',cases,name))
    if name=='control':
        target=ROOT/'src/bin/v096_nn_control.cpp';lines=source.read_text().splitlines();lines[0]='// '+target.name
        target.write_text('\n'.join(lines)+'\n');assert source.read_text().splitlines()[1:]==target.read_text().splitlines()[1:]
        selected['submission_source']=str(target)
    else:selected['submission_source']=str(source)
    result=dict(selected=selected,final_in=result,completed_at=now());save(marker,result);print(json.dumps({'selected':selected,'metrics':result['final_in']['metrics']}),flush=True);return result


if __name__=='__main__':execute()
