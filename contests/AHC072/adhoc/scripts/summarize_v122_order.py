#!/usr/bin/env python3
"""固定済みの順序学習結果を入力別に集計する。追加のsolver実行は行わない。"""
import csv
import gzip
import json
import shutil
import numpy as np
from build_v122_order import ROOT, RUN, BASE
from v089_data import save, sha, now


def load(path):return json.loads(path.read_text())


def main():
    trained=load(RUN/'training/result.json')
    assert not trained['development']['passed']
    choices=np.load(RUN/'training/development_selection.npz')
    selection={int(i):int(s) for i,s in zip(choices['indices'],choices['selected'])}
    per_case=[];index=0;common=0;common_delta=0
    for case in load(RUN/'inputs.json'):
        totals=dict(case=case['index'],groups=0,baseline_complete=0,model_complete=0,
                    mean_difference=0.,sum_difference=0,common_complete=0,common_difference=0,
                    wins=0,ties=0,losses=0,oracle_difference=0)
        with gzip.open(RUN/'data'/f"{case['index']:06d}"/'rows.jsonl.gz','rt') as stream:
            for line in stream:
                row=json.loads(line)
                if not row['extracted']:continue
                at=index;index+=1
                if at not in selection:continue
                base=min(row['costs'][0],row['costs'][row['alternate']])
                selected=min(row['costs'][0],row['costs'][selection[at]])
                difference=selected-base
                totals['groups']+=1;totals['sum_difference']+=difference
                totals['baseline_complete']+=base<=row['allowance']
                totals['model_complete']+=selected<=row['allowance']
                totals['wins']+=difference<0;totals['ties']+=difference==0;totals['losses']+=difference>0
                totals['oracle_difference']+=min(row['costs'])-base
                if max(base,selected)<=row['allowance']:
                    common+=1;common_delta+=difference
                    totals['common_complete']+=1;totals['common_difference']+=difference
        if totals['groups']:
            totals['mean_difference']=totals['sum_difference']/totals['groups'];per_case.append(totals)
    assert len(per_case)==64 and sum(row['groups'] for row in per_case)==trained['development']['groups']
    assert sum(row['sum_difference'] for row in per_case)==trained['development']['sum_difference']
    summary=dict(decision='development_gate_failed',pilot=load(RUN/'pilot/result.json'),
                 collection=load(RUN/'full/result.json'),training=trained,
                 common_completed_development=dict(groups=common,sum_difference=common_delta,mean_difference=common_delta/common),
                 development_case_wins=sum(r['sum_difference']<0 for r in per_case),
                 development_case_ties=sum(r['sum_difference']==0 for r in per_case),
                 development_case_losses=sum(r['sum_difference']>0 for r in per_case),
                 completion_score_is_proxy=True,full_solver_evaluated=False,
                 timing_gate_evaluated=False,cpp_model_numerical_evaluated=False,
                 retained_candidate=dict(source=str(BASE.relative_to(ROOT)),sha256=sha(BASE)),
                 model_sha256=sha(RUN/'training/model.json'),
                 official_source=str(RUN.relative_to(ROOT)),completed_at=now())
    save(RUN/'result.json',summary)
    save(RUN/'status.json',dict(stage='completed',decision='development_gate_failed',updated_at=now()))
    shared=ROOT/'adhoc/v122';shared.mkdir(parents=True,exist_ok=True)
    save(shared/'result_summary.json',summary)
    shutil.copy2(RUN/'training/model.json',shared/'model.json')
    with (shared/'development_by_input.csv').open('w',newline='') as stream:
        writer=csv.DictWriter(stream,fieldnames=list(per_case[0]),lineterminator='\n')
        writer.writeheader();writer.writerows(per_case)
    print(json.dumps({k:v for k,v in summary.items() if k not in ['pilot','collection','training']},ensure_ascii=False))


if __name__=='__main__':main()
