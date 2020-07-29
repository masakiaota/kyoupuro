# https://atcoder.jp/contests/arc025/tasks/arc025_2
# 二次元累積和するだけじゃない？
import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def ints(): return list(map(int, read().split()))


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


class cumsum2d:  # 二次元累積和クラス pypyでも使えるよ
    def __init__(self, ls: list):
        '''
        2次元のリストを受け取る
        '''
        from itertools import product
        H = len(ls)
        W = len(ls[0])
        self.ls_accum = [[0] * (W + 1)]
        for l in ls:
            self.ls_accum.append([0] + l)

        # 縦に累積
        for i, j in product(range(1, H + 1), range(1, W + 1)):
            self.ls_accum[i][j] += self.ls_accum[i - 1][j]
        # 横に累積
        for i, j in product(range(1, H + 1), range(1, W + 1)):
            self.ls_accum[i][j] += self.ls_accum[i][j - 1]

    def total(self, i1, j1, i2, j2):
        '''
        点(i1,j1),(i1,j2),(i2,j1),(i2,j2)の4点が成す四角の中の合計を取り出す
        ただし i1<=i2, j1<=j2
        ただし、i軸に関しては[i1,i2),j軸に関しては[j1,j2)の半開区間である
        '''
        return self.ls_accum[i2][j2] - self.ls_accum[i1][j2] \
            - self.ls_accum[i2][j1] + self.ls_accum[i1][j1]


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from itertools import product, permutations, combinations

H, W = ints()
C = read_matrix(H)
# 白色は-にしよっか
for i in range(H):
    for j in range(W):
        if (i + j) & 1:
            C[i][j] *= -1
# print(*C, sep='\n')
C_cum = cumsum2d(C)
ans = 0
for si, sj in product(range(H), range(W)):  # 左上の点
    for ti, tj in product(range(si + 1, H + 1), range(sj + 1, W + 1)):  # 右下の点(含まない)
        s = C_cum.total(si, sj, ti, tj)
        if s == 0:
            ans = max(ans, (ti - si) * (tj - sj))
print(ans)
