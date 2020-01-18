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


N = read_a_int()
A = read_ints()
B = read_ints()
candi = set()
for n in range(1 << N):
    tmp = []
    flg = False
    for j in range(N):
        AorB = n >> j
        if AorB == 1:
            now = A[j]
        else:
            now = B[j]

        pre = now
        tmp.append(now)
    tmp.sort()
    candi.add(tuple(tmp))


print(candi)  # ありえる並び

