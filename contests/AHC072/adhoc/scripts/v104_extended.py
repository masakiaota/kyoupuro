#!/usr/bin/env python3
"""v102の保存状態を保ったまま、補助なしの追加学習条件を固定する。"""
from pathlib import Path
import numpy as np
import torch
from v089_data import ROOT,save,sha
from v091_env import load
from v092_stream import excluded_inputs

PARENT=ROOT/'results/nn_rank/v102/20261004_bc_ablation_studio/without_bc'
INITIAL=PARENT/'training/latest.pt'
INITIAL_SHA='c4fc2cd727e65382ef6f5f14d7dd755230b0fe68de1414a5ba48b09616f285d9'
POINTS=(80,160,320)


def configuration():
    config=load(PARENT/'training/config.json')['config']
    assert config['bc_coef']==config['reverse_coef']==0 and config['method']=='group'
    return dict(config,iterations=320,seconds=14400,initial_lr=config['final_lr'],final_lr=config['final_lr'])


def restore(model,optimizer,rngs,saved,device):
    model.load_state_dict(saved['model']);optimizer.load_state_dict(saved['optimizer'])
    for name,rng in rngs.items():rng.bit_generator.state=saved['rngs'][name]
    torch.set_rng_state(saved['torch_rng'])
    if device=='mps':torch.mps.set_rng_state(saved['mps_rng'])


def blocked_inputs(root,completed_iterations=0):
    blocked=excluded_inputs()
    # 親と自分の消費済み入力は、別seedで偶然一致した場合も次の学習から除く。
    for directory,limit in ((PARENT/'training/episodes',40),(root/'training/episodes',completed_iterations)):
        for path in sorted(directory.glob('*.json')):
            # 列の記録はcheckpointより先に書く。未保存の更新に属する記録は再開後に読み直さない。
            if int(path.stem)>limit:continue
            for item in load(path)['inputs']:blocked.add(item['sha256'])
    diagnostic=ROOT/'results/nn_rank/v103/20261004_self_imitation_studio/diagnostic/data/input_manifest.json'
    if diagnostic.exists():
        for item in load(diagnostic):blocked.add(item['sha256'])
    return blocked


def inherited_state(root):
    path=root/'initial/checkpoint.pt';assert sha(path)==INITIAL_SHA
    saved=torch.load(path,map_location='cpu',weights_only=False)
    assert saved['iteration']==40
    assert {int(v['step']) for v in saved['optimizer']['state'].values()}=={27840}
    assert saved['queue']['consume_ticket']==1280 and saved['queue']['next_ticket']==1344
    return saved
