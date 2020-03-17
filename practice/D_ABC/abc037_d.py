# https://atcoder.jp/contests/abc037/tasks/abc037_d
# 普通に自力不足だった グラフ構造でdpを更新していく。dpは再帰メモ化で書く。

import sys
sys.setrecursionlimit(1 << 25)
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


MOD = 10**9 + 7
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
# そのマスをスタートにしたときの経路の数dp[i,j]を考える.
# 答えはズバリsum(dp)である。
# ではどうやってdp[i,j]を計算する？dp[i,j]はそこにたどり着ける通りの数+1(自分自身)である。これを深さ優先探索などで計算すれば良い.

H, W = read_ints()
A = read_matrix(H)
dp = [[-1] * W for _ in range(H)]

mv = [(0, 1), (-1, 0), (0, -1), (1, 0)]  # こっちだと2倍ぐらいかかる 通らない
mv = [(0, -1), (1, 0), (0, 1), (-1, 0)]  # こっちだと通る


def dfs(i, j):
    # i,jの周りで自分よりも低い数字のますがあったらそこから通りの数を受け取る
    if dp[i][j] != -1:  # 再帰メモ化
        return dp[i][j]
    ret = 1
    for di, dj in mv:
        ni, nj = i + di, j + dj
        if not (0 <= ni < H and 0 <= nj < W):
            continue
        if A[ni][nj] > A[i][j]:  # 自分よりも大きいマスから通りの数が伝播される
            ret += dfs(ni, nj)
    ret %= MOD
    dp[i][j] = ret  # メモ化
    return ret


ans = 0
for i, j in product(range(H), range(W)):
    ans += dfs(i, j)
    if ans >= MOD:
        ans -= MOD
print(ans)
