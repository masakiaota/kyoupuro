#!/usr/bin/env python3
"""保存済みの計画選別診断を集計し、採否と再利用する提出候補を記録する。"""
import csv
import json
import shutil
from statistics import mean

from build_v129_allocation import ROOT, RUN, BASE, BASE_SHA
from v089_data import save, sha, now


def load(path):
    return json.loads(path.read_text())


def write_csv(path, rows):
    with path.open('w', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), lineterminator='\n')
        writer.writeheader()
        writer.writerows(rows)


def main():
    result = load(RUN / 'result.json')
    diagnostic = load(RUN / 'training/result.json')
    assert result['decision'] == 'diagnostic_failed' and not diagnostic['passed']
    assert sha(BASE) == BASE_SHA
    frozen = load(RUN / 'pipeline_config.json')
    for name, expected in frozen['scripts'].items():
        assert sha(ROOT / 'adhoc/scripts' / name) == expected, name
    collection = load(RUN / 'collection_config.json')
    assert sha(ROOT / 'adhoc/bin/collect_v129_allocation.cpp') == collection['source_sha256']
    assert sha(RUN / 'input_manifest.json') == collection['inputs_sha256']
    reports = [load(path) for path in sorted((RUN / 'data').glob('*/129101/result.json'))]
    assert len(reports) == 512
    assert all(r['parent_result']['complete'] and r['parent_result']['E'] == 0 for r in reports)
    assert sum(r['rows'] for r in reports) == diagnostic['quality']['measured_completions']
    assert max(r['max_feature_error'] for r in reports) <= 2e-5
    assert load(RUN / 'pipeline/exit.json')['exit_code'] == 0
    guard = load(RUN / 'guard_pipeline/exit.json')
    assert guard['exit_code'] == 0 and guard['child_exit_code'] == 0
    previous = ROOT / 'results/nn_rank/v128/20261004_capacity_allocation_studio/result.json'
    retained_in = load(previous)['retained_tools_in']
    assert retained_in['source_sha256'] == BASE_SHA
    aggregates = {}
    for arm, values in diagnostic['arms'].items():
        aggregates[arm] = {
            'training_teacher_difference': mean(r['teacher_difference'] for r in values['training']),
            'training_other_seed_difference': mean(r['other_seed_difference'] for r in values['training']),
            'training_seconds': sum(r['seconds'] for r in values['training']),
            'improved_folds': sum(x < 0 for x in values['fold_differences']),
        }
    summary = dict(
        decision=result['decision'], diagnostic=diagnostic, training_aggregates=aggregates,
        collection=dict(inputs=512, complete=512,
                        cpp_replays=sum(r['cpp_replays'] for r in reports),
                        python_replays=sum(r['python_replays'] for r in reports),
                        python_prefix_replays=sum(r['python_prefix_replays'] for r in reports),
                        max_feature_error=max(r['max_feature_error'] for r in reports)),
        guard=guard, mechanism=load(RUN / 'mechanism/result.json'),
        loss_check=load(RUN / 'loss_check/result.json'),
        retained=dict(version='v113', source=str(BASE.relative_to(ROOT)), source_sha256=BASE_SHA),
        retained_tools_in=retained_in, validation1_opened=False,
        learned_cpp_evaluated=False, solver_cross_validation_executed=False,
        new_256_evaluated=False, final_in_repeated=False,
        completed_at=result['completed_at'], summarized_at=now())
    save(RUN / 'summary.json', summary)
    shared = ROOT / 'adhoc/v129'
    shared.mkdir(exist_ok=True)
    save(shared / 'result_summary.json', summary)
    shutil.copy2(RUN / 'training/config.json', shared / 'training_config.json')
    curves = []
    for arm, prefix in [('control_zero', 'control_fold_'), ('structure', 'fold_')]:
        write_csv(shared / f'{arm}_diagnostic_cases.csv',
                  load(RUN / 'training' / f'{arm}_diagnostic_cases.json'))
        for fold in range(4):
            directory = RUN / 'training' / f'{prefix}{fold}'
            model = load(directory / 'model.json')
            assert model['epochs'] == 500 and model['architecture'] == [42, 64, 64, 1]
            assert model['checkpoint_sha256'] == sha(directory / 'latest.pt')
            history = load(directory / 'history.json')
            assert len(history) == 500 and history[-1]['epoch'] == 500
            curves.extend(dict(arm=arm, fold=fold, **row) for row in history)
            shutil.copy2(directory / 'model.json', shared / f'{prefix}{fold}.json')
    write_csv(shared / 'training_curves.csv', curves)
    write_csv(shared / 'collection_cases.csv', [dict(
        case=r['case'], rows=r['rows'], complete=r['parent_result']['complete'],
        T=r['parent_result']['T'], E=r['parent_result']['E'], S=r['parent_result']['S'],
        max_feature_error=r['max_feature_error'], seconds=r['seconds']) for r in reports])
    print(json.dumps(dict(collection=summary['collection'], training_aggregates=aggregates,
                          retained='v113'), ensure_ascii=False))


if __name__ == '__main__':
    main()
