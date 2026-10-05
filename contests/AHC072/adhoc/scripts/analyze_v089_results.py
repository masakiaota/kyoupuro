#!/usr/bin/env python3
"""保存済みの出力だけでv089の結果と操作の傾向を整理する。solverは実行しない。"""
import argparse
import json
from pathlib import Path

import numpy as np

from replay_slime_output import replay
from run_v089_evaluation import statistics
from v089_data import ROOT, RUN, SOURCE, save, sha, now


def analyze(run):
    evaluation = run / 'evaluation'
    comparison = json.loads((evaluation / 'comparison.json').read_text())
    config = json.loads((evaluation / 'config.json').read_text())
    cases = json.loads((evaluation / 'input_manifest.json').read_text())
    data = json.loads((run / 'dataset.json').read_text())
    training = {c['sha256'] for c in data['cases'] if c['role'] == 'train'}
    assert len(cases) == 128 and all(c['sha256'] not in training for c in cases)
    assert config['jobs'] == 20 and config['local']
    assert sha(run / 'model.json') == config['model_sha256']
    assert sha(ROOT / 'src/bin/v089_nn_board.cpp') == config['solver_sha256']
    assert sha(evaluation / 'frozen/v089_nn_board.cpp') == config['solver_sha256']
    learning = json.loads((run / 'result.json').read_text())
    assert learning['final']['epoch'] == 40 and learning['final']['steps'] == 52120
    assert sha(run / 'latest.pt') == learning['checkpoint_sha256']
    rows = {}
    profiles = {'teacher': {}, 'greedy': {}, 'beam4': {}}
    for condition in ('greedy', 'beam4'):
        rows[condition] = json.loads((evaluation / f'cases_{condition}.json').read_text())
        records = json.loads((evaluation / f'records_{condition}.json').read_text())
        assert len(rows[condition]) == len(records) == 128
        assert len({r['run_id'] for r in records}) == 1
        official = {int(Path(r['case_name']).stem): r for r in records}
        assert len(official) == 128
        for row in rows[condition]:
            record = official[row['case']]
            assert record['status'] == 'ok' and record['local'] and row['S'] == record['score']
            assert row['S'] == row['T'] + 100000 * row['E']
            assert row['elapsed_ms'] == record['elapsed']
            output = evaluation / 'outputs' / condition / f"{row['case']:06d}.txt"
            metrics = replay(evaluation / 'inputs' / output.name, output)['metrics']
            assert all(metrics[k] == row[k] for k in ('T', 'E', 'S'))
            profiles[condition][row['case']] = metrics
        assert statistics(rows[condition]) == comparison['initial'][condition]
    for case in cases:
        assert sha(evaluation / 'inputs' / f"{case['index']:06d}.txt") == case['sha256']
        solution = SOURCE / 'cases' / f"{case['index']:06d}" / 'best.txt'
        metrics = replay(SOURCE / case['path'], solution)['metrics']
        assert metrics['E'] == 0 and metrics['T'] == case['frames']
        profiles['teacher'][case['index']] = metrics
    common = [g['case'] for g, b in zip(rows['greedy'], rows['beam4']) if g['E'] == b['E'] == 0]
    assert len(common) == comparison['common_completed']
    details = []
    for condition, values in profiles.items():
        metrics = [values[idx] for idx in common]
        operations = sum(m['T'] for m in metrics)
        details.append({'condition': condition, 'cases': len(common),
                        'mean_T': float(np.mean([m['T'] for m in metrics])),
                        'long_jump_fraction': sum(m['long_jumps'] for m in metrics) / operations,
                        'mean_packet_size': sum(m['moved_count'] for m in metrics) / operations,
                        'mixed_move_fraction': sum(m['mixed_moves'] for m in metrics) / operations,
                        'mean_moved_distance_sum': float(np.mean([m['W'] for m in metrics])),
                        'mean_max_height': float(np.mean([m['max_height'] for m in metrics]))})
    suffix = json.loads((evaluation / 'suffix_cases.json').read_text()); assert len(suffix) == 512
    for condition in ('greedy', 'beam4'):
        for length in (10, 50):
            selected = [r for r in suffix if r['condition'] == condition and r['teacher_suffix'] == length]
            assert len({r['case'] for r in selected}) == 128
            assert statistics(selected) == comparison['suffix'][f'{condition}_last{length}']
    common_g = [profiles['greedy'][idx]['T'] for idx in common]
    common_b = [profiles['beam4'][idx]['T'] for idx in common]
    differences = np.array(common_g) - np.array(common_b)
    result = {'verified': True, 'solver_reruns': 0, 'completed_at': now(), 'initial_records': 256,
              'suffix_records': 512, 'profile_common_completed': details,
              'common_beam_vs_greedy': {'mean_saved': float(differences.mean()),
                                      'wins': int((differences > 0).sum()), 'draws': int((differences == 0).sum()),
                                      'losses': int((differences < 0).sum())},
              'failures': {c: [r for r in rs if r['E']] for c, rs in rows.items()},
              'learning_history': learning['history'],
              'train_history_caveat': '1・10・20周の訓練診断は固定間引き集合、40周は全訓練例。検証は各時点で全例。',
              'memory_peak_gb_conservative': json.loads((run / 'guard_train/exit.json').read_text())['peak_observed_bytes'] / 1e9,
              'pipeline_seconds': json.loads((run / 'pipeline/exit.json').read_text())['seconds']}
    save(evaluation / 'post_analysis.json', result)
    print(json.dumps({k: v for k, v in result.items() if k not in ('learning_history', 'failures')}, ensure_ascii=False))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('--run', type=Path, default=RUN)
    args = parser.parse_args(); analyze(args.run.resolve())
