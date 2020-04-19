# https://atcoder.jp/contests/abc034/tasks/abc034_d
# 直感で考えると濃度でソートして濃い方からK個選べばいい気がするが？
# ナップサックでも解けそうだが？

# ソートして濃いのを選ぶのは？←だめ、濃くても量が多いと混ぜ合わせたときに濃くなりにくく成ることがある
# というよりは量が少なくて濃い食塩水を薄めることになる という言い方のほうがいいかな


# dp_p[k]...ちょうどk個の食塩水を選んだときの最高の濃度
# dp_w[k]...上記pに対応する食塩水の質量
# 両テーブルを更新していく

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


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
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

N, K = read_ints()
W, P = read_col(N)

# dp_p[k]...ちょうどk個の食塩水を選んだときの最高の濃度
# dp_w[k]...上記pに対応する食塩水の質量
# 両テーブルを更新していく

dp_p = [-INF] * (K + 1)
dp_w = [-INF] * (K + 1)
is_used = [False] * (N)

# 初期値決める →0こ考慮すべて0、
dp_p[0] = 0
dp_w[0] = 0

for k in ra(K):
    now_p = dp_p[k]
    now_w = dp_w[k]
    p_max = -1
    for i in ra(N):
        if is_used[i]:
            continue
        candi_p = P[i]
        candi_w = W[i]
        new_p = (now_p * now_w + candi_p * candi_w) / (now_w + candi_w)
        if new_p > p_max:
            p_max = new_p
            w_max = now_w + candi_w
            idx = i
    is_used[idx] = True
    dp_p[k + 1] = p_max
    dp_w[k + 1] = w_max

# print(dp_p)
print(dp_p[K])

# ちゃんと定式化してDPを考えた！えらい！
# なお、嘘解法らしい...
