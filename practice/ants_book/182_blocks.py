# dpの定式化を珍しく本でちゃんと書いてくれててわかりやすい


import numpy as np

MOD = 10**4 + 7


def matrix_pow(mat: np.matrix, n: int, mod: int):
    # nの2進数表記でビットが立っているところだけ処理すればいい
    mattmp = mat.copy()
    ret = np.matrix(np.eye(3, dtype='int64'))  # 単位元は対角行列
    while n > 0:
        if n & 1:  # ビットが立っているなら処理する
            ret *= mattmp
            ret %= mod
        mattmp = mattmp**2
        n >>= 1  # ビットを処理
    return ret


# 入力
N = int(input())

A = np.matrix([[2, 1, 0],
               [2, 2, 2],
               [0, 1, 2]])
A_n = matrix_pow(A, N, mod=MOD)
print(A_n[0, 0])
