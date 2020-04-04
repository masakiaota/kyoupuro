# https://atcoder.jp/contests/abc098/tasks/arc098_a

import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
rr = range


def read_ints():
    return list(map(int, read().split()))


def read_a_int():
    return int(read())


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

# 要は各地点で向きが揃っている人数をカウントすれば良い
# 0番目がリーダーのとき、向きが揃っている人はWのカウントとなる(あとはリーダーの文字次第)
# i番目を++するとき、[0,i)は向きが揃っているひとをカウントできる、[i+1,N)で向きが揃っているのはWが登場するたび引けば良い


N = read_a_int()
S = read()[:-1]
W = 'W'
E = 'E'

l = 0  # iの左側でEを向いている人数
r = S.count(W)  # iの右側でWを向いている人数

ans = 10**9
for s in S:
    if s == W:  # 右を更新
        r -= 1

    # 答えを更新
    ans = min(ans, N - 1 - (r + l))  # -1は自分のコストを考えない分

    if s == E:  # 左を更新
        l += 1

print(ans)
