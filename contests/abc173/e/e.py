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


class ModInt:
    def __init__(self, x, MOD=10 ** 9 + 7):
        '''
        pypyじゃないと地獄みたいな遅さなので注意
        '''
        self.mod = MOD
        self.x = x % MOD

    def __str__(self):
        return str(self.x)

    __repr__ = __str__

    def __add__(self, other):
        if isinstance(other, ModInt):
            return ModInt(self.x + other.x, self.mod)
        else:
            return ModInt(self.x + other, self.mod)

    def __sub__(self, other):
        if isinstance(other, ModInt):
            return ModInt(self.x - other.x, self.mod)
        else:
            return ModInt(self.x - other, self.mod)

    def __mul__(self, other):
        if isinstance(other, ModInt):
            return ModInt(self.x * other.x, self.mod)
        else:
            return ModInt(self.x * other, self.mod)

    def __truediv__(self, other):
        if isinstance(other, ModInt):
            return ModInt(self.x * pow(other.x, self.mod - 2, self.mod), self.mod)
        else:
            return ModInt(self.x * pow(other, self.mod - 2, self.mod), self.mod)

    def __pow__(self, other):
        if isinstance(other, ModInt):
            return ModInt(pow(self.x, other.x, self.mod), self.mod)
        else:
            return ModInt(pow(self.x, other, self.mod), self.mod)

    __radd__ = __add__

    def __rsub__(self, other):  # 演算の順序が逆
        if isinstance(other, ModInt):
            return ModInt(other.x - self.x, self.mod)
        else:
            return ModInt(other - self.x, self.mod)

    __rmul__ = __mul__

    def __rtruediv__(self, other):
        if isinstance(other, ModInt):
            return ModInt(other.x * pow(self.x, self.mod - 2, self.mod), self.mod)
        else:
            return ModInt(other * pow(self.x, self.mod - 2, self.mod), self.mod)

    def __rpow__(self, other):
        if isinstance(other, ModInt):
            return ModInt(pow(other.x, self.x, self.mod), self.mod)
        else:
            return ModInt(pow(other, self.x, self.mod), self.mod)


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b


'''
absでソートして大きい方から要素をK個選ぶ。
このとき、積の結果が+であれば、そのまま採用

-であれば、一番小さな-を不採用にして、採用してない中で一番大きな＋を探す？
もしくは
負のときの処理が難しい

正負で場合分けすれば良い
負の数の集合からは偶数個使う場合は、大きい方順に取り出せば良い
奇数個使う場合は、小さい順に取り出せば良い

正の数の集合からは大きい順に取り出せば良い

'''
N, K = ints()
A = ints()
A.sort()
sep = bisect_left(A, 0)
minas = A[:sep]
pulas = list(reversed(A[sep:]))
# print(minas, pulas)

# minasを+にできる場合 (要素のすべてが-でなければ必ず+にできる場合が存在する...?コーナーケースがいくつかあるな)
mul_mina = {}
mul_mina[0] = 1
for m in range(2, len(minas) + 1, 2):
    mul_mina[m] = mul_mina[m - 2] * minas[m - 1] % MOD * minas[m - 2] % MOD

mul_pura = {}
mul_pura[0] = 1
for n in range(1, len(pulas) + 1):
    mul_pura[n] = mul_pura[n - 1] * pulas[n - 1] % MOD

print(mul_mina, mul_pura)
# これを全探索しようとしても大小情報が抜けてるから比較できないじゃん!ええーーー詰です


# mul_mina = ModInt(1)
# mul_pura = ModInt(1)
# for m in range(0, len(minas), 2):
#     n = K - m
#     print(n)
#     if n > len(pulas):  # 個数なければbreak
#         continue
