# 繰り返し二乗法の応用

MOD = 10 ** 4
import numpy as np


# 入力
n = int(input())


def matrix_pow(mat: np.matrix, n: int, mod: int):
    # nの2進数表記でビットが立っているところだけ処理すればいい
    mattmp = mat.copy()
    ret = np.matrix(np.eye(2, dtype='int32'))  # 単位元は対角行列
    while n > 0:
        if n & 1:  # ビットが立っているなら処理する
            ret *= mattmp
            ret %= mod
        mattmp = mattmp**2
        n >>= 1  # ビットを処理
    return ret


def pow(x, n, mod):  # 一般化繰り返し二乗法(メモ)
    ret = 1  # 単位元
    tmp = copy(x)  # 二乗していくもの #書き換えないためにコピーしておく
    while n > 0:
        if n & 1:  # ビットが立っているなら処理する
            ret *= tmp
            ret %= mod
        tmp = tmp**2  # 二乗していく
        n >>= 1  # ビットを処理
    return ret


A = np.matrix([[1, 1],
               [1, 0]])

A_n = matrix_pow(A, n, MOD)
F1F0 = np.array([1, 0]).reshape(2, 1)
Fn = A_n * F1F0 % MOD

print(Fn[1, 0])
