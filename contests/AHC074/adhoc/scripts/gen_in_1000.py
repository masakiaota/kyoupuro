#!/usr/bin/env python3
"""公式生成器で凍結したin_1000を作り、再実行時には指紋を照合する。"""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'tools/in_1000'
MANIFEST = ROOT / 'results/in_1000_manifest.json'
DOMAIN = 'ahc074/in_1000/20261108/v1'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def public(data):
    tokens = data.split()
    assert tokens[:4] == [b'64', b'192', b'256', b'1024']
    assert len(tokens) == 4 + 256 + 1024
    products = tokens[4:260]
    assert len(set(products)) == 256
    assert all(len(s) == 64 and s.count(b'1') == 32 and set(s) <= {48, 49} for s in products)
    orders = list(map(int, tokens[260:]))
    assert all(0 <= a < 256 for a in orders)
    assert all(a != b for a, b in zip(orders, orders[1:]))
    return b'64 192 256 1024\n' + b'\n'.join(products) + b'\n'


def verify(manifest):
    assert manifest['seed_domain'] == DOMAIN
    assert sorted(p.name for p in DEST.iterdir()) == sorted(manifest['inputs'])
    for name, row in manifest['inputs'].items():
        data = (DEST / name).read_bytes()
        assert sha(data) == row['sha256'], name
        assert sha(public(data)) == row['public_sha256'], name
    assert len(manifest['inputs']) == 1000


def main():
    saved = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else None
    if DEST.exists():
        if saved is None:
            raise SystemExit('既存in_1000に指紋がないため、上書きせず停止する')
        verify(saved)
        print('in_1000: 1000件の保存済み指紋に一致')
        return
    generator_sha = sha((ROOT / 'tools/src/lib.rs').read_bytes())
    if saved is not None:
        assert saved['generator_source_sha256'] == generator_sha
    seeds = [int.from_bytes(hashlib.sha256(f'{DOMAIN}/{i}'.encode()).digest()[:8], 'big')
             for i in range(1000)]
    assert len(set(seeds)) == 1000
    # 既存ディレクトリを壊さず、生成と全件検査が成功してから公開する。
    with tempfile.TemporaryDirectory(prefix='.in_1000_', dir=ROOT / 'tools') as temp:
        area = Path(temp)
        (area / 'seeds.txt').write_text(''.join(f'{seed}\n' for seed in seeds))
        subprocess.run([str(ROOT / 'scripts/gen_tools.sh'), str(area / 'seeds.txt'),
                        '--dir', str(area / 'inputs')], check=True, cwd=ROOT)
        inputs = {}
        for i, seed in enumerate(seeds):
            name = f'{i:04}.txt'
            data = (area / 'inputs' / name).read_bytes()
            inputs[name] = dict(seed=seed, sha256=sha(data), public_sha256=sha(public(data)))
        fingerprints = {r['public_sha256'] for r in inputs.values()}
        assert len(fingerprints) == len(inputs) == 1000
        if saved is not None:
            assert saved['inputs'] == inputs, '公式生成器の出力が保存済み指紋と異なる'
            manifest = saved
        else:
            prior = set()
            checks = {}
            for folder in ('tools/in', 'tools/in_holdout'):
                values = {sha(public(p.read_bytes())) for p in (ROOT / folder).glob('*.txt')}
                assert not fingerprints & values, folder
                prior.update(values)
                checks[folder] = len(values)
            cache = ROOT / 'adhoc/analysis/joint_design_20261107/v5215/prior_public_cache.json'
            if cache.exists():
                cached = json.loads(cache.read_text())
                values = {r['sha256'] for r in cached.values() if r['sha256']}
                assert not fingerprints & values, '既存の公開商品表と重複'
                prior.update(values)
                checks[str(cache.relative_to(ROOT))] = dict(count=len(values), sha256=sha(cache.read_bytes()))
            # 直近の独立確認も、保存済みの指紋だけで照合する。
            recent = ROOT / 'results/v5266_inputs.json'
            if recent.exists():
                values = set(json.loads(recent.read_text())['public_sha256'])
                assert not fingerprints & values
                prior.update(values)
                checks['results/v5266_inputs.json'] = len(values)
            manifest = dict(seed_domain=DOMAIN, generator_source_sha256=generator_sha,
                            cases=1000, inputs=inputs, prior_checks=checks,
                            unique_prior_public=len(prior), overlap=0)
        (area / 'inputs').rename(DEST)
        MANIFEST.parent.mkdir(exist_ok=True)
        MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n')
    verify(manifest)
    print(f'in_1000: 1000件生成、既存{manifest["unique_prior_public"]}表との重複0')


if __name__ == '__main__':
    main()
