#!/usr/bin/env python3
"""学習済み選択器の診断を保存し、失敗した条件で後続評価を実行しない。"""
import csv
import json
import shutil

from train_v128_allocation import ROOT, RUN, PREVIOUS, BASE, BASE_SHA, load
from v089_data import save, sha, now


def main():
    result = load(RUN / 'training/result.json')
    assert not result['passed'], 'a passed diagnosis requires the preregistered C++ evaluation'
    config = load(RUN / 'training/config.json')
    assert sha(BASE) == BASE_SHA
    assert sha(ROOT / 'adhoc/scripts/train_v128_allocation.py') == config['source']
    assert sha(ROOT / 'adhoc/scripts/train_v126_allocation.py') == config['loss_source']
    for name in ['training/averaged_labels.json', 'training/folds.json',
                 'training/data_identity.json', 'input_manifest.json']:
        assert sha(PREVIOUS / name) == config[name]
    # v113の最終評価は再実行せず、固定済みの記録を再利用する。
    saved_in = load(PREVIOUS / 'summary.json')['retained_tools_in']
    assert saved_in['source_sha256'] == BASE_SHA
    summary = dict(decision='diagnostic_failed', diagnostic=result,
                   mechanism=load(RUN / 'mechanism/result.json'),
                   training_config_sha256=sha(RUN / 'training/config.json'),
                   retained=dict(version='v113', source=str(BASE.relative_to(ROOT)), source_sha256=BASE_SHA),
                   retained_tools_in=saved_in, validation1_opened=False,
                   solver_cross_validation_executed=False, new_256_evaluated=False,
                   final_in_repeated=False, completed_at=result['completed_at'], summarized_at=now())
    save(RUN / 'result.json', summary)
    shared = ROOT / 'adhoc/v128'
    shared.mkdir(parents=True, exist_ok=True)
    save(shared / 'result_summary.json', summary)
    shutil.copy2(RUN / 'training/config.json', shared / 'training_config.json')
    comparisons = load(RUN / 'training/diagnostic_cases.json')
    with (shared / 'diagnostic_cases.csv').open('w', newline='') as stream:
        writer = csv.DictWriter(stream, fieldnames=list(comparisons[0]), lineterminator='\n')
        writer.writeheader()
        writer.writerows(comparisons)
    for fold in range(4):
        directory = RUN / 'training' / f'fold_{fold}'
        model = load(directory / 'model.json')
        assert model['epochs'] == 500 and model['architecture'] == [18, 64, 64, 1]
        assert model['checkpoint_sha256'] == sha(directory / 'latest.pt')
        shutil.copy2(directory / 'model.json', shared / f'fold_{fold}.json')
    print(json.dumps(dict(decision=summary['decision'], mean_difference=result['mean_difference'],
                          parameters=result['parameters'], retained='v113'), ensure_ascii=False))


if __name__ == '__main__':
    main()
