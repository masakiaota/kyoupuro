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


from bisect import bisect_left, bisect_right
N, M, V, P = read_ints()
A = read_ints()
A.sort()
edge_idx = N - P
edge = A[-P]
# ここに処理を書けば終わりだ
# M人がちょうどV配ることによる副作用の扱いがわからん...
Mls = [M] * N
print(A)
left_point = V * M
for i in range(edge_idx - 1, -1, -1):
    sa = edge - A[i]
    if sa > left_point:
        break
    # 代入処理
    sa = min(sa, M)
    left_point -= sa
    Mls[i] -= sa
    A[i] = edge


mina = 0
# print(left_point)
if left_point > 0:
    for i, m in enumerate(Mls):
        if m == 0:
            mina += 1
            continue
        else:
            if left_point - m * (N - mina) < 0:
                left_point = m * (N - mina) +  # ああ実装ぐちゃぐちゃでわからん
                break
            elif left_point == 0:
                mina += 1
                break
            else:
                mina += 1

print(mina, left_point)
print(A)
print(Mls)
lower_bound_idx = bisect_left(A, edge)
# upper_bound_idx = bisect_right(A, edge)
for i in range(lower_bound_idx, lower_bound_idx + left_point):
    A[i] += 1
A.sort()

edge = A[-P]
lower_bound_idx = bisect_left(A, edge)
n_candi = edge_idx - lower_bound_idx

print(P + n_candi - mina)
