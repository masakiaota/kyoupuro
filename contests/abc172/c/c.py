import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


class cumsum1d:  # 一次元累積和クラス
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]


def two_pointers(ls: list):
    '''すべてのlに対して、条件is_okをみたすls[l:r]の中で
    r - lが最大になるような(l,r)の集合を返す'''
    n_ls = len(ls)
    ret = []

    def append(r, pre_states):
        '''状態にls[r]を考慮して更新する'''
        # 問題によって自分で定義
        return pre_states + ls[r]
        # 掛け算の例 return pre_state*ls[r]

    def pop(l, pre_states):
        '''状態からls[l]を抜く更新をする'''
        # 問題によって自分で定義
        # 掛け算の例 return pre_state//ls[l]
        return pre_states - ls[l]

    def is_ok(r, pre_states):
        # 問題によって自分で定義
        states = append(r, pre_states)
        # 114以下の最長の範囲が知りたい例 return states<=114
        return states <= K

    r = 0
    states = 0
    for l in range(n_ls):
        while r < n_ls and is_ok(r, states):
            # 更新式
            states = append(r, states)
            r += 1
        ret.append((l, r))
        # 抜けるときの更新
        states = pop(l, states)
    return ret


N, M, K = ints()
A = ints()
B = ints()
AB = list(reversed(A)) + B

ans = 0
for l, r in two_pointers(AB):  # 合計がK以下となる[l,r)
    if l <= N and N <= r:
        ans = max(ans, r - l)
print(ans)
