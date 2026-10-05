#!/usr/bin/env python3
"""成功した個体集合を、関係情報を使うモデルから直接生成する。"""
import argparse
import json
import math
from pathlib import Path
import signal
import time

import numpy as np
import torch
from torch import nn

from train_v077_rank import checkpoint
from train_v080_scaling import GPUReport
from v086_data import Dataset, ROOT, SEED, EPOCHS, BATCH, save, sha, status, now


class Policy(nn.Module):
    def __init__(self):
        super().__init__()
        self.node = nn.Linear(24, 32)
        self.message = nn.Linear(192, 32)
        self.choose = nn.Sequential(nn.Linear(102, 32), nn.ReLU(), nn.Linear(32, 1))
        self.stop = nn.Sequential(nn.Linear(65, 32), nn.ReLU(), nn.Linear(32, 1))

    def encode(self, nodes, edges, valid):
        h = torch.relu(self.node(nodes)) * valid[..., None]
        # 関係種ごとの加重平均を、各個体の特徴へ連結する。
        messages = torch.matmul(edges, h[:, None]).permute(0, 2, 1, 3).flatten(2)
        h = torch.relu(self.message(torch.cat((h, messages), -1))) * valid[..., None]
        global_h = h.sum(1) / valid.sum(1).clamp_min(1)[:, None]
        return h, global_h

    def decode(self, encoded, edges, valid, prefix):
        h, global_h = encoded
        B, S, M = prefix.shape
        count = prefix.sum(-1, keepdim=True)
        selected_h = torch.matmul(prefix, h) / count.clamp_min(1)
        related = torch.matmul(edges, prefix.transpose(1, 2)[:, None]).permute(0, 3, 2, 1)
        global_steps = global_h[:, None].expand(B, S, 32)
        length = count / 12
        features = torch.cat((h[:, None].expand(B, S, M, 32),
                              global_steps[:, :, None].expand(B, S, M, 32),
                              selected_h[:, :, None].expand(B, S, M, 32),
                              related, length[:, :, None].expand(B, S, M, 1)), -1)
        logits = self.choose(features).squeeze(-1)
        eligible = valid[:, None] & (prefix == 0) & (count < 12)
        logits = logits.masked_fill(~eligible, -1e9)
        stop = self.stop(torch.cat((global_steps, selected_h, length), -1))
        stop = stop.masked_fill(count == 0, -1e9)
        return torch.cat((logits, stop), -1)

    def forward(self, batch):
        encoded = self.encode(batch['nodes'], batch['edges'], batch['valid'])
        return self.decode(encoded, batch['edges'], batch['valid'], batch['prefix'])


def model_new(device):
    torch.manual_seed(SEED)
    if device == 'mps':
        torch.mps.manual_seed(SEED)
    return Policy().to(device)


def tensors(batch, device):
    return {key: torch.from_numpy(np.array(value, copy=True)).to(device)
            for key, value in batch.items() if key != 'cases'}


def per_example_loss(logits, batch):
    remaining = (batch['target'][:, None].float() - batch['prefix']).clamp_min(0)
    total = remaining.sum(-1, keepdim=True)
    distribution = torch.cat((remaining / total.clamp_min(1), (total == 0).float()), -1)
    loss = -(distribution * torch.log_softmax(logits, -1)).sum(-1)
    steps = torch.arange(loss.shape[1], device=loss.device)[None] <= batch['sizes'][:, None]
    return (loss * steps).sum(-1) / (batch['sizes'] + 1)


@torch.no_grad()
def greedy(model, batch):
    encoded = model.encode(batch['nodes'], batch['edges'], batch['valid'])
    B, M = batch['valid'].shape
    selected = torch.zeros((B, 1, M), device=batch['nodes'].device)
    finished = torch.zeros(B, dtype=torch.bool, device=selected.device)
    for _ in range(13):
        logits = model.decode(encoded, batch['edges'], batch['valid'], selected)[:, 0]
        choices = logits.argmax(-1)
        take = (choices != M) & ~finished
        # 終了済みの例には更新を適用しない。動的なgatherの形を避ける。
        one_hot = torch.nn.functional.one_hot(choices, M + 1)[:, :M].float()
        selected[:, 0] += one_hot * take[:, None]
        finished |= choices == M
    assert finished.all().item() and (selected <= 1).all().item()
    return selected[:, 0].bool()


def set_accuracy(predicted, target):
    common = (predicted & target).sum(-1).float()
    return {'recall': float((common / target.sum(-1)).mean().item()),
            'exact': float((predicted == target).all(-1).float().mean().item()),
            'jaccard': float((common / (predicted | target).sum(-1)).mean().item())}


