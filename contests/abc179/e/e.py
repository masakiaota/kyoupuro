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


# X%M + X^(2)%M + X^(4)%M + X^(8)%M ...
# 重要な性質X^(k)%M for k=[0,M-1]は循環する
# X^(k) = X^(c*M+k) (mod M)
# = X^(k%M)

# X(0%M)%M + X^(2%M)%M + X^(4%M)%M + X^(8%M)%M ...

# あまりの循環テーブルT
# T[0%M] + T[2%M] + T[4%M] ...
# T[2^(0)%M] + T[2^(1)%M] + T[2^(2)%M] ...(M)  ... (循環するので×だけ) (N%M) ...
# N//M回循環する
# 2のべき乗をMで割ったテーブル
# 正確には違ったね

# (X^2*k) = (X^2)^k
# (X^2)^kが循環する

N, X, M = ints()
A = [X]
for i in range(M):
    A.append(pow(A[-1], 2, M))  # 循環は単純ではないのか

# 早期終了
if N < M:
    exit(sum(A[:N]))

if M == 1:
    exit(0)

# 非ループ部分とそれ以外の検出
visited = set()
for i in range(M):
    if A[i] in visited:
        len_total = i
        break
    visited.add(A[i])
# i番目の要素が初めて出てくるところを改めて探す
s = A.index(A[len_total])
cut = A[:s]
loop = A[s:len_total]
# print(cut)
# print(loop)
# print(A) #ok
ans = sum(cut)
N -= len(cut)

ans += sum(loop) * (N // len(loop))
ans += sum(loop[:N % len(loop)])
print(ans)
