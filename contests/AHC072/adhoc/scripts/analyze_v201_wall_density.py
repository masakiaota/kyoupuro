#!/usr/bin/env python3
"""保存済みの親の結果で、壁を含む密度の単一閾値を比較する。"""
import csv
import hashlib
import json
from pathlib import Path

from analyze_v201_fresh import fit, measure, predict, relative
from v201_fresh import ROOT, WORK, PARENTS, read, save, sha

OUT = WORK / 'wall_density'
NAMES = ['floors', 'M', 'density', 'density_all_cells']


def verify_features(row):
    path = ROOT / row['path']
    assert sha(path) == row['sha256'], row['case']
    lines = path.read_text().splitlines()
    N, K = map(int, lines[0].split())
    grid = ''.join(lines[1:])
    assert len(grid) == N * N
    M = sum('a' <= c <= 'l' for c in grid)
    floors = sum(c != '#' for c in grid)
    f = row['features']
    assert (f['N'], f['K'], f['M'], f['floors']) == (N, K, M, floors)
    assert f['density'] == M / floors
    f['density_all_cells'] = M / (N * N)
    # 同じ定義を密度と壁率からも照合する。元の特徴・規則は書き換えない。
    assert abs(f['density_all_cells'] - f['density'] * (1 - f['wall_fraction'])) < 1e-12


def rule_text(model):
    if 'feature' not in model:
        return PARENTS[model['use_nn']]
    return (f"{model['feature']} <= {model['threshold']:.12g}: "
            f"{PARENTS[model['left']['use_nn']]}; else: "
            f"{PARENTS[model['right']['use_nn']]}")


def saved_set(role):
    manifest = [r for r in read(WORK / 'manifest.json') if r['role'] == role]
    datasets = [{r['case_name']: r for r in read(
        WORK / 'runs' / f'v201_fresh_{role}_{b.split("_")[0]}' / 'cases.json')}
        for b in PARENTS]
    cached = {r['case']: r['features'] for r in read(WORK / 'holdout_cases.json')}
    rows = []
    for r in manifest:
        row = dict(r, features=dict(cached[r['case']]),
                   scores=[d[r['case']]['score'] for d in datasets])
        verify_features(row)
        rows.append(row)
    return rows


def saved_hundred():
    run_ids = ['20260930T202705+0900_v076_relative_tuned_e688d5',
               '20261002T190030+0900_v079_nn_immediate_d63947']
    data = [{} for _ in PARENTS]
    with (ROOT / 'results/eval_records.jsonl').open() as stream:
        for line in stream:
            r = json.loads(line)
            if r['run_id'] in run_ids:
                i = run_ids.index(r['run_id'])
                assert r['bin'] == PARENTS[i] and r['status'] == 'ok' and r['local']
                assert r['case_name'] not in data[i]
                data[i][r['case_name']] = r['score']
    assert len(data[0]) == len(data[1]) == 100 and data[0].keys() == data[1].keys()
    rows = []
    for case in sorted(data[0]):
        path = ROOT / 'tools/in' / case
        lines = path.read_text().splitlines()
        N, K = map(int, lines[0].split())
        grid = ''.join(lines[1:])
        M = sum('a' <= c <= 'l' for c in grid)
        floors = sum(c != '#' for c in grid)
        row = dict(case=case, path=str(path.relative_to(ROOT)), sha256=sha(path),
                   scores=[d[case] for d in data],
                   features=dict(N=N, K=K, M=M, floors=floors,
                                 density=M/floors, wall_fraction=1-floors/(N*N)))
        verify_features(row)
        rows.append(row)
    return rows


def main():
    OUT.mkdir(exist_ok=False)
    protected = [ROOT/'src/bin/v201_floor_nn_selector.cpp', WORK/'selection.json',
                 WORK/'selection_cases.json', WORK/'solver_frozen.json',
                 ROOT/'results/eval_records.jsonl', ROOT/'results/score_summary.csv',
                 ROOT/'results/score_detail.csv']
    before = {str(p.relative_to(ROOT)): sha(p) for p in protected}
    old = read(WORK/'selection.json')
    rows = read(WORK/'selection_cases.json')
    assert len(rows) == 500 and sha(WORK/'selection_cases.json') == old['selection_cases_sha256']
    for row in rows:
        verify_features(row)
        assert row['gain'] == relative(min(row['scores']), row['scores'][1]) - relative(min(row['scores']), row['scores'][0])
    assert all(sum(r['fold'] == i for r in rows) == 100 for i in range(5))
    results = {}
    for name in NAMES:
        choices = {}
        folds = []
        for fold in range(5):
            train = [r for r in rows if r['fold'] != fold]
            test = [r for r in rows if r['fold'] == fold]
            model = fit(train, [name], 1, 40)
            selected = [predict(model, r['features']) for r in test]
            choices.update((r['case'], c) for r, c in zip(test, selected))
            folds.append(dict(fold=fold, model=model, result=measure(test, selected)))
        cv = measure(rows, [choices[r['case']] for r in rows])
        if name in NAMES[:-1]:
            previous = next(r['cv'] for r in old['feature_analysis'] if r['feature'] == name)
            assert cv == previous, (name, cv, previous)
        model = fit(rows, [name], 1, 50)
        results[name] = dict(cv=cv, model=model, rule=rule_text(model), folds=folds,
                             fit=measure(rows, [predict(model, r['features']) for r in rows]),
                             cv_choices=choices)
    # 全500件から決めた規則を先に保存し、その後で別集合の親の出力を選ぶ。
    save(OUT/'rules_frozen.json', dict(results=results, sources=before, script_sha256=sha(Path(__file__))))
    other_sets = {'holdout200': saved_set('holdout200'), 'tools_in100': saved_hundred()}
    for role, data in other_sets.items():
        for name in NAMES:
            model = results[name]['model']
            results[name][role] = measure(data, [predict(model, r['features']) for r in data])
    baselines = {role: {b: measure(data, [i]*len(data)) for i, b in enumerate(PARENTS)}
                 for role, data in dict(selection500=rows, **other_sets).items()}
    for p in protected:
        assert sha(p) == before[str(p.relative_to(ROOT))], p
    report = dict(formula='M / (N * N)', comparison='saved parent outputs, pair-best reference, 100-point scale',
                  results=results, baselines=baselines, solver_runs=0, solver_modified=False,
                  previous_results_reproduced=True, protected_sha256=before,
                  datasets={'selection500':500, 'holdout200':200, 'tools_in100':100},
                  separate_sets_blind=False)
    save(OUT/'comparison.json', report)
    with (OUT/'case_choices.csv').open('w', newline='') as stream:
        writer = csv.writer(stream)
        writer.writerow(['set','case','M','N','floors','density','density_all_cells',
                         'v076_score','v079_score', *[n+'_use_nn' for n in NAMES]])
        for role, data in dict(selection500=rows, **other_sets).items():
            for row in data:
                f = row['features']
                selected = [results[n]['cv_choices'][row['case']] if role == 'selection500'
                            else predict(results[n]['model'],f) for n in NAMES]
                writer.writerow([role,row['case'],*[f[n] for n in ['M','N','floors','density','density_all_cells']],
                                 *row['scores'],*selected])
    print(json.dumps({n:{k:v for k,v in r.items() if k not in ['folds','cv_choices']}
                      for n,r in results.items()}, ensure_ascii=False, indent=2))
    print('new density folds:', [r['model'] for r in results['density_all_cells']['folds']])
    print('baselines:', json.dumps(baselines, ensure_ascii=False))


if __name__ == '__main__':
    main()
