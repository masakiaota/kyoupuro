# https://atcoder.jp/contests/abc133/tasks/abc133_e

# あー解説あたまいい
# そのノードで塗れる色の候補を持つのではなくて、
# そのノードの子で塗れる通りの数を持っておく
# あるノードの子で塗れる通りの数は、親とそのノードで塗る色の通りも確定してると考えると、(K-2)P(子の数)となる。(自身が根のときは自身の色の通りも考慮して、(K)P(子の数+1)となる)
# これをdfsで実装すれば良い

import sys
from collections import defaultdict

sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


MOD = 10**9 + 7

N, K = read_ints()
tree = defaultdict(lambda: [])
for _ in range(N - 1):
    a, b = read_ints()
    a, b = mina(a, b)
    tree[a].append(b)
    tree[b].append(a)


def perm_mod(n, r, mod=MOD):
    '''nPrをmodを取って返す'''
    if n < r:  # そんな通りはありえない
        return 0
    ret = 1
    for _ in range(r):
        ret *= n
        ret %= mod
        n -= 1
    return ret


def dfs(u, p):  # 現在のuと親のp
    if len(tree[u]) == 1 and tree[u][0] == p:
        # 葉なので終了
        return 1
    ret = 1  # その地点までの通りの数
    for to in tree[u]:
        if to == p:
            continue
        ret *= dfs(to, u)
        ret %= MOD
    if p == -1:
        ret *= perm_mod(K, len(tree[u]) + 1)
    else:
        ret *= perm_mod(K - 2, len(tree[u]) - 1)

    return ret % MOD


print(dfs(0, -1))
