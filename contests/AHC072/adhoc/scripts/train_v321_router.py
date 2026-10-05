#!/usr/bin/env python3
"""v321の固定した特徴・費用学習。保存済み500件だけを読み、solverは実行しない。"""
import collections
import hashlib
import json
import math
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
FEATURES = ['N', 'K', 'M', 'floors', 'density', 'wall_fraction',
            'largest_color_fraction', 'color_concentration', 'M_per_K',
            'singleton_color_fraction', 'mean_home_distance', 'max_home_distance',
            'mean_nest_distance', 'dead_end_fraction', 'mean_degree',
            'mean_straight_reach', 'mean_nearest_same_color_manhattan']


def sha(data):
    return hashlib.sha256(data).hexdigest()


def features(text):
    tokens = text.split()
    N, K = map(int, tokens[:2])
    grid = tokens[2:]
    assert len(grid) == N and all(len(row) == N for row in grid)
    floor = [(y, x) for y in range(N) for x in range(N) if grid[y][x] != '#']
    allowed = set(floor)
    neighbors = {p: [(p[0]+dy, p[1]+dx) for dy, dx in [(-1, 0), (1, 0), (0, -1), (0, 1)]
                     if (p[0]+dy, p[1]+dx) in allowed] for p in floor}
    slimes = [[] for _ in range(K)]
    nests = [None]*K
    for y, x in floor:
        c = grid[y][x]
        if 'a' <= c <= 'l':
            slimes[ord(c)-ord('a')].append((y, x))
        elif 'A' <= c <= 'L':
            nests[ord(c)-ord('A')] = (y, x)
    assert all(n is not None for n in nests)
    distances = []
    for nest in nests:
        distance = {nest: 0}
        q = collections.deque([nest])
        while q:
            p = q.popleft()
            for v in neighbors[p]:
                if v not in distance:
                    distance[v] = distance[p]+1
                    q.append(v)
        assert len(distance) == len(floor)
        distances.append(distance)
    counts = [len(s) for s in slimes]
    M, F = sum(counts), len(floor)
    assert M > 0 and F > K
    home = [distances[g][p] for g in range(K) for p in slimes[g]]
    nest_pairs = [distances[g][nests[h]] for g in range(K) for h in range(g+1, K)]
    reach = 0
    for y, x in floor:
        for dy, dx in [(-1, 0), (1, 0), (0, -1), (0, 1)]:
            for step in range(1, 9):
                if (y+step*dy, x+step*dx) not in allowed:
                    break
                reach += 1
    nearest = 0
    for group in slimes:
        for p in group:
            # 同色の相手がいない個体は0。巣は最近傍候補に含めない。
            nearest += min((abs(p[0]-q[0])+abs(p[1]-q[1]) for q in group if p != q), default=0)
    return [N, K, M, F, M/(F-K), 1-F/(N*N), max(counts)/M,
            sum((c/M)**2 for c in counts), M/K, sum(c == 1 for c in counts)/K,
            sum(home)/M, max(home), sum(nest_pairs)/len(nest_pairs),
            sum(len(neighbors[p]) == 1 for p in floor)/F,
            sum(len(neighbors[p]) for p in floor)/F, reach/(4*F), nearest/M]


def fold_indices(rows, indices, count):
    ordered = sorted(indices, key=lambda i: (rows[i]['sha256'], rows[i]['case']))
    return [ordered[f::count] for f in range(count)]


def shortlist(rows, indices, candidates, limit=3):
    def total(c):
        return sum(rows[i]['scores'][c] for i in indices)
    chosen = ['v111']
    best = min(candidates, key=lambda c: (total(c), c))
    if best not in chosen:
        chosen.append(best)
    while len(chosen) < min(limit, len(candidates)):
        best = min((c for c in candidates if c not in chosen),
                   key=lambda c: (sum(min(rows[i]['scores'][v] for v in chosen+[c]) for i in indices), c))
        chosen.append(best)
    return chosen


