# https://atcoder.jp/contests/abc084/tasks/abc084_d
# エラトステネスの篩を使う以外に考えられないが...
# あと高速化のために累積和も

import sys
read = sys.stdin.readline


def read_a_int():
    return int(read())


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


def ret_erators(N: int):
    # エラトステネスの篩
    is_prime = [True] * (N + 1)

    # 0と1は素数ではない
    is_prime[0] = False
    is_prime[1] = False

    for i in range(2, int(N**0.5) + 1):
        if is_prime[i]:
            j = i * 2  # iの倍数は素数ではない
            while j < N + 1:
                is_prime[j] = False
                j += i
    return is_prime


is_prime = ret_erators(10 ** 5)

# make is_like2017
is_like = [0] * (10**5 + 1)
for x in range((10 ** 5) // 2 + 1):
    if not is_prime[x]:
        continue
    n = 2 * x - 1
    if is_prime[n]:
        is_like[n] = 1

# make accum
from itertools import accumulate
is_like_accum = [0] + list(accumulate(is_like))

Q = int(input())
L, R = read_col(Q, 2)


for l, r in zip(L, R):
    print(is_like_accum[r + 1] - is_like_accum[l])
