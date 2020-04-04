# https://atcoder.jp/contests/ddcc2020-qual/tasks/ddcc2020_qual_c

# 横で切り分けてから、いちごが無い行は上の行のをコピる
# 実装が重い


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
rr = range


def read_ints():
    return list(map(int, read().split()))


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from fractions import gcd

c = 0


def ret_row(s: list):
    ret = [-1] * W
    l = 0
    global c
    for r, ss in enumerate(s):
        if ss == 1:
            c += 1
            for j in rr(l, r + 1):
                ret[j] = c
            l = r + 1
    for j in rr(l, W):
        ret[j] = c
    # assert -1 not in ret  # あとで消す
    return ret


H, W, K = read_ints()
S = read_map_as_int(H)

ans = []
for s in S:
    if 1 not in s:
        ans.append([-1] * W)
    else:
        ans.append(ret_row(s))

if ans[0][0] == -1:
    i = 0
    while ans[i][0] == -1:
        i += 1
    for j in rr(W):
        ans[0][j] = ans[i][j]


for i in rr(1, H):
    if ans[i][0] != -1:
        continue
    for j in rr(W):
        ans[i][j] = ans[i - 1][j]


for a in ans:
    print(*a)
