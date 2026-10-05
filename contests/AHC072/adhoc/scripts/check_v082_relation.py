#!/usr/bin/env python3
"""v082追加14特徴を、保存操作の独立した個体追跡で照合する。探索は行わない。"""
import argparse
import hashlib
import itertools
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'results/nn_rank/v077/20261002T021222_studio'
CONTROL = ROOT / 'results/nn_rank/v080/20261002T113935_studio'
DIRECTIONS = ((-1, 0), (1, 0), (0, -1), (0, 1))


def replay(input_text, state):
    """初期床IDを保持し、各便の乗客・下段と各個体の乗車時刻を返す。"""
    lines = input_text.splitlines()
    N, K = map(int, lines[0].split())
    grid = lines[1:N + 1]
    assert len(grid) == N and all(len(row) == N for row in grid)
    cells = [(i, j) for i in range(N) for j in range(N) if grid[i][j] != '#']
    board = {cell: [] for cell in cells}
    colors, nests = {}, {}
    for identity, cell in enumerate(cells):
        char = grid[cell[0]][cell[1]]
        if char.islower():
            colors[identity] = char
            board[cell].append(identity)
        elif char.isupper():
            nests[cell] = char.lower()
    history = {identity: [] for identity in colors}
    travel = {identity: 0 for identity in colors}
    events = []
    for time, move in enumerate(state['current']):
        p, k, direction, distance = map(int, move)
        cell = cells[p]
        assert 0 <= k < len(board[cell]), (time, 'source', move)
        assert 0 <= direction < 4 and 1 <= distance <= k + 1, (time, 'jump', move)
        di, dj = DIRECTIONS[direction]
        for step in range(1, distance + 1):
            assert (cell[0] + di * step, cell[1] + dj * step) in board, (time, 'wall')
        target = (cell[0] + di * distance, cell[1] + dj * distance)
        passengers = tuple(board[cell][k:])
        supporters = tuple(board[cell][:k])
        assert len(board[target]) + len(passengers) <= 8, (time, 'capacity')
        events.append((time, distance, k + 1 - distance, passengers, supporters))
        for identity in passengers:
            history[identity].append(time)
            travel[identity] += distance
        board[cell] = list(supporters)
        board[target].extend(reversed(passengers))
        for position in (cell, target):
            while board[position] and colors[board[position][-1]] == nests.get(position):
                board[position].pop()
    return {'events': events, 'history': history, 'travel': travel,
            'T': len(state['current']), 'M': len(colors), 'remaining': board}


def features(replayed, candidate_ids):
    """列32〜45。入力の候補順によらず、候補試行後の情報を使わない。"""
    ids = tuple(candidate_ids)
    selected = set(ids)
    assert ids and len(ids) == len(selected)
    assert selected <= replayed['history'].keys()
    count, length = len(ids), max(1, replayed['T'])
    raw = [0.0] * 14
    partners = set()
    slack_weight = 0
    for _, distance, slack, passengers, supporters in replayed['events']:
        inside = [p for p in passengers if p in selected]
        outside = [p for p in passengers if p not in selected]
        if inside:
            if outside:
                raw[5] += len(inside)
            raw[6] += len(outside)
            partners.update(outside)
        if distance <= 1:
            continue
        lost = sum(p in selected for p in supporters)
        raw[4] += len(inside) * (len(supporters) - lost)
        if outside:
            missing = max(0, lost - slack)
            raw[2] += missing
            if missing:
                raw[3] += len(outside)
            if lost:
                raw[0] += 1
                raw[1] += lost * len(outside)
                raw[12] += max(0, slack - lost) * len(outside)
                slack_weight += len(outside)
    for i in (1, 2, 3, 4, 5, 6):
        raw[i] /= count
    raw[7] = len(partners) / max(1, replayed['M'])
    raw[8] = sum(replayed['travel'][p] for p in ids) / count
    times = [replayed['history'][p] for p in ids]
    raw[9] = sum(t[0] if t else 0 for t in times) / count / length
    raw[10] = sum(t[-1] if t else 0 for t in times) / count / length
    raw[11] = max((b-a for t in times for a, b in zip(t, t[1:])), default=0) / length
    raw[12] = raw[12] / slack_weight if slack_weight else 0.0
    # 乗車のない個体を含む対も分母へ残し、重なりだけを0とする。
    pairs = list(itertools.combinations(times, 2))
    raw[13] = sum(max(0, min(a[-1], b[-1]) - max(a[0], b[0]))
                  if a and b else 0 for a, b in pairs) / max(1, len(pairs)) / length
    return raw


