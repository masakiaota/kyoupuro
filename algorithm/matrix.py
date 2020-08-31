import numpy as np


def matmul_mod(X: np.ndarray, Y: np.ndarray, MOD: int=10**9 + 7)->np.ndarray:
    # return X@Y %MOD
    ret = np.empty((X.shape[0], Y.shape[1]), dtype=np.int64)
    YT = Y.T
    for i in range(X.shape[0]):  # Xの行
        tmp = (YT * X[i, :]) % MOD
        ret[i, :] = tmp.sum(1) % MOD
    return ret


def matmul_mod(X: np.ndarray, Y: np.ndarray, MOD: int=10**9 + 7)->np.ndarray:
    # return X@Y %MOD
    X3d = X[:, :, np.newaxis].transpose(0, 2, 1)
    Y3d = Y[:, :, np.newaxis].transpose(2, 1, 0)
    return ((X3d * Y3d) % MOD).sum(2) % MOD
