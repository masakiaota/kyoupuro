import numpy as np


# def to_2(num):
#     ret = []
#     while (num != 0):
#         if num % 2 == 1:
#             ret.append(True)
#         else:
#             ret.append(False)
#         num = num // 2
#     return np.array(ret)

# def()

N, K = list(map(int, input().split()))
A = list(map(int, input().split()))

# 全探索
from itertools import combinations
A_2 = [bin(a) for a in A]

kouho = [sum(A[idx:(idx2+1)]) for idx, idx2 in combinations(range(0, N), 2)]


ma = 0
for A_sub in combinations(kouho, K):
    bina = A_sub[0]
    # print(bina)
    for i in range(1, K):
        bina = bina & A_sub[i]
    if bina > ma:
        ma = bina

print(ma)


# import pandas as pd
# kouho = pd.Series(kouho)
# kouho[kouho.str.len().max()-2 < kouho.str.len()]
