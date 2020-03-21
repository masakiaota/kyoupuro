
import sys
sys.setrecursionlimit(1 << 25)
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
    #→1,.→0として読み込む
    '''
    ret = []
    for _ in range(H):
        ret.append([1 if s == '#' else 0 for s in read()[:-1]])
        # 内包表記はpypyでは若干遅いことに注意
        # #numpy使うだろうからこれを残しておくけど
    return ret


MOD = 10**9 + 7
# default import
from itertools import product, permutations, combinations
H, W = read_ints()
S = read_map_as_int(H)  # が1 .が0

dp = [[100000] * (W) for _ in range(H)]  # -1アクセス用に余分に取っておく
# 初期化
dp[0][0] = S[0][0]

for i, j in product(range(H), range(W)):
    if i == 0 and j == 0:
        continue
    if i != 0:
        dp[i][j] = min(dp[i][j], dp[i - 1][j]
                       + (1 if S[i][j] == 1 and S[i - 1][j] == 0 else 0))
    if j != 0:
        dp[i][j] = min(dp[i][j], dp[i][j - 1]
                       + (1 if S[i][j] == 1 and S[i][j - 1] == 0 else 0))

print(dp[-1][-1])
