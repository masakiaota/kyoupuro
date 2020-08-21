import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd

N = a_int()
P = list(map(float, readline().split()))

'''
dp[i,j]...表がちょうどj回出る確率、コイン[:i]を考慮したとき
dp[i+1,j]+=dp[i,j]*(1-P[i]) #配る、裏だった場合
dp[i+1,j+1]+= dp[i,j]*P[i] #表立った場合
'''
dp = [[0] * (N + 2) for _ in range(N + 2)]
dp[0][0] = 1
for i in range(N):
    for j in range(N):
        dp[i + 1][j] += dp[i][j] * (1 - P[i])  # 配る、裏だった場合
        dp[i + 1][j + 1] += dp[i][j] * P[i]  # 表立った場合

# 表がN//2+1以上回の確率を合計する
print(sum(dp[N][N // 2 + 1:]))
# print(*dp, sep='\n')
