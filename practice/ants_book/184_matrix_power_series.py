# 累乗の累積和を行列形式で書くとオーダー落とせるというアイデア
# 頭がいい

import numpy as np


def matrix_pow(mat: np.matrix, n: int, mod: int):
    # nの2進数表記でビットが立っているところだけ処理すればいい
    mattmp = mat.copy()
    ret = np.matrix(np.eye(mat.shape[0], dtype='int64'))  # 単位元は対角行列
    while n > 0:
        if n & 1:  # ビットが立っているなら処理する
            ret *= mattmp
            ret %= mod
        mattmp = mattmp**2
        n >>= 1  # ビットを処理
    return ret



# 入力
n = 2
k = 2
M = 4
A = np.matrix([[0, 1],
               [1, 1]])

I = np.eye(A.shape[0])
AOII = np.hstack([A, np.zeros_like(A)])
AOII = np.vstack([AOII,
                  np.hstack([I, I])])

AOII_k = matrix_pow(AOII, k + 1, mod=M)

print((AOII_k[2:, :2] - I) % M)
