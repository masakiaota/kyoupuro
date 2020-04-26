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
# https://atcoder.jp/contests/arc098/tasks/arc098_b

# xorは桁の繰り上がりがない足し算！→範囲で1が1回以下出てくるような範囲は大丈夫
# 1が2回以上出てこない最長の範囲ならどのようにl,rを選んでも題意が成り立つ
# 尺取法か,累積和で、足し算とxorが成り立つ最長の範囲を取ってくれば良い
# Alに対して範囲が成り立つ最長のArを探せば良い
# だけど一つずらしても最長の区間が存在する場合は？
#

N = read_a_int()
A = read_ints()
# a_sum = 0
# a_xor = 0
# ans = 0
# r = 0
# for l in range(N):
#     while r < N and a_sum + A[r] == a_xor ^ A[r]:  # 条件にすべきなのは次の状態
#         a_sum += A[r]
#         a_xor ^= A[r]
#         r += 1
#         # print(l, r, a_sum, a_xor)
#     ans += r - l
#     # print(l, r)
#     a_sum -= A[l]
#     a_xor ^= A[l]
# print(ans)

# 一般化尺取法


def two_pointers(ls: list):
    '''すべてのlに対して、条件is_okをみたすls[l:r]の中で
    r - lが最大になるような(l,r)の集合を返す'''
    n_ls = len(ls)
    ret = []

    def append(r, pre_states):
        '''状態にls[r]を考慮して更新する'''
        # 問題によって自分で定義
        sum_state, xor_state = pre_states
        sum_state += ls[r]
        xor_state ^= ls[r]
        return (sum_state, xor_state)

    def pop(l, pre_states):
        '''状態からls[l]を抜く更新をする'''
        # 問題によって自分で定義
        sum_state, xor_state = pre_states
        sum_state -= ls[l]
        xor_state ^= ls[l]
        return (sum_state, xor_state)

    def is_ok(r, pre_states):
        '''満たしていてほしい条件'''
        # 問題によって自分で定義
        states = append(r, pre_states)
        sum_state, xor_state = states
        return sum_state == xor_state

    r = 0
    states = (0, 0)
    for l in range(n_ls):
        while r < n_ls and is_ok(r, states):
            # 更新式
            states = append(r, states)
            r += 1
        ret.append((l, r))
        # 抜けるときの更新
        states = pop(l, states)
    return ret


idxs = two_pointers(A)
ans = 0
for l, r in idxs:
    ans += r - l
print(ans)
