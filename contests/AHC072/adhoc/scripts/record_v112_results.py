#!/usr/bin/env python3
"""保存済みの学習列と公式評価を集計する。solverや学習は再実行しない。"""
import csv
import hashlib
import json
from pathlib import Path
from statistics import mean

ROOT=Path(__file__).resolve().parents[2]
RUN=ROOT/'results/nn_rank/v112/20261004_lns_mean_studio'
CONTROL=ROOT/'results/nn_rank/v111/20261004_weight_portfolio_studio'
OUT=ROOT/'adhoc/v112'


def load(path):return json.loads(path.read_text())
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def write_csv(path,rows):
    with path.open('w',newline='') as stream:
        writer=csv.DictWriter(stream,fieldnames=list(rows[0]),lineterminator='\n');writer.writeheader();writer.writerows(rows)


def main():
    OUT.mkdir(exist_ok=True)
    result=load(RUN/'result.json');training=result['training'];assessment=result['assessment']
    control=load(CONTROL/'result.json')
    assert load(RUN/'pipeline/exit.json')['exit_code']==0
    assert load(RUN/'guard_pipeline/exit.json')['exit_code']==0
    assert sha(RUN/'training/latest.pt')==training['checkpoint_sha256']
    assert sha(RUN/'training/model.json')==training['model_sha256']==assessment['model_sha256']
    assert sha(ROOT/assessment['source'])==assessment['source_sha256']
    assert assessment['numerical']['passed'] and assessment['numerical']['preprocessing_outside_model_identical']
    identity=load(RUN/'pipeline/config.json')
    assert all(sha(ROOT/path)==fingerprint for path,fingerprint in identity['sources'].items())

    records=load(RUN/'learned/evaluation/validation/official_records.json')
    current=assessment['validation'];rows=current['rows']
    assert len(records)==len(rows)==256 and len({r['case_name'] for r in records})==256
    assert len({r['run_id'] for r in records})==1
    official={r['case_name']:r for r in records}
    base={r['case']:r for r in control['validation']['base']['rows']}
    selected={r['case']:r for r in control['validation'][control['selected']]['rows']}
    pairs=[]
    for row in rows:
        record=official[row['filename']]
        assert record['status']=='ok' and record['local'] and record['score']==row['S']
        assert record['elapsed']==row['elapsed_ms'] and row['E']==0
        assert row['trace']['state_pool_free_at_end']==4 and row['trace']['lns_attempts']>0
        assert all(row['trace'].get(k,0)==0 for k in ('construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery'))
        pairs.append(dict(case=row['case'],base_T=base[row['case']]['T'],v111_T=selected[row['case']]['T'],
                          v112_T=row['T'],difference_base=row['T']-base[row['case']]['T'],
                          difference_v111=row['T']-selected[row['case']]['T'],S=row['S'],E=row['E'],elapsed_ms=row['elapsed_ms']))
    assert mean(r['S'] for r in rows)==current['metrics']['mean_S_all']
    assert mean(r['difference_base'] for r in pairs)==assessment['differences']['base']['mean_T_difference']
    assert mean(r['difference_v111'] for r in pairs)==assessment['differences'][control['selected']]['mean_T_difference']
    write_csv(OUT/'paired_validation.csv',pairs)

    excluded=set(load(RUN/'training/excluded_sha256.json'));seen=set();failures=[];curve=[]
    actions=episodes=successes=lns_trials=0
    for history in training['history']:
        iteration=history['iteration'];batch=load(RUN/'training/episodes'/f'{iteration:04d}.json')
        inputs=batch['inputs'];trials=batch['rows']
        assert len(inputs)==32 and len(trials)==128
        for item in inputs:
            fingerprint=hashlib.sha256(item['text'].encode()).hexdigest()
            assert fingerprint==item['sha256'] and fingerprint not in seen and fingerprint not in excluded
            seen.add(fingerprint)
        for trial in trials:
            actions+=len(trial['actions']);episodes+=1;successes+=trial['E']==0
            assert len(trial['actions'])==trial['T']
            labels=trial['lns_results'];lns_trials+=len(labels)
            assert len(labels)==(4 if trial['E']==0 else 0)
            assert all(x['verified'] and len(x['actions'])==x['T'] for x in labels)
            if trial['E']:failures.append({k:trial[k] for k in ('case','seed','T','E','cost','group_advantage')}|dict(iteration=iteration))
        assert sum(t['E']==0 for t in trials)==history['successes']
        curve.append({k:history[k] for k in ('iteration','seconds','steps','successes','mean_T_completed',
                     'mean_lns_T_completed','mean_cost_all','mean_group_spread','entropy','approx_kl',
                     'gradient_norm','updates','learning_rate','nn_collect_seconds','lns_seconds','update_seconds')}|
                     dict(input_mean_N=mean(x['statistics']['N'] for x in inputs),
                          input_mean_M=mean(x['statistics']['M'] for x in inputs),
                          input_mean_K=mean(x['statistics']['K'] for x in inputs)))
    assert len(seen)==1024 and episodes==training['episodes']==4096
    assert successes==training['successes'] and actions==training['environment_steps']
    assert lns_trials==successes*4 and training['iterations']==32
    write_csv(OUT/'learning_curve.csv',curve)
    windows=[]
    for start in range(0,32,8):
        part=curve[start:start+8]
        windows.append(dict(first=start+1,last=start+8,**{k:mean(x[k] for x in part) for k in (
            'mean_T_completed','mean_lns_T_completed','mean_cost_all','entropy','approx_kl','gradient_norm',
            'input_mean_N','input_mean_M','input_mean_K')}))
    timing={key:sum(x[key] for x in curve) for key in ('nn_collect_seconds','lns_seconds','update_seconds')}
    retained=result['final']['retained'];assert sha(ROOT/retained['source'])==retained['source_sha256']
    summary=dict(accepted=assessment['accepted'],metrics=current['metrics'],differences=assessment['differences'],
        training={k:v for k,v in training.items() if k!='history'},unique_training_inputs=len(seen),lns_trials=lns_trials,
        optimizer_updates=sum(x['updates'] for x in curve),kl_stops=sum(x['kl_stopped'] for x in training['history']),
        timing_seconds=timing,curve_windows=windows,training_failures=failures,
        numerical=assessment['numerical'],retained=retained,heldout_evaluated=False,
        guard=load(RUN/'guard_pipeline/exit.json'),guard_gpu_separately_counted=load(RUN/'guard_pipeline/launch.json')['gpu_used'],
        hashes=dict(checkpoint=training['checkpoint_sha256'],model=training['model_sha256'],source=assessment['source_sha256']),
        completed_at=result['completed_at'])
    (OUT/'result_summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
    print(json.dumps({k:summary[k] for k in ('accepted','unique_training_inputs','lns_trials','optimizer_updates',
                     'kl_stops','timing_seconds','curve_windows','training_failures')},ensure_ascii=False,indent=2))


if __name__=='__main__':main()
