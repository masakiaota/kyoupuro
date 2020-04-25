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


# https://atcoder.jp/contests/abc159/tasks/abc159_d
# Kを抜かないときは簡単
# その数字のカウントをnとするとnC2通り
# k番目の数字を抜く kが減る→ n-1 C 2通りになる

# カウントn1,n2...とすると sum (n1,n2...) nC2 で
# kのカウントにnkについて通りを引いて新しい通りに更新してやれば良い

N = read_a_int()
A = read_ints()
cnt = Counter(A)
s = 0
for v in cnt.values():
    s += v * (v - 1) // 2

for a in A:
    n = cnt[a]
    ans = s - (n * (n - 1) // 2) + ((n - 1) * (n - 2) // 2)
    print(ans)
