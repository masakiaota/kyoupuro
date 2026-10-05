#!/usr/bin/env python3
"""事前固定した3種類の重み供給を同じ提出探索で比較する。"""
import argparse
import fcntl
from pathlib import Path
import shutil
import time
import traceback
import torch
from v111_portfolio import ROOT, save, sha, status, now, load, with_model, numerical, evaluate, comparison, valid

RUN=ROOT/'results/nn_rank/v111/20261004_weight_portfolio_studio'
BASE=ROOT/'results/nn_rank/v105/20261004_nn_lns_studio/training/model.json'
LEARNED=ROOT/'results/nn_rank/v109/20261004_dp_reward_studio/training/model.json'
PARENT=ROOT/'src/bin/v401_incremental_cnn.cpp'


def finish(root, selected, result):
    directory=root/'final'; directory.mkdir(exist_ok=True)
    if (directory/'result.json').exists(): return load(directory/'result.json')
    if selected=='base':
        out=dict(accepted=False,retained='v401',heldout_evaluated=False,completed_at=now())
        save(directory/'result.json',out);return out
    source=ROOT/'src/bin/v111_weight_portfolio.cpp'
    parent=ROOT/result['sources'][selected]
    content='// '+source.name+'\n'+parent.read_text().split('\n',1)[1]
    if source.exists(): assert source.read_text()==content
    else: source.write_text(content)
    candidate=dict(condition=selected,source=str(source.relative_to(ROOT)),source_sha256=sha(source),
                   models=result['models'],fixed_at=result['fixed_at'])
    save(directory/'candidate.json',candidate)
    test=evaluate(root,'final',source,'test');official=evaluate(root,'final',source,'final_in')
    out=dict(accepted=True,candidate=candidate,test=test,tools_in=official,completed_at=now())
    save(directory/'result.json',out);return out


def execute(root):
    root.mkdir(parents=True,exist_ok=True)
    lock=(root/'lock').open('a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    if (root/'exit.json').exists() and load(root/'exit.json')['exit_code']==0:return
    started=time.monotonic()
    try:
        frozen=root/'frozen';frozen.mkdir(exist_ok=True)
        models={'base':sha(BASE),'learned':sha(LEARNED)}
        identity=dict(parent_sha256=sha(PARENT),models=models,conditions=['base','learned','mixed'],
                      jobs=12,local_ratio=.80,model_schedule=[0,1,0,1],
                      scripts={name:sha(ROOT/'adhoc/scripts'/name) for name in ['v111_portfolio.py','run_v111_portfolio.py']})
        if (root/'config.json').exists():assert load(root/'config.json')==identity
        else:
            save(root/'config.json',identity)
            shutil.copy2(PARENT,frozen/'v401.cpp')
            for path in [ROOT/'notes/experiments/v111.md',BASE,LEARNED]+[ROOT/'adhoc/scripts'/x for x in identity['scripts']]:
                dest=frozen/path.relative_to(ROOT);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,dest)
        sources={}
        for label,model in [('base',BASE),('learned',LEARNED),('mixed',LEARNED)]:
            status(root,'building_and_numerical',condition=label)
            source=with_model(root,label,model);sources[label]=str(source.relative_to(ROOT))
            if label=='mixed':
                numerical(root,'mixed_base',source,BASE)
                numerical(root,'mixed_learned',source,LEARNED)
            else:numerical(root,label,source,model)
        results={}
        for label in identity['conditions']:
            status(root,'evaluating',condition=label)
            results[label]=evaluate(root,label,ROOT/sources[label])
        assert valid(results['base']), 'control invalid; inspect before judging weights'
        differences={label:comparison(results[label],results['base']) for label in ['learned','mixed']}
        eligible=[label for label in ['learned','mixed'] if valid(results[label]) and
                  differences[label]['mean_T_difference']<0 and differences[label]['mean_S_difference']<0]
        selected=min(eligible,key=lambda label:(results[label]['metrics']['mean_S_all'],
                     results[label]['metrics']['mean_T_completed'],label!='learned')) if eligible else 'base'
        result=dict(selected=selected,accepted=bool(eligible),validation=results,differences=differences,
                    sources=sources,models=models,fixed_at=now())
        save(root/'assessment.json',result)
        result['final']=finish(root,selected,result);save(root/'result.json',result)
        save(root/'exit.json',dict(exit_code=0,seconds=time.monotonic()-started,completed_at=now()))
        status(root,'completed',selected=selected,accepted=bool(eligible))
    except BaseException as error:
        save(root/'exit.json',dict(exit_code=1,error=repr(error),traceback=traceback.format_exc(),completed_at=now()))
        status(root,'failed',error=repr(error));raise


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--run',type=Path,default=RUN)
    args=parser.parse_args();torch.set_num_threads(2);execute(args.run.resolve())
