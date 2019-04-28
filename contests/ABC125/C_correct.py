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
A = read_ints()

from fractions import gcd
# 累積GCDを使って解く
# 累積の概念は広く使えるxor,min,max,積等...
# 差ではなく左右から挟み込んで演算という概念をしっかりと身につけよう

left = []
right = []
tmp_fo = 0
tmp_re = 0
for fo, re in zip(A, A[::-1]):
    tmp_fo = gcd(tmp_fo, fo)
    tmp_re = gcd(tmp_re, re)
    left.append(tmp_fo)
    right.append(tmp_re)

right = right[::-1]

ans = 0
for i in range(1, N-1):
    ans = max(ans, gcd(left[i - 1], right[i + 1]))

# 端の処理に注意
ans = max(ans, right[1], left[-2])

print(ans)
