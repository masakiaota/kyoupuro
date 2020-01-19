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


# 累積最大値で逆から見ていく
N = read_a_int()
P = read_ints()

# from itertools import accumulate

# P_accum = list(accumulate(P, func=max))
# print(P_accum)
# ans = 0
# for i in range(N - 1, 0, -1):
#     print(i, P[i], P_accum[i - 1])
#     if P[i] <= P_accum[i - 1]:
#         ans += 1
# print(ans)
ans = 0
MIN = P[0]
for p in P:
    if MIN >= p:
        MIN = p
        ans += 1

print(ans)
