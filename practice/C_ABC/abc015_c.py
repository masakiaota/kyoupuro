# https://atcoder.jp/contests/abc015/tasks/abc015_3
# 制約はたかだか5**5なので全探索することができる
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


# N, K = read_ints()
# T = read_matrix(N)


# def dfs(i, j, x):  # i行目j列目を使うときについて、xは今までの経路のxor
#     # 終了条件
#     # print(i, j)
#     if i == N - 1:
#         if x == 0:  # バグがあればTrueを返す
#             print('Found')
#             exit()
#         return

#     # 探索
#     for jj in range(K):
#         dfs(i + 1, jj, x ^ T[i + 1][jj])


# for j in range(K):
#     dfs(0, j, T[0][j])
# print('Nothing')

'''
上記のようにdfsで全探索できるけど、実装バグらせたりする
K進数のようなリストを返してくれるようなiteratorがあると便利ね
'''


def iter_p_adic(p, length):
    '''
    連続して増加するp進数をリストとして返す。lengthはリストの長さ
    return
    ----------
    所望のp進数をリストとして返してくれるiterator
    '''
    from itertools import product
    tmp = [range(p)] * length
    return product(*tmp)


N, K = read_ints()
T = read_matrix(N)
iterator = iter_p_adic(K, N)

for idxes in iterator:
    sumxor = 0
    for i, j in zip(range(N), idxes):
        sumxor ^= T[i][j]
    if sumxor == 0:
        print('Found')
        exit()
print('Nothing')
