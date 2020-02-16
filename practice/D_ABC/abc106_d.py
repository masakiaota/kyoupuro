# https://atcoder.jp/contests/abc106/tasks/abc106_d
# どうやったら高速に区間[p,q]に完全に入っている区間の数を取得できるか？
# p,qを二次元座標として捉えると電車1本の線は二次元平面上の1点として表せる
# このときp,qの区間に完全に入っている電車の本数というのは、x軸、y軸の両方において[p,q]となる四角形内にある電車の本数
# queryに高速に答えるために二次元累積和を構築する
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


# import numpy as np

# N, M, Q = read_ints()
# LR = read_tuple(M)
# pq = read_tuple(Q)

# # numpy使わずに再実装 #入れるときはpythonのほうが速いので
# LR2d = [[0] * (N + 1) for _ in range(N + 1)]
# for L, R in LR:
#     LR2d[L][R] += 1
# LR2d = np.array(LR2d, np.int64)

# LR2d_accum = np.zeros((N + 2, N + 2), np.int64)
# LR2d_accum[1:, 1:] = LR2d.cumsum(axis=0).cumsum(axis=1)  # 1次元累積和同様半開区間で考える
# # つまりインデックスi,jとはもとの配列においてi,jを含まない四角形の領域の合計を出す

# # queryに答えていく
# ans = []
# for p, q in pq:
#     ans.append(LR2d_accum[q + 1, q + 1] - LR2d_accum[q + 1, p] -
#                LR2d_accum[p, q + 1] + LR2d_accum[p, p])

# print(*ans, sep='\n')

# クラスで実装してみる
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


N, M, Q = read_ints()
LR = read_tuple(M)
pq = read_tuple(Q)

# numpy使わずに再実装 #入れるときはpythonのほうが速いので
LR2d = [[0] * (N + 1) for _ in range(N + 1)]
for L, R in LR:
    LR2d[L][R] += 1

LR2d_accum = cumsum2d(LR2d)

# queryに答えていく
ans = []
for p, q in pq:
    ans.append(int(LR2d_accum.total(p, p, q + 1, q + 1)))

print(*ans, sep='\n')
