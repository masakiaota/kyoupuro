#!/usr/bin/env python3
"""保存済みの比較だけを集計する。solverの追加実行は行わない。"""
import csv
import json
from pathlib import Path
import re

import numpy as np
from build_v113_integrated import ROOT, RUN, CONDITIONS
from v089_data import save, sha, now


def load(path):
    return json.loads(path.read_text())


def paired(current,before):
    old={r['case']:r for r in before['rows']}
    delta=np.array([r['S']-old[r['case']]['S'] for r in current['rows']],dtype=float)
    rng=np.random.default_rng(113004)
    means=delta[rng.integers(0,len(delta),size=(10000,len(delta)))].mean(axis=1)
    return dict(mean_difference=float(delta.mean()),total_difference=int(delta.sum()),
                interval95=np.quantile(means,[.025,.975]).tolist(),
                wins=int((delta<0).sum()),ties=int((delta==0).sum()),losses=int((delta>0).sum()),
                control_relative=100*float(np.mean([old[r['case']]['S']/r['S'] for r in current['rows']])))


def main():
    result=load(RUN/'result.json');a=result['assessment'];final=result['final']
    out=ROOT/'adhoc/v113';out.mkdir(exist_ok=True)
    controls=ROOT/'results/nn_rank/v111/20261004_weight_portfolio_studio'
    oldfinal=load(controls/'final/result.json')
    base=a['validation']['base'];summary={}
    for key,value in a['validation'].items():
        summary[key]=dict(metrics=value['metrics'],source_sha256=value['source_sha256'],
                          vs_v111=paired(value,base),activation=a['activation'].get(key,{}))
        if key!='base':
            matched=0;retained=0
            for row in value['rows']:
                name=row['filename']
                old=(controls/'learned/evaluation/validation/outputs'/(name+'.err')).read_text()
                new=(RUN/key/'evaluation/validation/outputs'/(name+'.err')).read_text()
                # uint64のハッシュを浮動小数点へ変換しない。
                field=lambda text,k:re.search(r'\b'+k+r'=(\d+)',text).group(1)
                matched+=field(old,'v311_nn_first_hash')==field(new,'v311_nn_first_hash')
                retained+=field(old,'v311_nn_retained')==field(new,'v311_nn_retained')
            summary[key]['initial_first_hash_matched']=matched
            summary[key]['retained_count_matched']=retained
    finalsummary=dict(candidate=final['candidate'],accepted=final['accepted'],
                      validation=summary[a['selected']],test=dict(metrics=final['test']['metrics'],vs_v111=paired(final['test'],oldfinal['test'])),
                      tools_in=dict(metrics=final['tools_in']['metrics'],vs_v111=paired(final['tools_in'],oldfinal['tools_in'])))
    save(out/'result_summary.json',dict(selected=a['selected'],conditions=summary,final=finalsummary,
         mechanism=a['mechanism'],completed_at=result['completed_at'],analysis_at=now(),
         limits=['各入力各条件1回の時間依存探索。入力再標本化の区間は再実行ノイズを含まない。',
                 '7条件から選んだ検証上の最良差には選別の影響がある。',
                 '保留とinは過去測定済みの固定比較集合。今回の選別には使っていない。']))
    save(out/'source_manifest.json',load(RUN/'sources.json'))
    for role,current,old in [('validation',a['validation'][a['selected']],base),('test',final['test'],oldfinal['test']),('in',final['tools_in'],oldfinal['tools_in'])]:
        before={r['case']:r for r in old['rows']}
        with (out/(role+'_paired.csv')).open('w',newline='') as stream:
            writer=csv.writer(stream,lineterminator='\n');writer.writerow(['case','v111_T','selected_T','difference','E','official_S','elapsed_ms'])
            for row in current['rows']:
                writer.writerow([row['filename'],before[row['case']]['T'],row['T'],row['T']-before[row['case']]['T'],row['E'],row['S'],row['elapsed_ms']])
    with (out/'validation_conditions.csv').open('w',newline='') as stream:
        writer=csv.writer(stream,lineterminator='\n');writer.writerow(['case','v111']+CONDITIONS)
        tables={k:{r['case']:r['S'] for r in v['rows']} for k,v in a['validation'].items()}
        for row in base['rows']:writer.writerow([row['filename'],row['S']]+[tables[k][row['case']] for k in CONDITIONS])
    print(json.dumps(dict(selected=a['selected'],conditions={k:v['vs_v111'] for k,v in summary.items()},final=finalsummary),ensure_ascii=False,indent=2))


if __name__=='__main__':
    main()
