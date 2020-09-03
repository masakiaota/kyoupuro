import sys
sys.setrecursionlimit(1 << 25)
readline = sys.stdin.buffer.readline
read = sys.stdin.readline  # 文字列読み込む時はこっち
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(readline())


def ints(): return list(map(int, readline().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, readline().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, readline().split())))
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


N = a_int()
X = read()[:-1]
tmp = X.count('1')  # Xの1の数
X = list(map(int, list(X[::-1])))

# オリジナルをpop(X)+-1で割ったものを作っておく
Xmodpop = [0, 0, 0]
for i, x in enumerate(X):
    if x:
        Xmodpop[1] += pow(2, i, tmp + 1)
        if tmp != 1:
            Xmodpop[-1] += pow(2, i, tmp - 1)
Xmodpop[1] %= tmp + 1
if tmp != 1:
    Xmodpop[-1] %= tmp - 1

X_nex = []  # 一度操作したあとの値
for i in range(N):
    if X[i] == 1:
        if tmp == 1:
            X_nex.append(-1)
            continue
        X_nex.append((Xmodpop[-1] - pow(2, i, tmp - 1)) % (tmp - 1))
    else:
        X_nex.append((Xmodpop[1] + pow(2, i, tmp + 1)) % (tmp + 1))

# 愚直に操作
ans = []
for i in range(N):
    n = X_nex[i]
    if n == -1:
        ans.append(0)
        continue
    cnt = 1
    while n:
        popc = bin(n).count('1')
        n %= popc
        cnt += 1
    ans.append(cnt)

print(*ans[::-1], sep='\n')
