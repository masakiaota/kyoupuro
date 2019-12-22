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
import sys
sys.setrecursionlimit(1 << 25)


def f(n):
    if n < 2:
        return 1
    return n * f(n - 2)


a = f(N)
cnt = 0
while a % 10 == 0:
    cnt += 1
    a = a // 10
print(cnt)


def print_f(n):
    a = f(n)
    cnt = 0
    while a % 10 == 0:
        cnt += 1
        a = a // 10
    print(n, cnt)


for i in range(10, 501, 10):
    print_f(i)
