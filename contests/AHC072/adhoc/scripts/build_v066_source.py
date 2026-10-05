#!/usr/bin/env python3
"""v059へv062の登録済み短縮差分だけを移す。探索は行わない。"""
from pathlib import Path
import hashlib, json

ROOT = Path(__file__).resolve().parents[2]
AUDIT = ROOT / 'adhoc/v066_audit'
NAME = 'v066_repair_deferred'

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    assert not (AUDIT / '20260930_submit/manifest.json').exists(), '凍結後は再生成しない'
    parent = ROOT / 'src/bin/v059_repair_priority.cpp'
    common = ROOT / 'src/bin/v057_search_reductions.cpp'
    donor = ROOT / 'src/bin/v062_deferred_passenger.cpp'
    assert sha(parent) == 'a940c3b2c90e33d4425f531cacfd85286ff170a285fa09a97667c4bad121e002'
    registered = json.loads((ROOT / 'adhoc/v062_audit/registered_changes.json').read_text())
    assert sha(common) == registered['parent_sha256']
    restored = donor.read_text()
    for c in reversed(registered['changes']):
        assert restored.count(c['new']) == 1
        restored = restored.replace(c['new'], c['old'], 1)
    assert restored == common.read_text()
    changes = [dict(old='// v059_repair_priority.cpp', new=f'// {NAME}.cpp'), *registered['changes'][1:]]
    source = parent.read_text()
    for c in changes:
        assert source.count(c['old']) == 1
        source = source.replace(c['old'], c['new'], 1)
    # Keep both donors untouched, including schedules, RNG calls and counters.
    (ROOT / 'src/bin' / f'{NAME}.cpp').write_text(source)
    AUDIT.mkdir(parents=True, exist_ok=True)
    (AUDIT / 'registered_changes.json').write_text(json.dumps(changes, ensure_ascii=False, indent=2) + '\n')
    checker = (ROOT / 'adhoc/bin/check_v062_deferred_passenger.cpp').read_text()
    checker = checker.replace('v062_deferred_passenger', NAME).replace('v062', 'v066')
    (ROOT / 'adhoc/bin/check_v066_repair_deferred.cpp').write_text(checker)
    print(f'{NAME}: v059 + unchanged v062 reduction ({len(changes)} registered changes)')

if __name__ == '__main__':
    main()
