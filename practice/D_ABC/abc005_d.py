# https://atcoder.jp/contests/abc005/tasks/abc005_4
# 二次元累積和でたこ焼き器の矩形の美味しさを取得できる O(2500)
# 各店員美味しさ合計全探索→O(2500C2) O(3125000) 前処理時点で各pに対応する最大値を保持しておけば良さそう


import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


# class cumsum2d:  # 二次元累積和クラス
#     def __init__(self, ls: list):
#         '''
#         2次元のリストを受け取る
#         '''
#         import numpy as np
#         self.ls = np.array(ls)
#         H, W = self.ls.shape
#         self.ls_accum = np.zeros((H + 1, W + 1))
#         self.ls_accum[1:, 1:] = self.ls.cumsum(axis=0).cumsum(axis=1)

#     def total(self, i1, j1, i2, j2):
#         '''
#         点(i1,j1),(i1,j2),(i2,j1),(i2,j2)の4点が成す四角の中の合計を取り出す
#         ただし i1<=i2, j1<=j2
#         ただし、i軸に関しては[i1,i2),j軸に関しては[j1,j2)の半開区間である
#         '''
#         return self.ls_accum[i2, j2] - self.ls_accum[i1, j2] \
#             - self.ls_accum[i2, j1] + self.ls_accum[i1, j1]


class cumsum2d:  # 二次元累積和クラス
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


# default import
from itertools import product, permutations, combinations


N = read_a_int()
D = read_matrix(N)
D_cum = cumsum2d(D)

# preprocessing
ans = [-1] * (N ** 2 + 1)
points = []
for i, j in product(range(N + 1), range(N + 1)):
    points.append((i, j))

for s, t in combinations(points, r=2):
    si, sj = s
    ti, tj = t
    if si < ti and sj < tj:
        pass
    elif si > ti and sj > tj:
        si, ti = ti, si
        sj, tj = tj, sj
    else:
        continue
    S = (ti - si) * (tj - sj)
    ans[S] = max(ans[S], D_cum.total(si, sj, ti, tj))

ma = -1
ans2 = []
for a in ans:
    if a > ma:
        ma = a
    ans2.append(ma)

Q = read_a_int()
for _ in range(Q):
    p = read_a_int()
    print(int(ans2[p]))
