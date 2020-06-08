# この問題の解説もこれで終わり https://www.slideshare.net/hcpc_hokudai/advanced-dp-2016

import numpy as np

MOD = 10**4 + 7


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
n = 4
k = 2

G = np.matrix([[0, 1, 1, 0],
               [0, 0, 1, 0],
               [0, 0, 0, 1],
               [1, 0, 0, 0]])

ans = matrix_pow(G, 2, mod=MOD).sum()
print(ans)
