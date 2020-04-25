# https://atcoder.jp/contests/abc058/tasks/arc071_b
# 定式化してみると、独立に計算できる部分と、
# 前処理で累積和を求めておけばループ内でO(1)に減る部分と、
# 愚直に計算する部分に減る

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


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


def read_ints():
    return list(map(int, read().split()))


from itertools import accumulate

n, m = read_ints()
X = read_ints()
Y = read_ints()
MOD = 10 ** 9 + 7


def mul_term(X: list, n):
    S = list(accumulate(reversed(X)))
    S.reverse()
    ret = ModInt(0)
    for i in range(n - 1):
        ret += S[i + 1] - (n - i - 1) * X[i]
    return ret


# print(mul_term(X, n), mul_term(Y, m))
print(mul_term(X, n) * mul_term(Y, m))
