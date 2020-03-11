# https://atcoder.jp/contests/abc099/tasks/abc099_d
# さまざまな合わせ技
# ダイクストラ、全探索

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため



# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations

# from scipy.sparse.csgraph import dijkstra
# from scipy.sparse import csr_matrix, lil_matrix
# 0 まずはダイクストラでX→Yの最小コストを求めておく #えーーーーこれ想定されてないの？
# 1 座標ごとに3つのグループわけができる(n=1のときだけ場合分け
# 2 グループごとに色をカウント
# 3 各グループの色を仮定して全探索

N, C = read_ints()
D = read_matrix(C)
# D = csr_matrix(D) #これやるとかえって不正解とかうそやろ
# D = dijkstra(D)


# 1 座標のグループわけ
Cmap = read_matrix(N)

if N == 1:
    print(0)
    exit()

lsbymod = defaultdict(lambda: [])
for i in range(N):
    for j in range(N):
        lsbymod[(i + j + 2) % 3].append(Cmap[i][j] - 1)

# グループごとに色のカウント
for k, v in lsbymod.items():
    lsbymod[k] = Counter(v)


def ret_score(*arg):
    # 各仮定から違和感を計算
    ret = 0
    for i in range(3):
        for k, v in lsbymod[i].items():
            ret += v * D[k][arg[i]]
    return ret


# 各グループの色を仮定して全探索
ans = 2**63
for a, b, c in permutations(range(C), 3):
    # print(a, b, c, ret_score(a, b, c))
    ans = min(ans, ret_score(a, b, c))

print(ans)
