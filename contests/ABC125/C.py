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


import numpy as np

N = read_a_int()
A = np.array(read_ints())


def gcd(a, b):
    while (b != 0):
        tmp = a
        a = b
        b = tmp % b

    return a


import math


def is_prime(n):
    if n == 1:
        return True

    for k in range(2, int(math.sqrt(n)) + 1):
        if n % k == 0:
            return False

    return True


gcd_tmp = A[0]
for a in A[1:]:
    gcd_tmp = gcd(gcd_tmp, a)

A_gcd = A//gcd_tmp

# delidx = None
for i, a in enumerate(A_gcd):
    if is_prime(a):
        delidx = i
        break


ans = A[delidx-1]
for i, a in enumerate(A[:delidx].tolist()+A[delidx+1:].tolist()):
    ans = gcd(ans, a)

print(ans)
