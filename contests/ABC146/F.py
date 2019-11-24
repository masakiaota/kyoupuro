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
S = read()[:-1]
S = S[::-1]
ans = []
cur_i = 0
while True:
    dice = M
    cur_i += dice
    if cur_i > N:
        if N - (cur_i - M) == 0:
            pass
        else:
            ans.append(N - (cur_i - M))
        break
    # if cur_i == N + 1:
    #     ans.append(cur_i - N)
    #     break
    while S[cur_i] == '1':
        cur_i -= 1
        dice -= 1
        if dice == 0:
            print(-1)
            exit()
    ans.append(dice)
print(*ans[::-1])
