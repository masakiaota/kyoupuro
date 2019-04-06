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


T = read_col(5, 1)[0]

# 10に丸め込んで、一番最後に小さい値を求めれば良い

ans = 0

for t in T:
    if t % 10 == 0:
        ans += t
    else:
        ans += (t // 10 + 1) * 10

# print(ans)

tmp = [x % 10 for x in T if x % 10 != 0]

if tmp == []:
    ans += 10
else:
    ans += min(tmp)

print(ans-10)
