#!/usr/bin/env python3
"""小型モデルの重みとC++推論を、直接提出できる単一ファイルへまとめる。"""
import argparse
import json
from pathlib import Path

import numpy as np

from v089_data import ROOT, RUN


def build(model_path, destination):
    model = json.loads(model_path.read_text()); p = model['parameters']
    arrays = [('input_w', p['input.weight']), ('input_b', p['input.bias']),
              ('actor_w', p['actor.0.weight']), ('actor_b', p['actor.0.bias']),
              ('actor_out_w', p['actor.2.weight']), ('actor_out_b', p['actor.2.bias']),
              ('critic_w', p['critic.0.weight']), ('critic_b', p['critic.0.bias']),
              ('critic_out_w', p['critic.2.weight']), ('critic_out_b', p['critic.2.bias'])]
    def literals(values):
        return ','.join(f'{float(x):.9e}f' for x in np.asarray(values).ravel())
    parts = [(ROOT / 'adhoc/scripts/v089_core.cpp.txt').read_text(),
             f'// 学習epoch={model["epoch"]}, dataset SHA256={model["dataset_sha256"]}\n']
    for name, values in arrays:
        flat = np.asarray(values).ravel()
        parts.append(f'static constexpr array<float,{len(flat)}> nn_{name} = {{{literals(flat)}}};\n')
    for name, key in [('depth_w', 'depth.weight'), ('depth_b', 'depth.bias'),
                       ('point_w', 'point.weight'), ('point_b', 'point.bias')]:
        values = [np.asarray(p[f'blocks.{b}.{key}']).ravel() for b in range(3)]
        parts.append(f'static constexpr array<array<float,{len(values[0])}>,3> nn_{name} = {{{{')
        parts.append(','.join('{' + literals(v) + '}' for v in values)); parts.append('}};\n')
    parts.append((ROOT / 'adhoc/scripts/v089_search.cpp.txt').read_text())
    text = ''.join(parts).splitlines(); text[0] = '// ' + destination.name
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text('\n'.join(text) + '\n')


if __name__ == '__main__':
    p = argparse.ArgumentParser(); p.add_argument('--model', type=Path, default=RUN / 'model.json')
    p.add_argument('--output', type=Path, default=ROOT / 'src/bin/v089_nn_board.cpp')
    args = p.parse_args(); build(args.model, args.output)
