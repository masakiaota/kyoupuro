# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


def read_matrix(H):
    '''
    H is number of rows
    '''
    return [list(map(int, read().split())) for _ in range(H)]


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_col(H, n_cols):
    '''
    H is number of rows
    n_cols is number of cols

    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


N, M = read_ints()
# グラフ構造が与えられたとき
from collections import defaultdict
light = []
for i in range(M):
    S = read_ints()
    light.append([s-1 for s in S[1:]])
P = read_ints()

# print(light)
n = N

ans = 0  # すべての電球がつくパターン
for i in range(1 << n):
    # スイッチの付き方のパターンを2進数で全探索
    for l_idx, l in enumerate(light):
        # 個々のlightについて探索
        ltmp = 0  # ついているスイッチの数
        for j in l:
            if (i >> j) % 2:
                # もしビットが1だったらswitch on
                ltmp += 1
        if ltmp % 2 != P[l_idx]:
            # 電球がつかなかった
            break
    else:
        ans += 1


print(ans)
