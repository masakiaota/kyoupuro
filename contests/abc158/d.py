# 入力が10**5とかになったときに100ms程度早い
import sys
read = sys.stdin.readline


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


def read_map_as_int(H):
    '''
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
    return ret


from collections import deque
S = deque(read()[:-1])
Q = read_a_int()


is_flip = 0  # 0は反転してない
for _ in range(Q):
    q = input().split()
    if len(q) == 1:
        is_flip = 1 - is_flip
    else:
        _, f, c = q
        f = int(f)
        if is_flip:
            if f == 1:
                S.append(c)
            else:
                S.appendleft(c)
        else:  # flipしてないときは通常の処理
            if f == 1:
                S.appendleft(c)
            else:
                S.append(c)

if is_flip:
    print(''.join(S)[::-1])
else:
    print(''.join(S))
