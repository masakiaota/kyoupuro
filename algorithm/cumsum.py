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


def solve_max_a_plas_b(A: list, B: list):  # 累積maxの応用
    '''max_{i<j}(a_i + b_j)をO(n)で解く (古いpypyだとfuncが未実装かも)'''
    from itertools import accumulate
    assert len(A) == len(B)
    A_accum = list(accumulate(A, func=max))
    B_accum = list(accumulate(reversed(B), func=max))[::-1]
    ret = 0
    for i in range(len(A_accum) - 1):
        ret = max(A_accum[i] + B_accum[i + 1], ret)
    return ret

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


class Compress:
    def __init__(self, ls):
        # 座標圧縮クラス(仮) #どうしたら使いやすくなるのか知らんけど
        self.i_to_orig = sorted(set(ls))
        self.orig_to_i = {}
        for i, zahyou in enumerate(self.i_to_orig):
            self.orig_to_i[zahyou] = i
        self.len = len(self.i_to_orig)

    def __len__(self):
        return len(self.i_to_orig)


# 尺取法 累積積などは数が大きくなりすぎて累積できないことが多いので

def two_pointers(ls: list):
    '''すべてのlに対して、条件is_okをみたすls[l:r]の中で
    r - lが最大になるような(l,r)の集合を返す'''
    n_ls = len(ls)
    ret = []

    def append(r, pre_states):
        '''状態にls[r]を考慮して更新する'''
        # 問題によって自分で定義
        # 掛け算の例 return pre_state*ls[r]
        pass

    def pop(l, pre_states):
        '''状態からls[l]を抜く更新をする'''
        # 問題によって自分で定義
        # 掛け算の例 return pre_state//ls[l]
        pass

    def is_ok(r, pre_states):
        # 問題によって自分で定義
        states = append(r, pre_states)
        # 114以下の最長の範囲が知りたい例 return states<=114
        pass

    r = 0
    states = (自分で定義 複数変数でも構わない)
    for l in range(n_ls):
        while r < n_ls and is_ok(r, states):
            # 更新式
            states = append(r, states)
            r += 1
        ret.append((l, r))
        # 抜けるときの更新
        states = pop(l, states)
    return ret
