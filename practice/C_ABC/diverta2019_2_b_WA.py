# https://atcoder.jp/contests/diverta2019-2/tasks/diverta2019_2_b
# 問題文はよく読もうね！

# 実際にボールを拾っていくシミュレーションをしていっても十分間に合う
# 実装をあとでじっくり見直そう

from itertools import combinations
from collections import defaultdict
from operator import itemgetter

N = int(input())
points = [tuple(map(int, input().split())) for _ in range(N)]
ans = 0


def is_online(x0, y0, xa, ya, p, q):
    # x0,y0を通る直線がp,qで与えられたときに、xa,yaがその線上にあるか判別
    dx = xa - x0
    dy = ya - y0
    if dx == 0 and dy == 0:
        return True
    elif dx == 0 or dy == 0:
        return False
    # print(x0, y0, xa, ya, p, q)
    return (dy / dx) == (q / p)  # 傾きが同じならば線上にある


def pickballs(balls):
    # cnt most frequent p,q
    global ans

    # 最適なpq探索
    cnts = defaultdict(lambda: 0)
    for pa, pb in combinations(balls, 2):
        dx = pa[0] - pb[0]
        dy = pa[1] - pb[1]
        if dx == 0 or dy == 0:
            continue
        if dx < 0:
            dx *= -1
            dy *= -1
        cnts[(dx, dy)] += 1
    p, q = max(cnts.items(), key=itemgetter(1))[0]

    # 最適な最初のpickball探索
    cnts = defaultdict(lambda: 0)
    for p0 in balls:
        for p1 in balls:
            if p0 == p1:
                continue
            if is_online(p0[0], p0[1], p1[0], p1[1], p, q):
                cnts[p0] += 1
    x, y = max(cnts.items(), key=itemgetter(1))[0]

    # 実際にpickball
    ans += 1
    ret = []
    for p1 in balls:
        if is_online(x, y, p1[0], p1[1], p, q):
            continue
        ret.append(p1)
    return ret


while points:
    points = pickballs(points)
print(ans)
