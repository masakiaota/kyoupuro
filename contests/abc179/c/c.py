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


class FastFactorization:
    def __init__(self, N: int):
        '''構築O(NloglogN)、クエリO(logN)'''
        self.N = N
        self.min_prime = self._make_minimum_prime()

    def _make_minimum_prime(self):
        # xの最小の素因数表を作成
        min_prime = [x for x in range(self.N + 1)]
        # min_prime[0] = 0  # 0と1は素数ではない
        # min_prime[1] = 1
        for i in range(2, int(self.N ** 0.5) + 1):
            if min_prime[i] == i:  # 素数だったら更新
                for j in range(2 * i, self.N + 1, i):  # iの倍数は素数でない
                    if min_prime[j] == j:
                        min_prime[j] = i
        return min_prime

    def query(self, x: int):
        # 最小素数配列min_primeを使ってO(log N)で因数分解
        # -> Counter[p,n] (素数,冪数) を格納
        # xはself.N以下
        if x == 1:
            return Counter()  # 1は素数ではない

        # 素因数分解
        arr = []
        tmp = x
        while tmp != 1:
            p = self.min_prime[tmp]
            tmp //= p
            arr.append(p)
        return Counter(arr)


# a*b = N-c
# 1 <= N-c <= N-1
# N-c=X
# a*b = X #Xの約数の個数は？
# N-cを素因数分解 -> 冪数 p1^m1 p2^m2 p3^m3  の+1の積

# 素因数分解 -> O(√X)
# O(N√N) ~ 10^9 ← TLE

# 高速素因数分解 -> 前処理 O(N) クエリ O(log(X))
# →O(N + NlogN) ~ 10^7 ←ちょうどいい感じ
# table[i] ... 整数iを構成する最小の素因数

N = a_int()
fact = FastFactorization(N + 1)
ans = 0
for x in range(1, N):
    tmp = 1
    for v in fact.query(x).values():
        tmp *= v + 1
    ans += tmp

print(ans)

# たてのアイデアが天才的だった
