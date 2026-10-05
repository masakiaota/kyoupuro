"""盤面座標と個体IDの独立再生による輸送構造の24特徴。"""
import math
import numpy as np
from v089_data import Geometry


def structure(path, codes):
    geo = Geometry(path)
    identities = list(map(int, np.flatnonzero(geo.initial)))
    towers = [[p] if geo.initial[p] else [] for p in range(400)]
    transport = {p: 0 for p in identities}
    support = dict(transport)
    visited = {p: {p} for p in identities}
    revisited, homed, hometimes = set(), set(), []
    totals = dict(carried=0, single=0, bulk=0, mixed=0, colors=0, jumps=0, length=0,
                  lower=0, merges=0, home_moves=0, nonprogress=0, up=0, travel=0, revisits=0)
    initial_distance = sum(float(geo.distance[int(geo.initial[p]), p]) for p in identities)
    for step, code in enumerate(codes):
        p, k, d, length = code & 511, (code >> 9) & 7, (code >> 12) & 3, (code >> 14) + 1
        assert 0 <= k < len(towers[p]) and 1 <= length <= k + 1
        di, dj = [(-1, 0), (1, 0), (0, -1), (0, 1)][d]
        i, j = divmod(p, 20)
        for distance in range(1, length + 1):
            ni, nj = i + di * distance, j + dj * distance
            assert 0 <= ni < geo.N and 0 <= nj < geo.N and geo.floor[20 * ni + nj]
        q = 20 * (i + di * length) + j + dj * length
        cargo = towers[p][k:]
        assert len(towers[q]) + len(cargo) <= 8
        varieties = len({int(geo.initial[v]) for v in cargo})
        totals['carried'] += len(cargo)
        totals['single'] += len(cargo) == 1
        totals['bulk'] += len(cargo) >= 4
        totals['mixed'] += varieties > 1
        totals['colors'] += varieties
        totals['jumps'] += length > 1
        totals['length'] += length
        totals['lower'] += k
        totals['merges'] += bool(towers[q])
        totals['travel'] += len(cargo) * length
        if length > 1:
            for v in towers[p][:k]:
                support[v] += 1
        differences = []
        for v in cargo:
            c = int(geo.initial[v])
            difference = float(geo.distance[c, p] - geo.distance[c, q])
            differences.append(difference)
            totals['up'] += max(0, -difference)
            transport[v] += 1
            if q in visited[v]:
                revisited.add(v)
                totals['revisits'] += 1
            visited[v].add(q)
        totals['nonprogress'] += sum(differences) <= 0
        towers[p] = towers[p][:k]
        towers[q].extend(reversed(cargo))
        returned = 0
        for cell in (p, q):
            while towers[cell] and int(geo.initial[towers[cell][-1]]) == int(geo.nest[cell]) + 1:
                v = towers[cell].pop()
                assert v not in homed
                homed.add(v)
                hometimes.append((step + 1) / max(1, len(codes)))
                returned += 1
        totals['home_moves'] += returned > 0
    assert len(homed) == len(identities) and not any(towers)
    t, m = max(1, len(codes)), max(1, len(identities))
    moves = list(transport.values())
    mean = sum(moves) / m
    variance = max(0, sum(v*v for v in moves) / m - mean*mean)
    home_mean = sum(hometimes) / m
    home_var = max(0, sum(v*v for v in hometimes) / m - home_mean*home_mean)
    return [totals['carried']/t/8, totals['single']/t, totals['bulk']/t,
            totals['mixed']/t, totals['colors']/t/8, totals['jumps']/t,
            totals['length']/t/8, totals['lower']/t/8, totals['merges']/t,
            totals['home_moves']/t, m/max(1, totals['home_moves'])/8,
            totals['nonprogress']/t, totals['up']/max(1, totals['travel']),
            totals['travel']/max(1, initial_distance)-1, mean/16, math.sqrt(variance)/16,
            max(moves, default=0)/64, sum(v == 1 for v in moves)/m, len(revisited)/m,
            totals['revisits']/max(1, totals['carried']), sum(v > 0 for v in support.values())/m,
            max(support.values(), default=0)/t, home_mean, math.sqrt(home_var)]
