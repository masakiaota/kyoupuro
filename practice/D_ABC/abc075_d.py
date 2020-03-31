# https://atcoder.jp/contests/abc075/tasks/abc075_d
# 座標圧縮して二次元累積和に打ち込めば良さそう

# はじめての座圧...?

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])
    return ret


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


class ZaAtu:
    def __init__(self, ls):
        # 座標圧縮クラス(仮) #どうしたら使いやすくなるのか知らんけど
        self.i_to_orig = sorted(set(ls))
        self.orig_to_i = {}
        for i, zahyou in enumerate(self.i_to_orig):
            self.orig_to_i[zahyou] = i
        self.len = len(self.i_to_orig)

    def __len__(self):
        return len(self.i_to_orig)


from itertools import product

N, K = read_ints()
X_ori, Y_ori = read_col(N, 2)
X = ZaAtu(X_ori)
Y = ZaAtu(Y_ori)
table = [[0] * Y.len for _ in range(X.len)]  # 二次元累積和を作る
for x, y in zip(X_ori, Y_ori):
    i = X.orig_to_i[x]
    j = Y.orig_to_i[y]
    table[i][j] += 1

table = cumsum2d(table)
# 怒りの全探索
ans = (2 * 10**9)**2 + 114514
for i, j in product(range(X.len), range(Y.len)):  # 始点
    for ii, jj in product(range(i, X.len + 1), range(j, Y.len + 1)):  # 終点
        if table.total(i, j, ii, jj) >= K:
            S = (X.i_to_orig[ii - 1] - X.i_to_orig[i]) * \
                (Y.i_to_orig[jj - 1] - Y.i_to_orig[j])  # 辺上に乗っているという連続値な考え方なのでこの計算式でよい
            ans = min(ans, S)
print(ans)
