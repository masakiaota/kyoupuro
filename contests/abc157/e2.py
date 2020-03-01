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


def read_tuple(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


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


def index(a, x):
    'Locate the leftmost value exactly equal to x'
    i = bisect_left(a, x)

    if i != len(a) and a[i] == x:
        return i
    raise ValueError

# ポイント
# 文字列update
# 文字種の高速取得

# 文字種の高速取得について考えてみる
# 文字のidxを取得しておいて二分探索することで高速に取得することが可能


N = read_a_int()
S = list(read()[:-1])

from collections import defaultdict
from bisect import bisect_left, bisect_right, insort_left
char_idxs = defaultdict(lambda: [])
for i, s in enumerate(S):
    char_idxs[s].append(i)


def get_syurui(char_idxs, l, r):
    ret = 0
    for v in char_idxs.values():
        l_idx = bisect_left(v, l)
        r_idx = bisect_right(v, r)
        # print(v,l_idx,r_idx)
        if r_idx - l_idx > 0:
            ret += 1
    return ret


Q = read_a_int()
for q in range(Q):
    com, a, b = read().split()
    if int(com) == 2:
        a, b = int(a) - 1, int(b) - 1
        print(get_syurui(char_idxs, a, b))
    else:
        i = int(a) - 1
        if S[i] == b:
            continue
        # i文字目を消す
        tmp = char_idxs[S[i]]  # S[i]を更新しなきゃ
        del char_idxs[S[i]][index(tmp, i)]
        # cのidxに挿入し直す
        insort_left(char_idxs[b], i)
        # S[i]を更新しなきゃ
        S[i] = b
