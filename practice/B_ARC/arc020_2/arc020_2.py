# https://atcoder.jp/contests/arc020/tasks/arc020_2
# 一行飛ばしでカウントしておけば良い
# もし同色だったら次のいろを使うことにする
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

n, c = read_ints()
n_od = n // 2
n_ev = n - n_od
A, = read_col(n)

cnt_ev = Counter(A[::2])
cnt_od = Counter(A[1::2])
cnt_ev = sorted(cnt_ev.items(), reverse=True, key=itemgetter(1)) + [(-1, 0)]
cnt_od = sorted(cnt_od.items(), reverse=True, key=itemgetter(1)) + [(-1, 0)]

if cnt_ev[0][0] != cnt_od[0][0]:
    ans = cnt_ev[0][1] + cnt_od[0][1]
else:
    ans = cnt_ev[0][1]
    ans += max(cnt_ev[1][1], cnt_od[1][1])

print((n - ans) * c)
