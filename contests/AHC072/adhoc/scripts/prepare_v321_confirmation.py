#!/usr/bin/env python3
"""固定済みCVの出力と新規入力を保存する。得点を取得しない。"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    run = Path(sys.argv[1]).resolve()
    manifest = json.loads((run/'manifest.json').read_text())
    cv = json.loads((run/'cv_result.json').read_text())
    # 今回、事前に許した深さ0が選ばれた。不要なルータ処理を提出物に残さない。
    assert cv['final_tree']['expert'] == 'v113' and cv['final_config']['depth'] == 0
    source = ROOT/'src/bin/v113_integrated_nn_lns.cpp'
    original = source.read_bytes()
    assert digest(original) == manifest['source_sha256']['v113']
    body = original.split(b'\n', 1)[1]
    target = ROOT/'src/bin/v321_cv_portfolio.cpp'
    submitted = (b'// v321_cv_portfolio.cpp\n'
                 b'// Nested validation CV selected depth 0: always use v113.\n'
                 b'// The executable body below is unchanged from v113_integrated_nn_lns.cpp.\n'+body)
    assert not target.exists()
    target.write_bytes(submitted)
    manifest['final_bin'] = target.stem
    manifest['final_source_sha256'] = digest(submitted)
    manifest['final_body_sha256'] = digest(body)
    manifest['final_expert'] = 'v113'
    manifest['confirmation_measurement_alias'] = 'v321 confirmation is also the unchanged v113 measurement'
    assert not (run/'confirm200').exists()
    directories = {str(ROOT/'tools/in'), str(ROOT/'tools/validation1')}
    for line in (ROOT/'results/eval_records.jsonl').open():
        value = json.loads(line).get('input_dir')
        if value:
            p = Path(value)
            directories.add(str(p if p.is_absolute() else ROOT/p))
    previous = set()
    inventory = []
    for name in sorted(directories):
        p = Path(name)
        files = sorted(p.glob('*.txt')) if p.is_dir() else []
        inventory.append({'directory': name, 'readable_cases': len(files)})
        previous.update(digest(f.read_bytes()) for f in files)
    (run/'prior_input_inventory.json').write_text(json.dumps(inventory, ensure_ascii=False, indent=2)+'\n')
    (run/'prior_input_hashes.json').write_text(json.dumps(sorted(previous))+'\n')
    seeds = []
    index = 0
    generated_hashes = {}
    destination = run/'confirm200'
    destination.mkdir()
    generated = run/'generator_batch'
    while len(seeds) < manifest['new_cases']:
        pending = []
        while len(pending) < manifest['new_cases']-len(seeds):
            seed = int.from_bytes(hashlib.sha256(f'AHC072-v321-confirm-20261004:{index}'.encode()).digest()[:8], 'big')
            index += 1
            if seed:
                pending.append(seed)
        batch_seeds = run/'generator_batch_seeds.txt'
        batch_seeds.write_text(''.join(f'{s}\n' for s in pending))
        with (run/'generator.log').open('a') as log:
            subprocess.run(['sh', 'scripts/gen_tools.sh', str(batch_seeds), '--dir', str(generated)],
                           cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        for i, seed in enumerate(pending):
            data = (generated/f'{i:04d}.txt').read_bytes()
            h = digest(data)
            if h in previous or h in generated_hashes.values():
                continue
            name = f'{len(seeds):04d}.txt'
            (destination/name).write_bytes(data)
            seeds.append(seed)
            generated_hashes[name] = h
    assert len(set(generated_hashes.values())) == 200
    assert not previous.intersection(generated_hashes.values())
    (run/'confirm_seeds.txt').write_text(''.join(f'{s}\n' for s in seeds))
    manifest['input_sha256']['confirm200'] = generated_hashes
    manifest['generator_sha256'] = {str(p.relative_to(ROOT)): digest(p.read_bytes()) for p in
                                   [ROOT/'tools/src/bin/gen.rs', ROOT/'tools/src/lib.rs', ROOT/'tools/Cargo.lock']}
    manifest['generator_binary_sha256'] = digest((ROOT/'tools/target/release/gen').read_bytes())
    manifest['prior_input_unique_hashes'] = len(previous)
    manifest['status'] = 'model_and_inputs_frozen'
    (run/'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n')
    (run/'frozen_router.json').write_text(json.dumps({'router': cv['final_tree'],
        'source_sha256': manifest['final_source_sha256'], 'body_sha256': manifest['final_body_sha256'],
        'new_input_sha256': generated_hashes}, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps({'final_expert': 'v113', 'new_cases': len(seeds),
                      'prior_unique_inputs_checked': len(previous),
                      'source_bytes': len(submitted), 'source_sha256': manifest['final_source_sha256']}))


if __name__ == '__main__':
    main()
