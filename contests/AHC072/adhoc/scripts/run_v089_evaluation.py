#!/usr/bin/env python3
"""固定40周のモデルで方策のみ・幅4探索を各1回評価し、未完走も保存する。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import shutil
import subprocess
import sys
import time

import numpy as np

from check_v089_board import compile_binary, trace
from v089_data import ROOT, SOURCE, RUN, Dataset, Geometry, remaining, save, sha, status, now

BINS = {'greedy': 'v089_nn_greedy', 'beam4': 'v089_nn_board'}


def command(args, log):
    with log.open('w') as out:
        subprocess.run(list(map(str, args)), cwd=ROOT, stdout=out, stderr=subprocess.STDOUT, check=True)


def validated_output(input_path, output, state=None):
    geo = Geometry(input_path)
    state = geo.initial.copy() if state is None else np.array(state, copy=True)
    lines = [line for line in output.splitlines() if line.strip()]
    for line in lines: geo.apply(state, line)
    E = remaining(state); T = len(lines)
    return {'T': T, 'E': E, 'S': T + 100000 * E, 'complete': E == 0}


def statistics(rows):
    finished = [r for r in rows if r['E'] == 0]
    return {'cases': len(rows), 'complete': len(finished), 'completion_rate': len(finished) / len(rows),
            'mean_S_all': float(np.mean([r['S'] for r in rows])), 'mean_E_all': float(np.mean([r['E'] for r in rows])),
            'max_E': max(r['E'] for r in rows),
            'mean_T_completed': float(np.mean([r['T'] for r in finished])) if finished else None,
            'mean_elapsed_ms': float(np.mean([r['elapsed_ms'] for r in rows])),
            'max_elapsed_ms': max(r['elapsed_ms'] for r in rows),
            'mean_inferences': float(np.mean([r['trace']['inferences'] for r in rows])),
            'mean_depth': float(np.mean([r['trace']['depth'] for r in rows])),
            'deadline_cases': sum(r['trace']['deadline'] for r in rows)}


def suffix_job(directory, binary, case, fid, suffix, condition, state):
    stem = f"{case['index']:06d}_last{suffix}_{condition}"
    snap = directory / f'{stem}.state'; snap.write_text(' '.join(map(str, state)) + '\n')
    input_path = SOURCE / case['path']; started = time.monotonic()
    proc = subprocess.run([binary, snap], input=input_path.read_text(), text=True,
                          capture_output=True, timeout=10., check=True)
    elapsed_ms = (time.monotonic() - started) * 1000.
    (directory / f'{stem}.txt').write_text(proc.stdout); (directory / f'{stem}.err').write_text(proc.stderr)
    result = validated_output(input_path, proc.stdout, state)
    counts = trace(proc.stderr)
    assert counts['T'] == result['T'] and counts['E'] == result['E']
    return {'case': case['index'], 'frame': fid, 'teacher_suffix': suffix, 'condition': condition,
            'elapsed_ms': elapsed_ms, 'trace': counts, **result}


def evaluate(run):
    assert json.loads((run / 'model.json').read_text())['epoch'] == 40
    assert json.loads((run / 'numerical_check.json').read_text())['passed']
    directory = run / 'evaluation'; directory.mkdir(exist_ok=False)
    (directory / 'inputs').mkdir(); (directory / 'frozen').mkdir(); (directory / 'outputs').mkdir()
    data = Dataset(run); cases = [c for c in data.cases if c['role'] == 'validation']; assert len(cases) == 128
    train_hashes = {c['sha256'] for c in data.cases if c['role'] == 'train'}
    for case in cases:
        src = SOURCE / case['path']; assert sha(src) == case['sha256'] and case['sha256'] not in train_hashes
        shutil.copy2(src, directory / 'inputs' / f"{case['index']:06d}.txt")
    save(directory / 'input_manifest.json', cases)
    source = ROOT / 'src/bin/v089_nn_board.cpp'; digest = sha(source)
    save(directory / 'config.json', {'jobs': 20, 'local': True, 'model_sha256': sha(run / 'model.json'),
                                    'solver_sha256': digest, 'conditions': BINS, 'created_at': now()})
    shutil.copy2(source, directory / 'frozen' / source.name)
    for condition, name in BINS.items():
        for local in (False, True):
            compile_binary(name, directory / 'frozen' / f'{name}_{"local" if local else "judge"}', local)
    all_rows = {}
    for condition, name in BINS.items():
        status(run, 'evaluating_initial_states', condition=condition, cases=128, jobs=20)
        assert sha(source) == digest
        output = ROOT / 'results/out' / name
        if output.exists(): raise FileExistsError(f'refuse to overwrite prior evaluation: {output}')
        label = f'{run.name}_v089_{condition}'
        command([sys.executable, ROOT / 'scripts/eval.py', name, directory / 'inputs', '-j', '20',
                 '--wait-lock', '--label', label], directory / f'eval_{condition}.log')
        shutil.copytree(output, directory / 'outputs' / condition)
        records = []
        with (ROOT / 'results/eval_records.jsonl').open() as stream:
            for line in stream:
                row = json.loads(line)
                if row['label'] == label: records.append(row)
        assert len(records) == len({r['case_name'] for r in records}) == 128
        assert len({r['run_id'] for r in records}) == 1
        save(directory / f'records_{condition}.json', records)
        indexed = {r['case_name']: r for r in records}; rows = []
        for case in cases:
            name_ = f"{case['index']:06d}.txt"; record = indexed[name_]
            assert record['status'] == 'ok' and record['local']
            result = validated_output(directory / 'inputs' / name_, (output / name_).read_text())
            counts = trace((output / (name_ + '.err')).read_text())
            assert result['S'] == record['score'] and counts['E'] == result['E'] and counts['T'] == result['T']
            rows.append({'case': case['index'], 'baseline_T': case['baseline_T'], 'teacher_T': case['frames'],
                         'elapsed_ms': record['elapsed'], 'trace': counts, **result})
        all_rows[condition] = rows
        save(directory / f'cases_{condition}.json', rows)
    assert sha(source) == digest
    suffix_dir = directory / 'suffix'; suffix_dir.mkdir()
    suffix_rows = []
    # 時間比較が同じ20並列になるよう、条件と残り長さごとに別々に実行する。
    for condition in BINS:
        helper = 'check_v089_greedy' if condition == 'greedy' else 'check_v089_board'
        binary = directory / 'frozen' / helper; compile_binary(helper, binary)
        for suffix in (10, 50):
            status(run, 'evaluating_suffixes', condition=condition, suffix=suffix, jobs=20)
            tasks = []
            for case in cases:
                assert case['frames'] >= suffix
                fid = case['frame_start'] + case['frames'] - suffix
                tasks.append((suffix_dir, binary, case, fid, suffix, condition, np.array(data.states[fid])))
            with ThreadPoolExecutor(max_workers=20) as pool:
                rows = list(pool.map(lambda args: suffix_job(*args), tasks))
            suffix_rows.extend(rows); save(directory / 'suffix_cases.json', suffix_rows)
    metrics = {condition: statistics(rows) for condition, rows in all_rows.items()}
    common = [(g, b) for g, b in zip(all_rows['greedy'], all_rows['beam4']) if g['E'] == b['E'] == 0]
    g, b = metrics['greedy'], metrics['beam4']
    continuation = b['complete'] >= 122 and b['max_elapsed_ms'] <= 2000 and (
        b['complete'] > g['complete'] or (b['complete'] == g['complete'] and b['mean_S_all'] < g['mean_S_all']))
    finished = [r for r in all_rows['beam4'] if r['E'] == 0]
    report = {'initial': metrics, 'all_legal': True, 'common_completed': len(common),
              'mean_T_saved_on_common': float(np.mean([x['T'] - y['T'] for x, y in common])) if common else None,
              'reference_saved_vs_saved_v079_on_beam_completed': float(np.mean([r['baseline_T'] - r['T'] for r in finished])) if finished else None,
              'reference_warning': '保存v079は教師採取時の値であり、同時条件の因果比較ではない。',
              'suffix': {f'{condition}_last{suffix}': statistics([r for r in suffix_rows if r['condition'] == condition and r['teacher_suffix'] == suffix])
                         for condition in BINS for suffix in (10, 50)},
              'prototype_continuation_gate': bool(continuation), 'all_128_complete_beam': b['complete'] == 128,
              'completed_at': now(), 'solver_sha256': digest}
    save(directory / 'comparison.json', report)
    # 固定した判定まで記録する。結果に応じた追加学習や探索条件の変更は行わない。
    note = ROOT / 'notes/experiments/v089.md'; text = note.read_text()
    sentence = ('事前登録の試作継続基準を満たした' if continuation else '事前登録の試作継続基準を満たさなかった')
    aftermath = (f'- 判定: {sentence}。提出版の全帰巣条件は{"達成" if b["complete"] == 128 else "未達"}。\n'
                 f'- 結果: 未学習128入力で、方策のみは{g["complete"]}件、幅4は{b["complete"]}件が全帰巣した。全出力は合法。'
                 f'幅4の最大外部時間は{b["max_elapsed_ms"]} ms。共通完走は{len(common)}件で、手数差と残り10・50手の診断は'
                 f'`{directory.relative_to(ROOT)}/comparison.json`を正本とする。\n'
                 '- 考察: 方策の教師一致率と、初期盤面から最後まで完走できる能力を別々に確認した。'
                 '今回の固定条件の判定後は変更を行わず、次の仮説はユーザーと相談する。\n')
    assert '## 実験後\n\n未実行。' in text
    note.write_text(text.replace('## 実験後\n\n未実行。', '## 実験後\n\n' + aftermath))
    backlog = ROOT / 'notes/backlog.md'; text = backlog.read_text()
    start = text.index('- **[B-117]')
    boundaries = [p for marker in ('\n- **[B-', '\n## ') if (p := text.find(marker, start + 1)) >= 0]
    end = min(boundaries) if boundaries else len(text)
    block = text[start:end]
    assert '[v089](experiments/v089.md)' in block
    first, rest = block.split('\n', 1)
    title = first.split(' → ', 1)[0]
    block = (f'{title} → [v089](experiments/v089.md) 初回試作の比較完了。{sentence}。'
             f'未学習128入力の全帰巣は方策のみ{g["complete"]}件、幅4は{b["complete"]}件。'
             '再開条件: この結果を踏まえた次の学習・探索案についてユーザーの新たな指示を受けた場合。'
             '\n' + rest)
    text = text[:start] + text[end:]
    heading = '## 決着済み'
    pos = text.index('\n', text.index(heading)) + 1
    text = text[:pos] + '\n' + block.rstrip() + '\n\n' + text[pos:]
    backlog.write_text(text)
    status(run, 'completed_with_evaluation', prototype_continuation_gate=continuation,
           greedy_complete=g['complete'], beam_complete=b['complete'])
    print(json.dumps(report, ensure_ascii=False), flush=True)


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--run', type=Path, default=RUN); args = p.parse_args()
    evaluate(args.run.resolve())
