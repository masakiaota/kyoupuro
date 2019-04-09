# https://atcoder.jp/contests/joi2011yo/tasks/joi2011yo_e

# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


def read_col(H, n_cols):
    '''
    A列、B列が与えられるようなとき
    '''
    ret = [[] for _ in range(n_cols)]
    for _ in range(H):
        tmp = list(map(int, read().split()))
        for col in range(n_cols):
            ret[col].append(tmp[col])

    return ret


print(read_col(4, 2))