def fit(rows, indices, candidates, depth, min_leaf):
    totals = {c: sum(rows[i]['scores'][c] for i in indices) for c in candidates}
    choice = min(candidates, key=lambda c: (totals[c], c))
    leaf = {'expert': choice, 'count': len(indices), 'train_cost': totals[choice]}
    if depth == 0 or len(indices) < 2*min_leaf:
        return leaf
    best = None
    for feature in range(len(FEATURES)):
        values = sorted(rows[i]['features'][feature] for i in indices)
        thresholds = set()
        for decile in range(1, 10):
            pos = decile*(len(values)-1)/10
            lo, hi = math.floor(pos), math.ceil(pos)
            thresholds.add(values[lo]+(pos-lo)*(values[hi]-values[lo]))
        for threshold in sorted(thresholds):
            left = [i for i in indices if rows[i]['features'][feature] <= threshold]
            right = [i for i in indices if rows[i]['features'][feature] > threshold]
            if min(len(left), len(right)) < min_leaf:
                continue
            cost = sum(min(sum(rows[i]['scores'][c] for i in group) for c in candidates)
                       for group in (left, right))
            key = (cost, feature, threshold)
            if best is None or key < best[0]:
                best = (key, left, right)
    if best is None or best[0][0] >= totals[choice]:
        return leaf
    (cost, feature, threshold), left, right = best
    return {'feature': feature, 'name': FEATURES[feature], 'threshold': threshold,
            'count': len(indices), 'leaf_cost': totals[choice], 'split_cost': cost,
            'left': fit(rows, left, candidates, depth-1, min_leaf),
            'right': fit(rows, right, candidates, depth-1, min_leaf)}


def predict(tree, x):
    while 'expert' not in tree:
        tree = tree['left'] if x[tree['feature']] <= tree['threshold'] else tree['right']
    return tree['expert']


def tune(rows, indices, candidates, count, configs):
    folds = fold_indices(rows, indices, count)
    scores = []
    audit = []
    for depth, minimum in configs:
        total = 0
        for held in folds:
            held_set = set(held)
            train = [i for i in indices if i not in held_set]
            pool = shortlist(rows, train, candidates)
            tree = fit(rows, train, pool, depth, minimum)
            total += sum(rows[i]['scores'][predict(tree, rows[i]['features'])] for i in held)
            audit.append({'depth': depth, 'min_leaf': minimum,
                          'train': [rows[i]['case'] for i in train],
                          'held': [rows[i]['case'] for i in held], 'candidates': pool})
        scores.append({'depth': depth, 'min_leaf': minimum, 'held_total': total})
    best = min(scores, key=lambda r: (r['held_total'], r['depth'], -r['min_leaf']))
    return best, scores, audit


def comparison(scores, baseline, reference):
    return {'sum': sum(scores), 'mean': sum(scores)/len(scores),
            'delta': sum(scores)-sum(baseline),
            'win': sum(a < b for a, b in zip(scores, baseline)),
            'tie': sum(a == b for a, b in zip(scores, baseline)),
            'loss': sum(a > b for a, b in zip(scores, baseline)),
            'relative': 100*sum(r/s for r, s in zip(reference, scores))/len(scores),
            'relative_delta': 100*sum(r/a-r/b for r, a, b in zip(reference, scores, baseline))/len(scores)}


