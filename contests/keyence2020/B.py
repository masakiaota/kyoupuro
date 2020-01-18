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


# 区間スケジューリングか？
N = read_a_int()
ST = []
for _ in range(N):
    x, l = read_ints()
    ST.append((x - l, x + l))

from operator import itemgetter
ST.sort(key=itemgetter(1))

# 区間スケジューリングを適応していく
pret = ST[0][1]
ans = 0
for s, t in ST:
    if s < pret:
        continue
    ans += 1
    pret = t
print(ans + 1)
