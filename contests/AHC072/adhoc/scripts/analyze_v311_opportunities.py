#!/usr/bin/env python3
"""保存済み600件からv311の構築、選抜、探索を分析する。solverは実行しない。"""
import csv
import hashlib
import json
from collections import Counter, defaultdict
from pathlib import Path
import random
import re
import statistics

from check_v028_two_orders import validate

ROOT = Path(__file__).resolve().parents[2]
RUN = ROOT / 'results/analysis/v311_v312/20261004T123011'
OLD = ROOT / 'results/analysis/v105/20261004T100932_local_eval'
OUT = ROOT / 'results/analysis/v311_opportunities_20261004'


def logs(path):
    s = path.read_text()
    counts = {k: int(v) for k, v in re.findall(r'^\[summary.count\] (\S+)=(-?\d+)$', s, re.M)}
    times = {k: float(v) for k, v in re.findall(r'^\[summary.time_ms\] (\S+)=([\d.]+)$', s, re.M)}
    detail = {}
    for k, v in re.findall(r'\b(v311_\w+)=(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)', s):
        detail[k] = float(v) if any(ch in v for ch in '.eE') else int(v)
    return counts, times, detail


def percentile(values, q):
    a = sorted(values)
    t = (len(a)-1)*q
    lo = int(t)
    return a[lo] + (a[min(lo+1, len(a)-1)]-a[lo])*(t-lo)


def distribution(values):
    return dict(mean=statistics.mean(values), median=statistics.median(values),
                p10=percentile(values, .1), p90=percentile(values, .9), max=max(values))


def compare(rows, field):
    delta = [r['score']-r[field] for r in rows]
    return dict(cases=len(rows), delta_sum=sum(delta), delta_mean=statistics.mean(delta),
                wins=sum(d<0 for d in delta), draws=delta.count(0), losses=sum(d>0 for d in delta),
                relative_delta=statistics.mean(100*r['reference']*(1/r['score']-1/r[field]) for r in rows))