def main():
    run = Path(sys.argv[1]).resolve()
    manifest = json.loads((run/'manifest.json').read_text())
    assert not (run/'cv_result.json').exists(), 'CV is already frozen'
    candidates = sorted(manifest['candidates'])
    for v, bin_name in manifest['candidates'].items():
        assert sha((ROOT/'src/bin'/f'{bin_name}.cpp').read_bytes()) == manifest['source_sha256'][v]
    saved = {v: {} for v in candidates}
    for line in (ROOT/'results/eval_records.jsonl').open():
        record = json.loads(line)
        for v in candidates:
            if (record['label'] == manifest['baseline_labels']['validation1'][v]
                    and record['bin'] == manifest['candidates'][v]):
                assert record['status'] == 'ok' and record['local'] is True
                assert record['input_dir'] == 'tools/validation1'
                assert record['case_name'] not in saved[v], 'duplicate saved record'
                assert 0 < record['score'] < 100000
                saved[v][record['case_name']] = int(record['score'])
    rows = []
    for path in sorted((ROOT/'tools/validation1').glob('*.txt')):
        data = path.read_bytes()
        assert sha(data) == manifest['input_sha256']['validation1'][path.name]
        rows.append({'case': path.name, 'sha256': sha(data), 'features': features(data.decode()),
                     'scores': {v: saved[v][path.name] for v in candidates}})
    assert len(rows) == 500 and all(len(s) == 500 for s in saved.values())
    indices = list(range(len(rows)))
    configs = [(d, n) for d in manifest['depths'] for n in manifest['min_leaves']]
    outer = []
    audit = []
    predictions = {}
    for f, held in enumerate(fold_indices(rows, indices, manifest['outer_folds'])):
        held_set = set(held)
        train = [i for i in indices if i not in held_set]
        config, tuning, inner_audit = tune(rows, train, candidates, manifest['inner_folds'], configs)
        pool = shortlist(rows, train, candidates)
        tree = fit(rows, train, pool, config['depth'], config['min_leaf'])
        constant = min(candidates, key=lambda c: (sum(rows[i]['scores'][c] for i in train), c))
        for i in held:
            assert i not in predictions
            expert = predict(tree, rows[i]['features'])
            predictions[i] = {'case': rows[i]['case'], 'fold': f, 'expert': expert,
                              'score': rows[i]['scores'][expert], 'constant': constant,
                              'constant_score': rows[i]['scores'][constant]}
        outer.append({'fold': f, 'candidates': pool, 'chosen': config, 'tuning': tuning, 'tree': tree,
                      'train': [rows[i]['case'] for i in train], 'held': [rows[i]['case'] for i in held]})
        # 内側の全ケースが外側訓練に含まれることを保存前に検査する。
        train_names = {rows[i]['case'] for i in train}
        for a in inner_audit:
            assert set(a['train']).isdisjoint(a['held'])
            assert set(a['train']) | set(a['held']) == train_names
        audit.append(inner_audit)
    config, tuning, full_audit = tune(rows, indices, candidates, manifest['outer_folds'], configs)
    pool = shortlist(rows, indices, candidates)
    tree = fit(rows, indices, pool, config['depth'], config['min_leaf'])
    predicted = [predictions[i]['score'] for i in indices]
    reference = [manifest['reference']['validation1'][r['case']] for r in rows]
    baselines = {v: comparison(predicted, [r['scores'][v] for r in rows], reference) for v in candidates}
    result = {'features': FEATURES, 'outer': outer, 'final_config': config,
              'final_tuning': tuning, 'final_candidates': pool, 'final_tree': tree,
              'oof_comparison': baselines,
              'oof_vs_train_selected_constant': comparison(predicted,
                    [predictions[i]['constant_score'] for i in indices], reference),
              'oof_experts': dict(collections.Counter(p['expert'] for p in predictions.values())),
              'training_oracle_sum': sum(min(r['scores'][v] for v in pool) for r in rows),
              'source_sha256': manifest['source_sha256'], 'method': 'pre-registered nested CV'}
    for name, data in [('training_data.json', rows), ('oof_predictions.json', list(predictions.values())),
                       ('cv_audit.json', {'outer_inner': audit, 'final': full_audit}),
                       ('cv_result.json', result), ('router.json', {'features': FEATURES, 'tree': tree})]:
        (run/name).write_text(json.dumps(data, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps({k: v for k, v in result.items() if k not in ('outer', 'features')}, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
