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
    return [read() for _ in range(H)]


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
S = read()[:-1]


# strategy 1
for i, s in enumerate(S):
    if s == '#':
        S = S[i:]
        break
else:
    print(0)
    exit()

ans = 0
# tmp = ['#' for _ in range(N-i)]
for i, s in enumerate(S):
    if s == '.':
        ans += 1
print(ans)


# strategy2
# 例えば、すべて
# ...####でも良い
# #.##. str1では2だが、str2でも2となる

# #.##.#


# ....#####を作るのに最適な位置を知りたい
#
