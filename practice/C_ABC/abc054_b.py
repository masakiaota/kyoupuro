# https://atcoder.jp/contests/abc054/tasks/abc054_b
# これも緑diffのB問題
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


from itertools import product
N, M = read_ints()
A = []
for a in read_map(N):
    for aa in a:
        A.append(1 if aa == '#' else 0)
B = []
for b in read_map(M):
    for bb in b:
        B.append(1 if bb == '#' else 0)


# 面倒なのでnumpy芸していっすか？
import numpy as np
A = np.array(A).reshape((N, N))
B = np.array(B).reshape((M, M))

for i, j in product(range(N - M + 1), range(N - M + 1)):
    ii, jj = i + M, j + M
    A_sub = A[i:ii, j:jj]
    if np.logical_xor(A_sub, B).sum() == 0:
        print('Yes')
        exit()

print('No')