@torch.no_grad()
def evaluate(model, data, role, device):
    ids = data.splits[role]
    # 検証順は固定し、同じ入力の正解集合を別の分割へ動かさない。
    by_case = {}
    for start in range(0, len(ids), BATCH):
        raw = data.batch(ids[start:start+BATCH], 0); batch = tensors(raw, device)
        values = per_example_loss(model(batch), batch).cpu().numpy()
        for case, weight, value, M in zip(raw['cases'], raw['weights'], values, [data.rows[i][2] for i in ids[start:start+BATCH]]):
            entry = by_case.setdefault(int(case), {'sum': 0., 'weight': 0., 'M': M})
            entry['sum'] += float(weight) * float(value); entry['weight'] += float(weight)
    assert len(by_case) == data.description['contributing_inputs'][role]
    values = [row['sum'] / row['weight'] for row in by_case.values()]
    result = {'loss': float(np.mean(values)), 'inputs': len(values), 'examples': len(ids)}
    for label, condition in [('M_lt_80', lambda m: m < 80), ('M_ge_80', lambda m: m >= 80)]:
        part = [row['sum']/row['weight'] for row in by_case.values() if condition(row['M'])]
        result[label] = {'loss': float(np.mean(part)), 'inputs': len(part)}
    return result


@torch.no_grad()
def generated_sets(model, data):
    by_case = {}
    sizes = {}
    ids = data.splits['validation']
    for start in range(0,len(ids),BATCH):
        raw=data.batch(ids[start:start+BATCH],0);batch=tensors(raw,'mps')
        predicted=greedy(model,batch).cpu().numpy();target=raw['target']
        common=(predicted&target).sum(-1);union=(predicted|target).sum(-1)
        for b,case in enumerate(raw['cases']):
            metrics=np.array([common[b]/target[b].sum(),common[b]/union[b],float(np.array_equal(predicted[b],target[b]))])
            entry=by_case.setdefault(int(case),[np.zeros(3),0.])
            entry[0]+=metrics*raw['weights'][b];entry[1]+=raw['weights'][b]
            size=int(predicted[b].sum());sizes[str(size)]=sizes.get(str(size),0)+1
    means=np.mean([value/weight for value,weight in by_case.values()],axis=0)
    return {'recall':float(means[0]),'jaccard':float(means[1]),'exact_set_rate':float(means[2]),
            'greedy_size_histogram':sizes,'inputs':len(by_case),'examples':len(ids),
            'meaning':'自由生成の模倣診断。正解集合の再現率であり、再構築時の短縮率ではない。'}


def update(model, opt, batch, scale=1.):
    loss = (per_example_loss(model(batch), batch) * batch['weights'] * scale).mean()
    if not math.isfinite(loss.item()):
        raise FloatingPointError('nonfinite policy loss')
    opt.zero_grad(set_to_none=True); loss.backward()
    torch.nn.utils.clip_grad_norm_(model.parameters(), 5, error_if_nonfinite=True)
    opt.step()
    return float(loss.item())


def synthetic_batch():
    random = np.random.default_rng(SEED)
    node = random.normal(size=(4,8,24)).astype(np.float32)
    edge = random.random((4,5,8,8)).astype(np.float32)
    edge *= 1 - np.eye(8, dtype=np.float32)[None,None]
    edge /= edge.sum(-1, keepdims=True)
    target = np.zeros((4,8), bool); prefix = np.zeros((4,5,8), np.float32)
    for b, size in enumerate(range(1,5)):
        selected = np.argsort(node[b,:,0])[-size:]
        target[b,selected] = True
        for k, at in enumerate(selected):
            prefix[b,k+1:] = prefix[b,k]; prefix[b,k+1:,at] = 1
    return {'nodes': node, 'edges': edge, 'valid': np.ones((4,8),bool), 'target': target,
            'prefix': prefix, 'weights': np.ones(4,np.float32), 'sizes': np.arange(1,5,dtype=np.int64)}


def fit_check(raw, device, deadline):
    batch = tensors(raw, device); model = model_new(device)
    opt = torch.optim.Adam(model.parameters(), lr=.005)
    for step in range(2000):
        if time.time() >= deadline:
            raise TimeoutError('mechanism_check_time_limit')
        update(model, opt, batch)
        if (step+1) % 100 == 0:
            measured = set_accuracy(greedy(model, batch), batch['target'])
            if measured['exact'] >= .95:
                return model, opt, {**measured, 'steps': step+1}
    raise AssertionError(('tiny_set_fit_failed', measured))


