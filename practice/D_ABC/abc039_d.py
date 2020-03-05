# https://atcoder.jp/contests/abc039/tasks/abc039_d
# 収縮の作業の逆をすれば良い。つまり.と接する#を.に置換して、収縮を再度行ったときに、画像が異なるようであればimpossible(?)

import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
    return ret


from itertools import product
H, W = read_ints()
S = read_map_as_int(H)
mv = [(0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1), (1, 0), (1, 1)]
newS = [[-1] * W for _ in range(H)]
for i, j in product(range(H), range(W)):
    if S[i][j] == 0:
        newS[i][j] = 0
        continue
    tmp = 1  # 単位元
    for di, dj in mv:
        ii = i + di
        jj = j + dj
        if ii < 0 or H <= ii or jj < 0 or W <= jj:
            continue
        tmp *= S[ii][jj]
    if tmp == 1:  # すべて#に囲まれている
        newS[i][j] = 1
    else:
        newS[i][j] = 0

# これを復元したときにもとに戻るか
fukugen = [[0] * W for _ in range(H)]
for i, j in product(range(H), range(W)):
    if newS[i][j] == 1:
        fukugen[i][j] = 1
        for di, dj in mv:
            ii = i + di
            jj = j + dj
            if ii < 0 or H <= ii or jj < 0 or W <= jj:
                continue
            fukugen[ii][jj] = 1


def print_result():
    for s in newS:
        print(*['#' if ss == 1 else '.'for ss in s], sep='')


import numpy as np
if np.logical_xor(fukugen, S).sum() == 0:
    print('possible')
    print_result()
else:
    print('impossible')