def reference(input_text, state, candidate_ids):
    return features(replay(input_text, state), candidate_ids)


def reference_state(input_text, state):
    replayed = replay(input_text, state)
    return [{'rank': c['rank'], 'raw14': features(replayed, c['ids'])}
            for c in sorted(state['candidates'], key=lambda c: c['rank'])]


def assert_vector(actual, expected):
    assert len(actual) == len(expected) == 14
    for column, (a, b) in enumerate(zip(actual, expected), 32):
        assert math.isfinite(a) and abs(a-b) <= 1e-10 + 1e-12 * abs(b), (column, a, b)


def self_test():
    # ID6を底にID7,8,11を積み、時刻4に上の2匹を3マス飛ばす。
    text = '5 1\n.....\n.aaa.\n.a...\n.....\n....A\n'
    moves = [[7,0,2,1], [8,0,2,1], [7,0,2,1], [11,0,0,1], [6,2,3,3]]
    state = {'current': moves}
    assert_vector(reference(text, {'current': []}, [6]), [0.0] * 14)
    records = replay(text, state)
    assert records['history'] == {6: [], 7: [0], 8: [1,2,4], 11: [3,4]}
    assert_vector(features(records, [6]), [1,2,1,2,0,0,0,0,0,0,0,0,0,0])
    assert_vector(features(records, [6,7]), [1,2,1,1,0,0,0,0,.5,0,0,0,0,0])
    assert_vector(features(records, [8]), [0,0,0,0,2,1,1,.25,5,.2,.8,.4,0,0])
    assert_vector(features(records, [8,11]), [0,0,0,0,2,0,0,0,4.5,.4,.8,.4,0,.2])
    assert_vector(features(records, [6,7,8,11]), [0,0,0,0,0,0,0,0,2.5,.2,.4,.4,0,1/30])
    # 乗車0回/1回の時刻・間隔と、期間の重なり0を検査する。
    assert_vector(reference(text, {'current': moves[:1]}, [6,7]),
                  [0,0,0,0,0,0,0,0,.5,0,0,0,0,0])
    # 下段3匹、距離2: 余裕2から1匹を除くと、非負余裕1が残る。
    one = replay(text, {'current': moves[:4] + [[6,3,3,2]]})
    assert_vector(features(one, [6]), [1,1,0,0,0,0,0,0,0,0,0,0,1,0])
    # 別の長距離便の残存乗客数で余裕を加重する。2便を独立な履歴とする。
    weighted = {'events': [(0,2,3,(8,11),(6,7,12,13)),
                           (1,3,1,(8,),(6,7,12))],
                'history': {6: [],7: [],8: [0,1],11: [0],12: [],13: []},
                'travel': {6: 0,7: 0,8: 5,11: 2,12: 0,13: 0}, 'T': 2, 'M': 6}
    assert_vector(features(weighted, [6]), [2,3,0,0,0,0,0,0,0,0,0,0,4/3,0])
    # 発射元と着地先の帰巣、壁による床IDの圧縮、上下反転を独立に検査する。
    home = '3 2\n.a#\nbAB\n...\n'
    ended = replay(home, {'current': [[1,0,2,1], [2,0,0,1], [0,0,1,1], [2,0,3,1], [3,1,3,1]]})
    assert not any(ended['remaining'].values())
    return {'passed': True, 'manual_feature_cases': 9, 'home_and_floor_id': True}


