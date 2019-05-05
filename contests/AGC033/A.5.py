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


def read_map(H, W):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [input() for _ in range(H)]


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


H, W = read_ints()
A = read_map(H, W)

# 黒の座標
end = []
# 白の座標
start = []
for i, a in enumerate(A):
    for j, aa in enumerate(a):
        if aa == '#':
            end.append((i, j))
        else:
            start.append((i, j))


def mahattan(sy, sx, ey, ex):
    return abs(sy - ey) + abs(sx - ex)


ans = 0

for i, j in start:
    tmp = 10000
    for ey, ex in end:
        tmp = min(tmp, mahattan(i, j, ey, ex))
    ans = max(ans, tmp)

print(ans)
