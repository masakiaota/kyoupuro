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

# 最初にすべてのマイナスが外せないか試して、
# abs最小の数に-をつけて合計すればいいんだ！

# 問題はどうやってすべてのマイナスがはずせるか判別するか
# +-+,---っていうパターンがあったらアウト
# flg = False
minuscount = 0
for a in A:
    if a < 0:
        minuscount += 1


anstmp = sum([abs(a) for a in A])
if minuscount % 2 == 0:
    print(anstmp)
    exit()

# else
mi = 10 ** 9
for a in A:
    mi = min(mi, abs(a))

print(anstmp-2*mi)
