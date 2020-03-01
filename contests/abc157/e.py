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

# ポイント
# 文字列update
# 文字種の高速取得

# 文字種の高速取得について考えてみる
# セグ木ってやつか？
# 文字列を数列に置き換えてみる


# セグ木で集合をマージしていく？

N = read_a_int()
S = read()[:-1]
Q = read_a_int()


n = N
A = [s - ord('a') for s in S]
# いや、セグ木じゃないかも


def init_max(init_max_val):
    # set_val
    for i in range(n):
        seg_max[i + num_max - 1] = init_max_val[i]
    # built
    for i in range(num_max - 2, -1, -1):
        seg_max[i] = max(seg_max[2 * i + 1], seg_max[2 * i + 2])


def update_max(k, x):
    k += num_max - 1
    seg_max[k] = x
    while k:
        k = (k - 1) // 2
        seg_max[k] = max(seg_max[k * 2 + 1], seg_max[k * 2 + 2])


def query_max(p, q):
    if q <= p:
        return ide_ele_max
    p += num_max - 1
    q += num_max - 2
    res = ide_ele_max
    while q - p > 1:
        if p & 1 == 0:
            res = max(res, seg_max[p])
        if q & 1 == 1:
            res = max(res, seg_max[q])
            q -= 1
        p = p // 2
        q = (q - 1) // 2
    if p == q:
        res = max(res, seg_max[p])
    else:
        res = max(max(res, seg_max[p]), seg_max[q])
    return res


#####単位元######
ide_ele_max = -1

# num_max:n以上の最小の2のべき乗
num_max = 2**(n - 1).bit_length()
seg_max = [ide_ele_max] * 2 * num_max

init_max(A)
