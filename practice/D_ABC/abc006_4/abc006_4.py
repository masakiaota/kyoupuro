# https://atcoder.jp/contests/abc006/tasks/abc006_4
# c0,c1,c2,c3...c(N-1)があって、i番目を抜いてj番目に挿入できる
# 最終的にc0<=c1<=c2...になりたい
# じゃもともと関係性を満たすものを移動する必要はない。
# 移動が必要なカードは一回の移動で必ず最適な位置に挿入できる。
# よってN-LISが答え


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate
from bisect import bisect_right, bisect_left


def read_a_int():
    return int(read())


N = read_a_int()
dp = []
for _ in ra(N):
    c = read_a_int()
    idx = bisect_left(dp, c)  # c以上になる左端のidx
    if idx == len(dp):
        dp.append(c)
    else:
        dp[idx] = c  # aに更新
print(N - len(dp))
