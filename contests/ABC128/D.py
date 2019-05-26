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


N, K = read_ints()
V = read_ints()

V_nonnegative = [max(v, 0) for v in V]

# 端から合計K個の要素の和が一番大きい部分を探す
sumtmp = sum(V_nonnegative[:K])
max_tmp = sumtmp
key_idx = 0
for i in range(1, K):
    sumtmp = sumtmp - V_nonnegative[K - i - 1] + V_nonnegative[-i]
    if sumtmp > max_tmp:
        key_idx = i
        max_tmp = sumtmp

# 大きい部分を切り出す
left = V[:K-i+1]
right = V[-i + 1:]
# print(left, right)

# マイナスを消していく

for k in range(K):
    tmp_del = min(left + right)
    if tmp_del == left[0]:
        tmp_edge = min(left[1], right[-1])
    elif tmp_del == right[-1]:
        tmp_edge = min(left[0], right[-2])
    else:
        tmp_edge = min(left[0], right[-1])

    if tmp_del+tmp_edge < 0:
        try:
            left.remove(tmp_del)
        except:
            right.remove(tmp_del)
        if tmp_edge == left[0]:
            del left[0]
        else:
            del right[-1]

print(left, right)

print(sum(left+right))
