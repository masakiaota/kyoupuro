def perm_mod(n, r, mod=10**9 + 7):
    '''nPrをmodを取って返す'''
    if n < r:  # そんな通りはありえない
        return 0
    ret = 1
    for _ in range(r):
        ret *= n
        ret %= mod
        n -= 1
    return ret


def combination_mod(n, r, mod=10**9 + 7):
    # mod取りながらcombination
    if r > n:
        return 0  # このような通りの数は無いため便宜上こう定義する
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    return nf * pow(rf, mod - 2, mod) % mod


def ret_list_comb_r(n, r, mod=10 ** 9 + 7):
    '''nC[0:r+1]を返す。for とかで再計算せずに済むように'''
    ret = [1]
    if r > n:
        raise ValueError('rがnより大きいけど大丈夫か？(0通り？)')
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
        ret.append(nf * pow(rf, mod - 2, mod) % mod)
    return ret


def ret_list_comb_n(n, m, r, mod=10 ** 9 + 7):
    '''[n:m]Crを返す。mは半開区間であることに注意'''
    # まずはnCrを計算
    if r > n:
        raise ValueError('rがnより大きいけど大丈夫か？(0通り？)')
    r = min(r, n - r)
    nf = rf = 1
    for i in range(r):
        nf = nf * (n - i) % mod
        rf = rf * (i + 1) % mod
    ret = {}
    ret[n] = nf * pow(rf, mod - 2, mod) % mod
    for i in range(1, m - n):
        ret[n + i] = ret[n + i - 1] * (n + i) % mod
        ret[n + i] *= pow(n - r + i, mod - 2, mod)  # 逆元を掛けて割る
        ret[n + i] %= mod  # 逆元を掛けて割る
    return ret


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