def check(run, deadline):
    data = Dataset(run)
    result = {'passed': False, 'torch': torch.__version__, 'parameters': sum(p.numel() for p in Policy().parameters())}
    model, opt, result['synthetic'] = fit_check(synthetic_batch(), 'cpu', deadline)
    chosen, seen = [], set()
    for i in data.splits['train']:
        case = data.rows[i][0]
        if case not in seen:
            chosen.append(i); seen.add(case)
            if len(chosen) == 4:
                break
    raw = data.batch(chosen, 0)
    model, opt, result['real_four'] = fit_check(raw, 'cpu', deadline)
    batch = tensors(raw, 'cpu')
    # 中断しない次の更新と、保存から戻した更新を同じ入力で照合する。
    path = run / 'mechanism_checkpoint.pt'
    checkpoint(path, {'model': model.state_dict(), 'optimizer': opt.state_dict()})
    saved = torch.load(path, weights_only=False, map_location='cpu')
    resumed = model_new('cpu'); resumed.load_state_dict(saved['model'])
    other = torch.optim.Adam(resumed.parameters(), lr=.005); other.load_state_dict(saved['optimizer'])
    update(model, opt, batch); update(resumed, other, batch)
    assert all(torch.equal(v, resumed.state_dict()[k]) for k,v in model.state_dict().items())
    result['checkpoint_next_update_exact'] = True
    gpu = model_new('mps'); gpu.load_state_dict(model.state_dict()); gpu_batch=tensors(raw, 'mps')
    with torch.no_grad():
        expected = model(batch).cpu().numpy(); actual = gpu(gpu_batch).cpu().numpy()
    eligible = expected > -1e8
    np.testing.assert_allclose(actual[eligible], expected[eligible], rtol=2e-5, atol=2e-4)
    result['cpu_mps_max_error'] = float(abs(actual[eligible]-expected[eligible]).max())
    # 個体番号を並べ替えても点数と集合の内容が対応することを確かめる。
    permutation = np.random.default_rng(SEED).permutation(raw['nodes'].shape[1])
    changed = {k: v.copy() for k,v in raw.items()}
    changed['nodes'] = raw['nodes'][:,permutation]
    changed['edges'] = raw['edges'][:,:,permutation][:,:,:,permutation]
    for key in ('valid', 'target'):
        changed[key] = raw[key][:,permutation]
    changed['prefix'] = raw['prefix'][:,:,permutation]
    with torch.no_grad():
        shuffled = model(tensors(changed,'cpu')).numpy()
    np.testing.assert_allclose(shuffled[:,:,:-1], expected[:,:,permutation], rtol=2e-5, atol=2e-4)
    np.testing.assert_allclose(shuffled[:,:,-1], expected[:,:,-1], rtol=2e-5, atol=2e-4)
    result['permutation_equivariant'] = True
    # 全体を代表する32例を一度だけ使い、同じ固定モデル・損失の速度を測る。
    positions = np.linspace(0,len(data.splits['train'])-1,BATCH,dtype=int)
    benchmark = tensors(data.batch([data.splits['train'][i] for i in positions],1),'mps')
    gpu = model_new('mps'); optimizer=torch.optim.Adam(gpu.parameters(),lr=.001)
    for step in range(40):
        if step == 10:
            torch.mps.synchronize(); started=time.monotonic()
        update(gpu,optimizer,benchmark)
    torch.mps.synchronize()
    result['seconds_per_update']=(time.monotonic()-started)/30
    result['training_estimate_seconds']=result['seconds_per_update']*math.ceil(len(data.splits['train'])/BATCH)*EPOCHS
    result['tiny_indices']=chosen
    result['passed']=True
    save(run/'learning_check.json',result)
    print(json.dumps(result),flush=True)


