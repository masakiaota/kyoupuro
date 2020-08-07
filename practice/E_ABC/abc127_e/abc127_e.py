# https://atcoder.jp/contests/abc127/tasks/abc127_e
# 自分の計算画像を参考に
# ポイントは二点
# 1. 通りを決定してからpairwiseの和→pairを先に決定してから条件を満たす通りを加算 というΣの入れ替え
# 2. 二点の全通りの差を加算は無理→二点の差は何回現れるか を加算


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


def combination(n, r):
    # 上記のModIntを用いた通りの数の実装
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = ModInt(1)
    for i in range(r):
        nf = nf * (n - i)
        rf = rf * (i + 1)
    return nf / rf


N, M, K = map(int, input().split())
C = combination(N * M - 2, K - 2)
x_term = ModInt(0)
for dx in range(1, N):
    x_term += (N - dx) * dx
x_term *= M**2

y_term = ModInt(0)
for dy in range(1, M):
    y_term += (M - dy) * dy
y_term *= N**2

print(C * (x_term + y_term))
