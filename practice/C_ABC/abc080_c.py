# https://atcoder.jp/contests/abc080/tasks/abc080_c
# 普通にbit全探索では？

N = int(input())
F = []
for _ in range(N):
    F.append(list(map(int, input().split())))
P = []
for _ in range(N):
    P.append(list(map(int, input().split())))

bitlen = 10


# def ret_score(n: int):
#     # 営業時間かぶり計算
#     C = []
#     for i in range(N):  # Nでiter
#         cnt = 0
#         for bit in range(bitlen):  # 桁とFを対応
#             if ((n >> bit) & 1) and F[i][bit]:  # もし1その桁で1なら
#                 cnt += 1
#         C.append(cnt)

#     # スコア計算
#     ret = 0
#     for i, c in enumerate(C):
#         ret += P[i][c]
#     return ret


# ans = -10**10
# for n in range(1, 1 << bitlen):
#     ans = max(ans, ret_score(n))
# print(ans)


# numpyでも実装してみる
import numpy as np
F = np.array(F)
P = np.array(P)

bitshifter = np.array([x for x in range(bitlen)])


def ret_score(n: int, F, bitshifter, P):
    bit_arr = (n >> bitshifter) & 1
    F_matched = bit_arr & F
    C = F_matched.sum(axis=1)

    # スコア計算
    ret = 0
    for i, c in enumerate(C):
        ret += P[i, c]
    return ret


ans = -10**10
for n in range(1, 1 << bitlen):
    ans = max(ans, ret_score(n, F, bitshifter, P))
print(ans)
