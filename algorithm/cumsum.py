# 累積和の類


class cumsum1d:  # 一次元累積和クラス
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]


class CumXor1d:  # 一次元累積xor(供養)
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        self.ls_accum = [0]
        for l in ls:
            self.ls_accum.append(l ^ self.ls_accum[-1])

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] ^ self.ls_accum[i]


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


# 1次元imos法


class Imos1d:
    def __init__(self, N: int):
        '''
        [0]*N の配列に対して、区間の加算を管理する。
        '''
        self.ls = [0] * (N + 1)  # 配列外参照を防ぐため多くとっておく
        self.N = N

    def add(self, l, r, x):
        '''
        [l,r)の区間にxを足し込む O(1)
        '''
        self.ls[l] += x
        self.ls[min(r, self.N)] -= x  # 配列外参照は余分に作ったところにまとめておく(どうせ使わない)

    def get_result(self):
        '''
        O(N) かけて、区間の加算結果を取得する
        '''
        from itertools import accumulate
        return list(accumulate(self.ls[:-1]))


# 尺取法 累積積などは数が大きくなりすぎて累積できないことが多いので
# 例として積が114を超えない最長の区間を取得する
arr = list(range(100))
r = -1  # [l,r)を前提とする
ans = 0
for l in range(len(arr)):
    # r = max(l, r)
    if r <= l:  # explicitに初期cumを決める
        r = l
        cum = arr[r]
    while r < len(arr) and cum <= 114:  # 初めて条件を満たさなくなるところ、というのが半開区間を使う理由
        r += 1
        if r == len(arr):  # 終端処理用
            break
        cum *= arr[r]
    # print(cum, l, r)
    ans = max(ans, r - l)
    cum //= arr[l]  # ちゃんと抜く