def read_probe(path):
    lines = iter(path.read_text().splitlines())
    first = next(lines)
    N = int(first.split()[0])
    input_text = '\n'.join([first] + [next(lines) for _ in range(N)]) + '\n'
    states = []
    for _ in range(int(next(lines))):
        phase, stagnant, progress, support, ride = next(lines).split()
        state = {'phase': int(phase)}
        for key in ('current', 'best'):
            state[key] = [list(map(int, next(lines).split())) for _ in range(int(next(lines)))]
        candidates = []
        for _ in range(int(next(lines))):
            parts = next(lines).split()
            rank, identity_hash, priority, count = parts[:4]
            ids = list(map(int, parts[4:]))
            assert len(ids) == int(count)
            candidates.append({'rank': int(rank), 'hash': identity_hash, 'ids': ids})
        state['candidates'] = candidates
        states.append(state)
    assert not list(lines)
    return input_text, states


def check_run(run, *, source=SOURCE, control=CONTROL, replay_only=False):
    """再生だけ、または抽出済みextra_raw.npyとの全保存probe照合。"""
    manifest = json.loads((run / 'probe_manifest.json').read_text())
    arrays = None
    if not replay_only:
        import numpy as np
        extra = np.load(run / 'data/extra_raw.npy', mmap_mode='r')
        # 入力・候補の正本はv080の凍結配列。新データ側の並べ替えを許さない。
        ranks = np.load(control / 'data/ranks.npy', mmap_mode='r')
        mask = np.load(control / 'data/mask.npy', mmap_mode='r')
        done = np.load(control / 'data/done.npy', mmap_mode='r')
        extracted_done = np.load(run / 'data/done.npy', mmap_mode='r')
        assert extra.shape == (262144, 32, 14) and extra.dtype == np.float64
        assert extracted_done.shape == (65536,) and extracted_done.dtype == np.bool_
        arrays = extra, ranks, mask, done, extracted_done
    count = 0
    max_error = 0.0
    for item in manifest:
        stem = item['stem']
        input_text, states = read_probe(run / 'saved_probes' / (stem + '.input'))
        original = source / item['path']
        assert hashlib.sha256(original.read_bytes()).hexdigest() == item['sha256']
        assert input_text == original.read_text()
        originals = {s['phase']: s for s in map(json.loads,
                     (source / 'cases' / stem / 'states.jsonl').read_text().splitlines())}
        for state in states:
            parent = originals[state['phase']]
            assert state['current'] == parent['current']
            assert state['best'] == parent['best']
            assert state['candidates'] == sorted(parent['candidates'], key=lambda c: c['rank'])
            expected = reference_state(input_text, state)
            if arrays is not None:
                extra, ranks, mask, done, extracted_done = arrays
                index = item['index'] * 4 + state['phase']
                assert done[item['index']], ('unfinished_control_input', item['index'])
                assert extracted_done[item['index']], ('unfinished_extra_input', item['index'])
                valid = [i for i, value in enumerate(mask[index]) if value]
                assert len(valid) == len(expected)
                assert [int(ranks[index, i]) for i in valid] == [row['rank'] for row in expected]
                for slot, row in zip(valid, expected):
                    actual = list(map(float, extra[index, slot]))
                    assert_vector(actual, row['raw14'])
                    max_error = max(max_error, max(abs(a-b) for a,b in zip(actual,row['raw14'])))
            count += len(expected)
    return {'passed': True, 'inputs': len(manifest), 'states': len(manifest)*4,
            'candidates': count, 'compared_raw14': not replay_only, 'max_abs_error': max_error,
            'comparison': 'raw_float64_tolerance' if not replay_only else 'replay_only',
            'input_and_current_and_candidate_ids_match_v077': True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', type=Path)
    parser.add_argument('--source', type=Path, default=SOURCE)
    parser.add_argument('--control', type=Path, default=CONTROL)
    parser.add_argument('--replay-only', action='store_true')
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    result = {'artificial': self_test()}
    if args.run:
        result['saved'] = check_run(args.run, source=args.source, control=args.control,
                                    replay_only=args.replay_only)
    if args.output:
        args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
