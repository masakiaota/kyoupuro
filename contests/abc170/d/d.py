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

# 一番小さい数はかならず条件を満たす(同じ数があるときはだめ)
# i!=j の任意のjで割り切れない → N-i!=jで一つでも割り切れるものがある数

'''
エラトステネスの篩の数列版じゃない？
1  2  3  4  5  6  7  8  9  10
11 12 13 14 15 16 17 18 19 20
21 22 23 24 25 26 27 28 29 30

小さい方から倍数を取ってけば良いかな
'''


N = read_a_int()
A = read_ints()


cnt = Counter(A)
A = []
B = []
for k, v in cnt.items():
    if v == 1:
        A.append(k)
        # ただ消すだけじゃだめなんだ
    else:
        B.append(k)


A.sort()
AMAX = 10**6 + 10
is_first = [True] * (AMAX + 1)
for a in B:
    if is_first[a]:
        for j in range(a, AMAX + 1, a):
            is_first[j] = False

cnt = 0  # なにでも割り切れないときのcnt
for a in A:
    if is_first[a]:
        cnt += 1
        for j in range(a, AMAX + 1, a):
            # print(j)
            is_first[j] = False
print(cnt)
