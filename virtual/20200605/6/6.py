import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


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
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/diverta2019/tasks/diverta2019_c

# B*A,*A,B*,*の4つに分類される。
# *に関しては事前に全部カウントしておけば良い
# B*A同士で作ったほうが有利(個数-1増える)
# *A,B*は 少ない方の個数増える
# 後者2つはコーナーケースに注意(つまり、B*Aがデータセットにある場合はさらに+1できる)(消化しきれない場合)

N = int(input())
base = 0
b_ = 0
_a = 0
b_a = 0
for _ in range(N):
    s = read()[:-1]
    if s.startswith('B') and s.endswith('A'):
        b_a += 1
    elif s.startswith('B'):
        b_ += 1
    elif s.endswith('A'):
        _a += 1
    base += s.count('AB')

ans = base
if b_a:
    ans += b_a - 1

if b_ and b_a:
    ans += 1
    b_ -= 1  # b_aの端に結合

if _a and b_a:
    ans += 1
    _a -= 1  # b_aの端に結合

ans += min(b_, _a)
print(ans)

