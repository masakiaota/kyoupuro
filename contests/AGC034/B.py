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


S = input()


section = []
tmp = []
B_flg = False
s_pre = S[0]

if s_pre == "A":
    tmp.append(1)
elif s_pre == 'B':
    B_flg = True

for s in S[1:]:
    if s == 'A':
        tmp.append(1)
    elif s == 'C' and B_flg:
        tmp.append(0)
        # 実装がgaijiすぎる

    if s == 'B':
        B_flg = True
    else:
        B_flg = False

print(tmp)
