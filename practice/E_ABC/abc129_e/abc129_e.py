# https://atcoder.jp/contests/abc129/tasks/abc129_e
# 桁DPの理解が深まる一問
# 詳しくはeditorial参照

import sys
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


MOD = 10 ** 9 + 7

L = read()[:-1]
n = len(L)

# a+bの各桁についてDPで組み合わせの総和を数え上げる
# dp[i][j]...上位[0,i)桁まで見たときのa,bの組み合わせの総数。j=0→L[:i]まで一致している場合の通り数、j=1→未満が確定している状態の通り数

dp = [[0] * 2 for _ in ra(n + 1)]
dp[0][0] = ModInt(1)
for i in ra(n):
    # i桁目が0の場合→a+bのi桁目はどちらも必ず0となるはず(通りの数は増えない)
    if L[i] == '0':
        # 未満の状態は未満の状態に
        # ちょうどの状態もちょうどの状態に遷移する
        dp[i + 1][0] += dp[i][0]
        dp[i + 1][1] += dp[i][1]
    else:
        # ちょうどの数も未満に遷移する
        dp[i + 1][1] += dp[i][0] + dp[i][1]

    # i桁目が1の場合→a+bのi桁目はどちらかが1となるはず(通りの数2倍になる)
    if L[i] == '0':
        # 未満は未満のままだけど
        # ちょうどだった状態は超えてしまう虚無に帰す
        dp[i + 1][1] += dp[i][1] * 2
    else:
        # 未満は未満のままで、ちょうどはちょうどのまま
        dp[i + 1][0] += dp[i][0] * 2
        dp[i + 1][1] += dp[i][1] * 2

print(dp[-1][0] + dp[-1][1])
