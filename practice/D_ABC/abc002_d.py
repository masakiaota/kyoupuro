# https://atcoder.jp/contests/abc002/tasks/abc002_4
# お互いに知っているかだから人づてはだめ→unionfindじゃない
# 全結合になっている領域が良い


import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def iter_p_adic(p, length):
    '''
    連続して増加するp進数をリストとして返す。lengthはリストの長さ
    return
    ----------
    所望のp進数リストを次々返してくれるiterator
    '''
    from itertools import product
    tmp = [range(p)] * length
    return product(*tmp)


from collections import defaultdict
from itertools import combinations
# ビット全探索で愚直に確かめる
N, M = read_ints()
graph = defaultdict(lambda: [])
for _ in range(M):
    x, y = read_ints()
    graph[x - 1].append(y - 1)
    graph[y - 1].append(x - 1)


def g_candi(perm):
    candi = []
    for i, p in enumerate(perm):
        if p == 0:  # この国会議員は使わない
            continue
        candi.append(i)
    return candi


ans = 0
for perm in iter_p_adic(2, N):
    n = sum(perm)
    for a, b in combinations(g_candi(perm), 2):
        if b not in graph[a]:  # ng
            n = -1
            break
    ans = max(n, ans)
print(ans)