def load_set(eval_set, manifest, records):
    name = 'in' if eval_set == 'in' else 'validation1'
    cfg = manifest['stages'][f'v311_{name}']
    old_label = f'v105_{"in_j2" if eval_set=="in" else "validation1_j1"}_20261004T100932'
    labels = {'score': cfg['label'], 'v105': old_label,
              'v312': manifest['stages'][f'v312_{name}']['label'],
              'v214': 'v214_local_color_runs_flexible_paired_same_budget' if eval_set=='in'
              else 'validation1_j1_v208_v214_20261004T021012_744383'}
    by = {}
    for key,label in labels.items():
        selected = [r for r in records if r.get('label')==label and (key!='v214' or r['bin']=='v214_local_color_runs')]
        assert all(r['status']=='ok' and r['local'] and r['input_dir']==cfg['input_dir'] for r in selected)
        by[key] = {r['case_name']:r for r in selected}
        assert len(selected)==len(by[key]), (key,'duplicate records')
    assert len(by['score']) == (100 if eval_set=='in' else 500)
    assert all(by[key].keys()==by['score'].keys() for key in by)
    rows = []
    for case, record in sorted(by['score'].items()):
        inp = ROOT / cfg['input_dir'] / case
        lines = inp.read_text().splitlines();N,K=map(int,lines[0].split());grid=''.join(lines[1:N+1])
        M=sum('a'<=c<='l' for c in grid);F=sum(c!='#' for c in grid)
        output=RUN / f'v311_{name}_outputs' / case
        moves=[]
        for line in output.read_text().splitlines():
            i,j,k,d,l=line.split();moves.append((int(i),int(j),int(k),d,int(l)))
        validate(str(inp),moves)
        c,t,x=logs(output.with_suffix('.txt.err'))
        old_output=OLD / ('in_outputs' if eval_set=='in' else 'validation_outputs') / case
        oc,ot,_=logs(old_output.with_suffix('.txt.err'))
        assert record['status']=='ok' and c['E']==0 and c['final_ops']==record['score']==len(moves)
        assert sum(bool(line.strip()) for line in old_output.read_text().splitlines())==by['v105'][case]['score']==oc['final_ops']
        assert all(c[k]==0 for k in ['construction_errors','lns_errors','lns_invalid_candidates','baseline_recovery','final_recovery'])
        count=x['v311_nn_retained'];winner=x['v311_winner'];seeds=[]
        for i in range(count):
            prefix=f'v311_seed_{i}_'
            seed={key:x[prefix+key] for key in ['origin','raw','pilot','final']}
            seed.update(id=i,round0=c.get(f'race_round_0_seed_{i}_ops'),round1=c.get(f'race_round_1_seed_{i}_ops'))
            assert seed['raw']>=seed['pilot']>=seed['final']
            seeds.append(seed)
        assert seeds[0]['raw']==c['pre_lns_ops'] and min(s['final'] for s in seeds)==c['pre_pair_ops']
        r=dict(eval_set=eval_set,case=case,N=N,K=K,M=M,floor_cells=F,slimes_per_floor=M/F,
               input_sha256=hashlib.sha256(inp.read_bytes()).hexdigest(),
               score=record['score'],v105=by['v105'][case]['score'],v312=by['v312'][case]['score'],v214=by['v214'][case]['score'],
               reference=manifest['reference'][case] if eval_set=='in' else min(z[case]['score'] for z in by.values()),
               elapsed_ms=record['elapsed'],seed_count=count,winner=winner,winner_origin=seeds[winner]['origin'],
               first_ops=x['v311_nn_first_ops'],raw_best=seeds[0]['raw'],winner_raw=seeds[winner]['raw'],
               winner_pilot=seeds[winner]['pilot'],winner_final=seeds[winner]['final'],
               first_ms=t['nn_initial'],old_first_ms=ot['nn_initial'],construction_ms=t['construction'],
               extra_ms=x['v311_nn_extra_seconds']*1000,lns_ms=t['temporal_lns'],old_lns_ms=ot['temporal_lns'],
               lns_attempts=c['lns_attempts'],old_lns_attempts=oc['lns_attempts'],
               inferences=c['inferences'],duplicates=c['duplicates'],incomplete=x['v311_nn_incomplete'],
               first_hash_equal=c['nn_initial_hash']==oc['nn_initial_hash'],first_ops_equal=x['v311_nn_first_ops']==oc['nn_initial_ops'],
               summary_T_factor=c['T']/record['score'],
               stage0_ms=t.get('race_round_0',0),stage1_ms=t.get('race_round_1',0),stage2_ms=t['race_round_2'],
               stage0_attempts=sum(v for k,v in c.items() if re.fullmatch(r'race_round_0_seed_\d+_attempts',k)),
               stage1_attempts=sum(v for k,v in c.items() if re.fullmatch(r'race_round_1_seed_\d+_attempts',k)),
               stage2_attempts=sum(v for k,v in c.items() if re.fullmatch(r'race_round_2_seed_\d+_attempts',k)),
               seeds=seeds,counts=c,times=t,old_counts=oc,old_times=ot)
        if count>1:
            stage0=sorted(seeds,key=lambda s:(s['round0'],s['id']))
            survivors=[s for s in seeds if s['round1'] is not None]
            assert len(survivors)==((count+1)//2 if count>2 else 0)
            if survivors:
                assert {s['id'] for s in survivors}=={s['id'] for s in stage0[:len(survivors)]}
            else:
                assert winner==stage0[0]['id']
            r.update(round0_leader=stage0[0]['id'],round0_margin=stage0[1]['round0']-stage0[0]['round0'],
                     round1_margin=max(s['round1'] for s in survivors)-min(s['round1'] for s in survivors) if survivors else None,
                     round1_changed_winner=stage0[0]['id']!=winner)
        rows.append(r)
    return rows


def aggregate(rows):
    multi=[r for r in rows if r['seed_count']>1]
    four=[r for r in rows if r['seed_count']==4]
    stages={f'stage{i}':dict(time_ms=statistics.mean(r[f'stage{i}_ms'] for r in rows),
                           attempts=statistics.mean(r[f'stage{i}_attempts'] for r in rows)) for i in range(3)}
    origin=defaultdict(lambda:Counter())
    for r in rows:
        for s in r['seeds']:
            o=origin[s['origin']];o['retained']+=1;o['raw_sum']+=s['raw'];o['round0_sum']+=s['round0'] if s['round0'] is not None else s['raw']
            o['winner']+=s['id']==r['winner'];o['raw_best']+=s['id']==0
    result=dict(cases=len(rows),total=sum(r['score'] for r in rows),mean=statistics.mean(r['score'] for r in rows),
        comparisons={k:compare(rows,k) for k in ['v105','v312','v214']},
        first_hash_equal=sum(r['first_hash_equal'] for r in rows),first_ops_equal=sum(r['first_ops_equal'] for r in rows),
        seed_counts=dict(Counter(r['seed_count'] for r in rows)),incomplete_seeds=sum(r['incomplete'] for r in rows),
        transformed_winner=sum(r['winner_origin']!=-105 for r in rows),nonshortest_initial_winner=sum(r['winner']>0 for r in rows),
        winner_ranks=dict(Counter(r['winner'] for r in rows)),origins=dict(origin),stages=stages,
        initial_gain_from_extra=distribution([r['first_ops']-r['raw_best'] for r in rows]),
        winner_raw_penalty=distribution([r['winner_raw']-r['raw_best'] for r in rows]),
        final_stage_gain=distribution([r['winner_pilot']-r['winner_final'] for r in rows]),
        final_stage_zero_gain=sum(r['winner_pilot']==r['winner_final'] for r in rows),
        final_stage_at_most_2=sum(r['winner_pilot']-r['winner_final']<=2 for r in rows),
        round1_changed_winner=sum(r['round1_changed_winner'] for r in multi),
        round0_ties=sum(r['round0_margin']==0 for r in multi),round1_ties=sum(r['round1_margin']==0 for r in multi),
        round1_cases=sum(r['round1_margin'] is not None for r in multi),
        near_round1=sum(r['round1_margin'] is not None and r['round1_margin']<=2 for r in multi),
        low_cost_four_seeds=sum(r['seed_count']==4 and r['construction_ms']<152 for r in rows),
        # 次の向きの実時間は未記録なので、元向きの実測時間を使った概算。
        # 実装が使う直前の向きの時間とは異なり、スコア改善も保証しない。
        fifth_estimated_within_current_deadline=sum(r['seed_count']==4 and r['construction_ms']+1.35*r['first_ms']<456 for r in rows),
        nn_duplicate_ratio=sum(r['duplicates'] for r in rows)/sum(r['inferences'] for r in rows),
        nn_duplicates=sum(r['duplicates'] for r in rows),nn_inferences=sum(r['inferences'] for r in rows),
        nn_cases_with_duplicates=sum(r['duplicates']>0 for r in rows),
        summary_T_factors=dict(Counter(r['summary_T_factor'] for r in rows)))
    result['four_seed_cases'] = dict(cases=len(four),winners=dict(Counter(r['winner_origin'] for r in four)),
        final_stage_gain=distribution([r['winner_pilot']-r['winner_final'] for r in four]),
        origin_means={str(origin):{key:statistics.mean(next(s[key] for s in r['seeds'] if s['origin']==origin) for r in four)
                       for key in ['raw','round0']} for origin in [-105,-3114,-3111,-3115]})
    for key in ['first_ms','old_first_ms','construction_ms','extra_ms','lns_ms','old_lns_ms','lns_attempts','old_lns_attempts']:
        result[key]=distribution([r[key] for r in rows])
    groups={}
    for feature, bins in [('M',[0,40,80,120,1000]),('floor_cells',[0,150,250,401]),('seed_count',[1,2,3,4,5])]:
        for lo,hi in zip(bins,bins[1:]):
            subset=[r for r in rows if lo<=r[feature]<hi]
            if subset:groups[f'{feature}:{lo}-{hi}']={**compare(subset,'v105'),
                'delta_vs_v312':sum(r['score']-r['v312'] for r in subset),
                'construction_ms':statistics.mean(r['construction_ms'] for r in subset)}
    result['groups']=groups
    # 個別処理の時間には包含関係がある。合算して全体時間や独立効果にしない。
    result['mean_times']={key:statistics.mean(r['times'].get(key,0) for r in rows)
                         for key in sorted(set().union(*(r['times'] for r in rows)))}
    return result


def selector_check(train, test):
    # In100だけで1つの閾値を選び、保存validation500へそのまま適用する。
    # 選んだ保存スコアの比較であり、統合実行の成績を保証しない。
    features=['M','N','K','floor_cells','slimes_per_floor']
    candidates=[dict(feature='always_v311',threshold=0,upper=True,train_delta=0)]
    for feature in features:
        values=sorted({r[feature] for r in train})
        for cutoff in [(a+b)/2 for a,b in zip(values,values[1:])]:
            for upper in [True,False]:
                selected=[r for r in train if (r[feature]>=cutoff)==upper]
                if len(selected)<10:continue
                delta=sum(r['v312']-r['score'] for r in selected)
                candidates.append(dict(feature=feature,threshold=cutoff,upper=upper,train_delta=delta))
    best=min(candidates,key=lambda x:x['train_delta'])
    selected=[r for r in test if best['feature']!='always_v311' and (r[best['feature']]>=best['threshold'])==best['upper']]
    return dict(selected_on_in100=best,validation_selected=len(selected),
        validation_delta_sum=sum(r['v312']-r['score'] for r in selected),
        validation_relative_delta=sum(100*r['reference']*(1/r['v312']-1/r['score']) for r in selected)/len(test),
        validation_oracle_gain=sum(max(0,r['score']-r['v312']) for r in test))


def main():
    OUT.mkdir(parents=True,exist_ok=True)
    manifest=json.loads((RUN/'manifest.json').read_text())
    source=ROOT/'src/bin/v311_neural_multistart.cpp'
    assert hashlib.sha256(source.read_bytes()).hexdigest()==manifest['stages']['v311_in']['source_sha256']
    labels={x['label'] for x in manifest['stages'].values()}|{'v105_in_j2_20261004T100932','v105_validation1_j1_20261004T100932',
        'v214_local_color_runs_flexible_paired_same_budget','validation1_j1_v208_v214_20261004T021012_744383'}
    records=[]
    for line in (ROOT/'results/eval_records.jsonl').open():
        if any(label in line for label in labels):records.append(json.loads(line))
    train=load_set('in',manifest,records);test=load_set('validation1',manifest,records)
    result=dict(source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),solver_executions=0,
                verification={'independent_replay_completed_cases':len(train)+len(test),
                              'official_scores_match_output_moves':True,'v105_saved_output_counts_match':True},
                relative_references={'in':manifest['reference_origin'],'validation1':'case minima of v311/v105/v214/v312'},
                evaluation_conditions={'local':True,'local_time_ratio':0.8,'in_jobs':2,'validation1_jobs':1},
                source_manifests=[str(RUN.relative_to(ROOT)/'manifest.json'),str(OLD.relative_to(ROOT)/'manifest.json')],
                inputs={'in':100,'validation1':500},sets={'in':aggregate(train),'validation1':aggregate(test)},
                saved_score_selector=selector_check(train,test))
    for name,group in result['sets'].items():
        previous=json.loads((RUN/f'v311_{name}_result.json').read_text())
        assert group['cases']==previous['cases'] and group['total']==previous['sum']
        for baseline,values in previous['comparisons'].items():
            assert group['comparisons'][baseline]['delta_sum']==values['delta_sum']
            assert abs(group['comparisons'][baseline]['relative_delta']-values['relative_delta_points'])<1e-9
    result['verification']['existing_result_summaries_match']=True
    # 既存の実行結果からケースを再標本化する。solverの再実行によるノイズ測定ではない。
    rng=random.Random(311)
    for name,rows in [('in',train),('validation1',test)]:
        deltas=[r['score']-r['v105'] for r in rows]
        samples=[sum(rng.choices(deltas,k=len(deltas)))/len(deltas) for _ in range(3000)]
        result['sets'][name]['paired_case_bootstrap_mean_delta_95']=[percentile(samples,.025),percentile(samples,.975)]
    (OUT/'summary.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    (OUT/'cases.json').write_text(json.dumps(train+test,ensure_ascii=False,indent=2)+'\n')
    flat=[{k:v for k,v in r.items() if not isinstance(v,(dict,list))} for r in train+test]
    with (OUT/'cases.csv').open('w',newline='') as out:
        columns=sorted(set().union(*(r.keys() for r in flat)));writer=csv.DictWriter(out,fieldnames=columns)
        writer.writeheader();writer.writerows(flat)
    for name,group in result['sets'].items():
        print(name,json.dumps({k:v for k,v in group.items() if k not in ['mean_times','groups']},ensure_ascii=False))
    print('selector',json.dumps(result['saved_score_selector'],ensure_ascii=False))


if __name__=='__main__':main()
