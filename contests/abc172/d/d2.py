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

# 解説によるO(N)解法


N = a_int()
ans = 0

'''
以下のような問題を高速化したい
for k in range(1, N + 1):
    for j in range(1, N + 1):
        ans += k * (k % j == 0)  # jがkの約数になっていれば加算する

式の形からk,jはひっくり返せる
jをgivenとしたとき, k * (k%j==0)となる数列の和を高速に列挙する。これをg(j)と定義する
→この数列はkがjの倍数のときだけ値をとる→nonzero数列だけに注目すると以下のように言い換えられる
「jの倍数であってN以下であるものの数列」の和を高速に知りたい

1j+2j+3j...yjを求めたいただし,yj<=N (⇔ y<=N//j)
g(j)=j*((1+y)*y)//2と式変形可能

'''
for j in range(1, N + 1):
    y = N // j
    ans += j * (1 + y) * y // 2
print(ans)
