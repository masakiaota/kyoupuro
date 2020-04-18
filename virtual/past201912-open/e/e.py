import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


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


def read_col(H):
    '''
    H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_matrix(H):
    '''
    H is number of rows
    '''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


def read_map(H):
    '''
    H is number of rows
    文字列で与えられた盤面を読み取る用
    '''
    return [read()[:-1] for _ in range(H)]


def read_map_as_int(H):
    '''
    # →1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# 1→O(1)
# 2→O(n) フォローバック
# 3→O(n**2)
# 最大 500 * 10000 → 10**7 間に合う

N, Q = read_ints()
# 隣接行列
adj_mat = [[0] * N for _ in range(N)]  # フォローi→jを1とする


def f1(tmp):
    a, b = tmp
    adj_mat[a - 1][b - 1] = 1


def f2(tmp):
    a = tmp[0] - 1
    # x→a で1になってるxに対して,a→xを1にする
    candi = []
    for x in range(N):
        if adj_mat[x][a] == 1:
            candi.append(x)
    for x in candi:
        adj_mat[a][x] = 1


def f3(tmp):
    a = tmp[0] - 1
    # a→xが1のx、x→yが1のyについてa→yに1をする
    candi = []
    for x in range(N):
        if adj_mat[a][x] != 1:
            continue
        candi.append(x)
    candi2 = []
    for x in candi:
        for y in range(N):
            if adj_mat[x][y] == 1:
                candi2.append(y)
    for y in candi2:
        if a == y: #ここに気づいてほしい
            continue
        adj_mat[a][y] = 1


for _ in range(Q):
    com, *tmp = read_ints()
    if com == 1:
        f1(tmp)
    elif com == 2:
        f2(tmp)
    else:
        f3(tmp)

# 結果を出力
for adj in adj_mat:
    ans = []
    for a in adj:
        ans.append('Y' if a == 1 else 'N')
    print(''.join(ans))
