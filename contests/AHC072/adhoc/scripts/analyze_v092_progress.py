#!/usr/bin/env python3
"""保存済みの学習ログだけからv092の中間推移を集計する。方策は実行しない。"""
import argparse
import csv
import hashlib
import json
from pathlib import Path

import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[2]


def summarize(metrics, episodes, first, last):
    ms = [x for x in metrics if first <= x['iteration'] <= last]
    es = [x for x in episodes if first <= x['iteration'] <= last]
    complete = [x for x in es if x['E'] == 0]
    result = dict(first=first, last=last, minutes=ms[-1]['seconds'] / 60,
                  episodes=len(es), completed=len(complete),
                  mean_T=float(np.mean([x['T'] for x in complete])),
                  median_T=float(np.median([x['T'] for x in complete])))
    for key in ('entropy', 'bc_ce', 'policy_loss', 'value_loss', 'approx_kl',
                'clip_fraction', 'gradient_norm', 'rollout_seconds', 'update_seconds'):
        result[key] = float(np.mean([x[key] for x in ms]))
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--through', type=int, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    data, sources = {}, {}
    for version, run, limit in [('v091', '20261003_ppo_studio', 215),
                                ('v092', '20261003_fresh_studio', args.through)]:
        logs = ROOT / 'results/nn_rank' / version / run / 'training'
        data[version] = {}
        for name in ('metrics', 'episodes'):
            raw = (logs / f'{name}.jsonl').read_bytes()
            rows = [json.loads(line) for line in raw.splitlines() if line.strip()]
            rows = [x for x in rows if x['iteration'] <= limit]
            data[version][name] = rows
            sources[f'{version}/{name}'] = dict(sha256=hashlib.sha256(raw).hexdigest(), rows=len(rows))
            (args.output / f'{version}_{name}.json').write_text(json.dumps(rows))
        metrics, episodes = data[version]['metrics'], data[version]['episodes']
        assert metrics[-1]['iteration'] == limit
        bins = [(a, min(a + 39, limit)) for a in range(1, limit + 1, 40)]
        data[version]['bins'] = [summarize(metrics, episodes, a, b) for a, b in bins]
        data[version]['last40'] = summarize(metrics, episodes, limit - 39, limit)
    summary = dict(through=args.through, sources=sources,
                   versions={v: {k: d[k] for k in ('bins', 'last40')} for v, d in data.items()},
                   limitations=['Training rollouts use stochastic choices and different inputs.',
                                'Early finished episodes are biased toward short cases.',
                                'No current checkpoint evaluation or training change.'])
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    with (args.output / 'intervals.csv').open('w') as stream:
        writer = csv.DictWriter(stream, fieldnames=['version', *data['v092']['bins'][0]])
        writer.writeheader()
        for version in data:
            for row in data[version]['bins']: writer.writerow(dict(version=version, **row))

    plt.rcParams.update({'font.family': 'Hiragino Sans', 'font.size': 11})
    fig, axes = plt.subplots(1, 2, figsize=(12, 4.8))
    for version, color, label in [('v091', '#737b87', 'v091：既存入力'),
                                  ('v092', '#087f8c', 'v092：新規公式入力')]:
        rows = data[version]['bins']
        x = [r['last'] for r in rows]
        axes[0].plot(x, [r['mean_T'] for r in rows], 'o-', color=color, label=label)
        axes[1].plot(x, [r['bc_ce'] for r in rows], 'o-', color=color, label=label)
    axes[0].set(title='学習中に完走した盤面の平均手数', ylabel='完成手数')
    axes[1].set(title='旧教師に対する模倣補助の損失', ylabel='交差エントロピー')
    for ax in axes:
        ax.set_xlabel('各学習の区間数（1区間＝16,384操作）')
        ax.grid(alpha=.2)
        ax.legend(frameon=False)
    fig.suptitle('v092の途中経過：184区間、約107分時点', fontsize=15)
    fig.text(.5, .015, 'v092はv091の終了重みから継続。横軸は各工程で0に戻す。\n40区間ごとに集計（末尾のみ端数）。入力は各区間で異なり、最初は短い完走例に偏る。',
             ha='center', fontsize=10, color='#555555')
    fig.tight_layout(rect=(0, .08, 1, .94))
    fig.savefig(args.output / 'progress.png', dpi=160)
    plt.close(fig)
    print(json.dumps(dict(output=str(args.output), through=args.through, sources=sources)))


if __name__ == '__main__':
    main()
