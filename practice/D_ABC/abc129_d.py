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


def full(shape, full_value):
    if isinstance(shape, tuple):
        sha = shape[::-1]
        ret = [full_value] * sha[0]
        for s in sha[1:]:
            ret = [ret.copy() for i in range(s)]
        return ret
    else:
        return [full_value] * shape


'''
例
4 6
#..#..
.x...#
....#.
#.#...

明かりは一つだけおく。
全探索なら4*10**6 でギリ間に合わなさそう?

前処理で縦にいくつ照らせるか事前にわかるのでは?(だめ、わからん)

ある点を調べたときにそこが縦にいくつ伸ばせるのか更新していくのは?
→良さそう

'''
H, W = read_ints()
S = read_map(H)
# まわりを壁でfill
MAP = ['#' * (W + 2)]
for s in S:
    MAP.append('#' + s + '#')
MAP.append('#' * (W + 2))


tate = []
yoko = []
for _ in range(H + 2):
    tmp = [0] * (W + 2)
    tate.append(tmp)
for _ in range(H + 2):
    tmp = [0] * (W + 2)
    yoko.append(tmp)

# yokoの構築
for i in range(H + 2):
    pre = '#'
    for j in range(1, W + 2):
        if pre == '#' and MAP[i][j] == '.':
            s = j
            pre = '.'
        if pre == '.' and MAP[i][j] == '#':
            t = j
            cnt = t - s
            pre = '#'
            for k in range(s, t):
                yoko[i][k] = cnt

# tateの構築
for j in range(W + 2):
    pre = '#'
    for i in range(1, H + 2):
        if pre == '#' and MAP[i][j] == '.':
            s = i
            pre = '.'
        if pre == '.' and MAP[i][j] == '#':
            t = i
            cnt = t - s
            pre = '#'
            for k in range(s, t):
                tate[k][j] = cnt

ans = 0
from itertools import product
for i, j in product(range(H + 2), range(W + 2)):
    if MAP[i][j] == '#':
        continue
    ans = max(ans, tate[i][j] + yoko[i][j] - 1)

print(ans)
