# https://atcoder.jp/contests/abc106/tasks/abc106_d
# 何も見ずにやり直してみる

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


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


N, M, Q = read_ints()

# 列車を二次元座標に代入
table = [[0] * N for _ in range(N)]  # 都市はそれぞれ-1扱い
for _ in range(M):
    L, R = read_ints()
    table[L - 1][R - 1] += 1

# 二次元累積和を取る
table = cumsum2d(table)

# クエリに答える
for _ in range(Q):
    p, q = read_ints()
    p -= 1
    q -= 1
    print(table.total(p, p, q + 1, q + 1))