def train(run, deadline):
    assert json.loads((run/'learning_check.json').read_text())['passed']
    data=Dataset(run); ids=data.splits['train']; fingerprint=sha(run/'dataset.json')
    model=model_new('mps'); opt=torch.optim.Adam(model.parameters(),lr=.001)
    epoch=offset=steps=0; history=[]; latest=run/'latest.pt'
    if latest.exists():
        stored=torch.load(latest,map_location='cpu',weights_only=False)
        assert stored['dataset_sha256']==fingerprint
        model.load_state_dict(stored['model']);opt.load_state_dict(stored['optimizer'])
        epoch,offset,steps,history=(stored[k] for k in ('epoch','offset','steps','history'))
        torch.set_rng_state(stored['torch_rng']);torch.mps.set_rng_state(stored['mps_rng'])
    elif not (run/'initial_metrics.json').exists():
        metrics={'train':evaluate(model,data,'train','mps'),'validation':evaluate(model,data,'validation','mps')}
        save(run/'initial_metrics.json',metrics)
    initial=json.loads((run/'initial_metrics.json').read_text())
    interrupted=False
    def stop(signum,frame):
        nonlocal interrupted
        interrupted=True
    signal.signal(signal.SIGTERM,stop);signal.signal(signal.SIGINT,stop)
    def persist():
        checkpoint(latest,{'model':{k:v.detach().cpu() for k,v in model.state_dict().items()},
                          'optimizer':opt.state_dict(),'epoch':epoch,'offset':offset,'steps':steps,
                          'history':history,'dataset_sha256':fingerprint,'seed':SEED,
                          'torch_rng':torch.get_rng_state(),'mps_rng':torch.mps.get_rng_state()})
    start_steps=steps;started=time.monotonic();last_report=started
    total_steps=math.ceil(len(ids)/BATCH)*EPOCHS
    while epoch<EPOCHS:
        order=np.random.default_rng(SEED+epoch).permutation(ids)
        while offset<len(order):
            if interrupted or time.time()>=deadline:
                persist();raise TimeoutError('registered_time_limit_or_signal')
            selected=order[offset:offset+BATCH]
            raw=data.batch(selected,epoch+1)
            loss=update(model,opt,tensors(raw,'mps'),len(ids)/data.description['contributing_inputs']['train'])
            offset+=len(selected);steps+=1
            if steps%128==0:
                persist()
            if time.monotonic()-last_report>=30:
                rate=(steps-start_steps)/max(1e-6,time.monotonic()-started)
                status(run,'training',epoch=epoch+1,completed_epochs=epoch,steps=steps,total_steps=total_steps,
                       loss=loss,updates_per_second=rate,remaining_seconds=(total_steps-steps)/rate)
                last_report=time.monotonic()
        epoch+=1;offset=0
        if epoch in (1,10,30,60):
            measured={'epoch':epoch,'steps':steps,'train':evaluate(model,data,'train','mps'),
                      'validation':evaluate(model,data,'validation','mps')}
            history.append(measured);save(run/f'metrics_epoch{epoch:02d}.json',measured)
        persist()
    final=history[-1];ratio=final['validation']['loss']/initial['validation']['loss']
    parameters={k:v.detach().cpu().numpy().tolist() for k,v in model.state_dict().items()}
    save(run/'model.json',{'parameters':parameters,'mean':data.description['mean'],'scale':data.description['scale'],
                          'seed':SEED,'epochs':EPOCHS,'dataset_sha256':fingerprint})
    result={'initial':initial,'final':final,'validation_loss_ratio':ratio,'gate_passed':ratio<=.9,
            'completed_at':now(),'checkpoint_sha256':sha(latest),
            'generated_validation_sets':generated_sets(model,data),
            'meaning':'成功集合の模倣の判定。探索全体の短縮量は未測定。'}
    save(run/'result.json',result)
    status(run,'completed',epochs=epoch,steps=steps,gate_passed=result['gate_passed'])


def main():
    parser=argparse.ArgumentParser();parser.add_argument('mode',choices=('check','train'))
    parser.add_argument('--run',type=Path,required=True);parser.add_argument('--seconds',type=int,default=14400)
    args=parser.parse_args();run=args.run.resolve();deadline=time.time()+args.seconds
    torch.set_num_threads(2)
    assert torch.backends.mps.is_available()
    torch.mps.set_per_process_memory_fraction(min(1.,16e9/torch.mps.recommended_max_memory()))
    reporter=GPUReport(run/'gpu_state.json')
    try:
        if args.mode=='check':check(run,deadline)
        else:
            config=json.loads((run/'config.json').read_text())
            for relative,digest in config['source_sha256'].items():
                assert sha(ROOT/relative)==digest, ('source_changed',relative)
            assert sha(run/'dataset.json')==config['dataset_sha256']
            train(run,deadline)
    except BaseException as error:
        save(run/'failure.json',{'mode':args.mode,'type':type(error).__name__,'message':str(error),'time':now()})
        status(run,'failed',mode=args.mode,error=str(error))
        raise
    finally:
        reporter.close()


if __name__=='__main__':main()
