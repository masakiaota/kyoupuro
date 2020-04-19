# https://atcoder.jp/contests/abc107/tasks/abc107_b
# 愚直かな

import sys
sys.setrecursionlimit(1 << 25)
ra = range
enu = enumerate

read = sys.stdin.readline


def read_map_as(H, replace={'#': 1, '.': 0}):
    '''
    文字列のmapを置換して読み込み。デフォでは#→1,.→0
    '''
    ret = []
    for _ in range(H):
        ret.append([replace[s] for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


H, W = map(int, input().split())
A = read_map_as(H)
import numpy as np
A = np.array(A)
# 削除する行の取得
del_i = set()
for i in range(H):
    if A[i].sum() == 0:
        del_i.add(i)

del_j = set()
for j in range(W):
    if A[:, j].sum() == 0:
        del_j.add(j)

# print
for i in range(H):
    if i in del_i:
        continue
    for j in range(W):
        if j in del_j:
            continue
        print('#' if A[i, j] == 1 else '.', end='')
    print()
