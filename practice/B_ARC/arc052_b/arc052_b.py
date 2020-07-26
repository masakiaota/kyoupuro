# https://atcoder.jp/contests/arc052/tasks/arc052_b
import sys
read = sys.stdin.readline


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


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


from math import pi

N, Q = ints()
X, R, H = read_col(N)
A, B = read_col(Q)

M = [0] * (2 * 10**4 + 10)  # [i,i+1)に含まれる体積
for x, r, h in zip(X, R, H):
    prev = 1 / 3 * pi * r * r * h
    for i in range(h):
        hh = h - i
        v = prev * ((hh - 1) / hh)**3  # 体積比の公式言えますか？
        M[x + i] += prev - v
        prev = v

# あとは累積和しておしまい
M_cum = cumsum1d(M)

ans = [M_cum.total(a, b) for a, b in zip(A, B)]
print(*ans, sep='\n')
