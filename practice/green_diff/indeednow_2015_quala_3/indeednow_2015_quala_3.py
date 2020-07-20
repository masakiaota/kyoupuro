# https://atcoder.jp/contests/indeednow-quala/tasks/indeednow_2015_quala_3
# 誤読でなければ普通の二分探索では？
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


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def bisect_right_reverse(a, x):
    '''
    reverseにソートされたlist aに対してxを挿入できるidxを返す。
    xが存在する場合には一番右側のidx+1となる。
    '''
    if a[0] < x:
        return 0
    if x <= a[-1]:
        return len(a)
    # 二分探索
    ok = len(a) - 1
    ng = 0
    while (abs(ok - ng) > 1):
        mid = (ok + ng) // 2
        if a[mid] < x:
            ok = mid
        else:
            ng = mid
    return ok


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

N = a_int()
S, = read_col(N)
S.sort(reverse=True)
tmp = []
for s in S:
    if s <= 0:
        break
    tmp.append(s)
S = tmp
# s点以上の人数が1人ならS[0],2人なら[1]というふうに対応する


Q = a_int()
K, = read_col(Q)

if len(S) == 0:
    exit(*[0] * Q, sep='\n')  # コーナーケース対策

ans = []
for k in K:
    # print(bisect_right_reverse(S, S[k - 1]))
    if k == 0:
        ans.append(S[0] + 1)
        continue
    if k != bisect_right_reverse(S, S[k - 1]):
        ans.append(S[k - 1] + 1)
    else:
        if S[k - 1] == S[-1]:
            ans.append(0)
        else:
            ans.append(S[k - 1])

print(*ans, sep='\n')
