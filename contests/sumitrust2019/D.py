# pypyで通った
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
S = read()[:-1]


def permutation_3to2(new3num: str):
    return (new3num[1:], new3num[0] + new3num[2])


passes = set()
set_2num = set()
passes.add(S[:3])
set_2num.add(S[:2])
tmp1, tmp2 = permutation_3to2(S[:3])
set_2num.add(tmp1)
set_2num.add(tmp2)
for c in S[3:]:
    for a, b in list(set_2num):
        tmp = a + b + c
        tmp1, tmp2 = permutation_3to2(tmp)
        set_2num.add(tmp1)
        set_2num.add(tmp2)
        passes.add(tmp)

print(len(passes))
