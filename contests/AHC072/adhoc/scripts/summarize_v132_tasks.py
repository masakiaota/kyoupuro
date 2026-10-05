#!/usr/bin/env python3
"""保存済みの共同探索診断を集計する。探索は実行しない。"""
import csv
import json
import shutil
from pathlib import Path
from build_v132_tasks import ROOT, RUN, OUT, SOURCE
from run_v132_tasks import save, sha


def main():
    assert json.loads((RUN / 'status.json').read_text())['phase'] == 'complete'
    rows = list(csv.DictReader((RUN / 'trials.csv').open()))
    prior = {r['tag']: r for r in csv.DictReader((ROOT / 'results/analysis/v044_joint_temporal.csv').open())}
    main_rows = [r for r in rows if r['control'] == '0']
    controls = [r for r in rows if r['control'] == '1']
    assert len(main_rows) == 20 and len(controls) == 1
    verification = json.loads((OUT / 'verification.json').read_text())
    mechanisms = list(csv.DictReader((OUT / 'task_admissions.csv').open()))
    control = controls[0]
    improvements = [{k: r[k] for k in ('tag', 'case', 'boundary', 'base_tail', 'best_tail', 'saved', 'ms', 'labels', 'expanded', 'rides', 'supports', 'skipped', 'artifact')}
                    for r in rows if int(r['saved']) > 0]
    statuses = {name: sum(r['status'] == name for r in main_rows) for name in sorted({r['status'] for r in main_rows})}
    result = dict(
        experiment='v132', parent='v044', submitted_candidate='v113', submission_changed=False,
        scope='saved_boundary_diagnostic', source_sha256=sha(SOURCE),
        main_trials=len(main_rows), improved_main_trials=sum(int(r['saved']) > 0 for r in main_rows),
        statuses=statuses, improvements=improvements,
        known_nine_move_found=int(control['best_tail']) <= 9,
        known_control=control, known_control_parent=prior[control['tag']],
        represented_known_trace=mechanisms,
        total_search_sec=sum(float(r['ms']) for r in rows) / 1000,
        maximum_search_sec=max(float(r['ms']) for r in rows) / 1000,
        total_labels=sum(int(r['labels']) for r in rows),
        total_expanded=sum(int(r['expanded']) for r in rows),
        total_macro_edges=sum(int(r['multi_edges']) for r in rows),
        total_background_edges=sum(int(r['background_edges']) for r in rows),
        max_task_length=max(int(r['maximum_task_length']) for r in rows),
        verification=verification,
        limits=['節目集合と最短経路を限定しており、原始操作の全探索ではない。',
                '保存された同一入力の複数境界は独立した入力数として数えない。',
                'v044と現在の実行環境の時間差は、アルゴリズムの速度倍率として解釈しない。',
                '2秒の完成solverへの組み込みと、固定時間での絶対スコア改善は未評価。'])
    save(RUN / 'result.json', result)
    shared = ROOT / 'adhoc/v132'
    save(shared / 'result_summary.json', result)
    for filename in ('trials.csv', 'build_verification.json'):
        shutil.copy2(RUN / filename, shared / filename)
    for filename in ('settings.json', 'sources_before.json', 'task_admissions.csv', 'fixtures.csv', 'verification.json'):
        shutil.copy2(OUT / filename, shared / filename)
    for item in verification['plans']:
        path = next(r['artifact'] for r in rows if r['tag'] == item['tag'])
        target = shared / 'plans' / Path(path).name
        target.parent.mkdir(exist_ok=True)
        shutil.copy2(ROOT / path, target)
    print(json.dumps({k: result[k] for k in ('main_trials', 'improved_main_trials', 'statuses', 'known_nine_move_found', 'total_search_sec', 'maximum_search_sec', 'total_labels', 'total_expanded', 'total_macro_edges', 'improvements')}, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
