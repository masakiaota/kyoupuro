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


S = read()[:-1]

flg1, flg2 = False, False

first = int(S[:2])
last = int(S[2:])

# print(first, last)
if 0 < first < 13:
    flg1 = True

if 0 < last < 13:
    flg2 = True

if flg1 and flg2:
    print('AMBIGUOUS')

if flg1 and not flg2:
    print('MMYY')

if flg2 and not flg1:
    print('YYMM')

if not flg2 and not flg1:
    print('NA')
