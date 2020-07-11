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

# クエリを高速に処理する必要あり
# シンプルに1クエリだけ考える
# 先頭からK番目以内だといくら得するのかで考えてやればいいのでは？
# でも各ラクダについてKが決まってるのが厄介
# 得する方から挿入できる一番後ろから挿入するのがいいんじゃない？
T = a_int()
ans = []


def solve():
    N = a_int()
    GIK = []  # (gain,i ,k)
    LRK = []
    for i in range(N):
        k, l, r = ints()
        LRK.append((l, r, k - 1))
        GIK.append((l - r, i, k - 1))
    GIK.sort(reverse=True)
    perm = [-1] * N  # 並び順
    ketu = []
    for g, i, k in GIK:
        while 0 <= k and perm[k] != -1:
            k -= 1
        if k == -1:
            ketu.append(k)  # あとでけつにtasu
        perm[k] = i
    ans = 0
    i = 0  # 前から何番目か
    for idx_rakuda in perm + ketu:
        if idx_rakuda != -1:
            l, r, k = LRK[idx_rakuda]
            ans += l if idx_rakuda <= k else r
            i += 1
    return ans


for _ in range(T):
    ans.append(solve())  # なんか違うんすけど
print(*ans, sep='\n')
