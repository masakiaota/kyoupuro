# 累積和の類

# 一次元累積和クラス


class cumsum1d:
    def __init__(self, ls: list):
        '''
        1次元リストを受け取る
        '''
        from itertools import accumulate
        self.ls_accum = [0] + list(accumulate(ls))

    def total(self, i, j):
        # もとの配列lsにおける[i,j)の中合計
        return self.ls_accum[j] - self.ls_accum[i]

# 二次元累積和クラス


class cumsum2d:
    def __init__(self, ls: list):
        '''
        2次元のリストを受け取る
        '''
        import numpy as np
        self.ls = np.array(ls)
        H, W = self.ls.shape
        self.ls_accum = np.zeros((H + 1, W + 1))
        self.ls_accum[1:, 1:] = self.ls.cumsum(axis=0).cumsum(axis=1)

    def total(self, i1, j1, i2, j2):
        '''
        点(i1,j1),(i1,j2),(i2,j1),(i2,j2)の4点が成す四角の中の合計を取り出す
        ただし i1<=i2, j1<=j2
        ただし、i軸に関しては[i1,i2),j軸に関しては[j1,j2)の半開区間である
        '''
        return self.ls_accum[i2, j2] - self.ls_accum[i1, j2] \
            - self.ls_accum[i2, j1] + self.ls_accum[i1, j1]
