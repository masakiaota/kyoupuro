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
R, S, P = read_ints()
T = read()[:-1]

points = {
    'r': P,
    's': R,
    'p': S
}

# hands = {
#     'r': 'p',
#     's': 'r',
#     'p': 's'
# }

handsls = []

ans = 0
for i in range(N):
    now = T[i]
    if i < K:
        ans += points[now]
        handsls.append(now)
    else:
        if now == handsls[i - K]:
            handsls.append('n')
            continue
        ans += points[now]
        handsls.append(now)
        # print(T[i], points[now])
# print(handsls)
print(ans)
