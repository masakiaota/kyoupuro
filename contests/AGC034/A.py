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


N, A, B, C, D = read_ints()
S = input()

A -= 1
B -= 1
C -= 1
D -= 1
# 大小関係の実装
plan = 1  # ひっくり返らない処理が1,ヒックリ変えるのが2、領域が二分割されるのが3

if D < C:
    plan = 2

if C < B:
    plan = 3

from sys import exit

# 入れ替わるかの条件ごとに実装する
if plan == 1:
    for s_left, s_right in zip(S[A:], S[A + 1:D]):
        if s_left == '#' and s_right == '#':
            print('No')
            exit()
    print('Yes')
elif plan == 2:
    # 入れ替わる場
    for s_left, s_right in zip(S[A:], S[A + 1:C]):
        if s_left == '#' and s_right == '#':
            print('No')
            exit()

    for s_left, s_center, s_right in zip(S[B-1:D], S[B:D+1], S[B+1:D+2]):
        if s_left == '.' and s_center == '.' and s_right == '.':
            print('Yes')
            exit()
    print('No')
elif plan == 3:
    # print(plan)
    # AC,BD間に##の存在を確かめれば良い
    for s_left, s_right in zip(S[A:], S[A+1:C]):
        if s_left == '#' and s_right == '#':
            print('No')
            exit()
    for s_left, s_right in zip(S[B:], S[B+1:D]):
        if s_left == '#' and s_right == '#':
            print('No')
            exit()
    print('Yes')
