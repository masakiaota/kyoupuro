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
    #→1,.→0として読み込む
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


# [1:m) の商品について、商品jが1つでもあるときの最小金額
# dp[i][state] ... S[:i]とC[:i]を考慮したときのstateになる最小金額
# where state → 商品jが少なくとも1つは含まれているときにjbit目に1が立っている数字

# dp[i+1][next_state] ... dp[i][state]に関して、遷移はstate|S(をbitにしたもの)
# 配るDPのほうが実装が楽かな

# dp[i][state] = INF
# dp[i+1][state|S(のbit)] = min(dp[i+1][state|S(のbit)], dp[i][state|S(のbit)]) #i番目の商品を取らないとき
# dp[i+1][state|S_i(のbit)] = min(dp[i+1][state|S(のbit)], dp[i][state] + C_i) #i番目の商品を取るとき

N, M = read_ints()


def encode(s: str):
    '''YYY→111(2bit)にしたい'''
    ret = 0
    for i, ss in enu(s):
        if ss == 'Y':
            ret += 1 << i
    return ret


S = []
C = []
for _ in ra(M):
    s, c = read().split()
    S.append(encode(s))
    C.append(int(c))

# dp tableの作成
max_state = pow(2, N)
dp = [[INF] * max_state for _ in range(M + 1)]
# 初期化
dp[0][0] = 0  # 初期では一つの品物もない状態を0円で買うのが最適でそれ以外はありえないのでINFのまま

for i, state in product(range(M), range(max_state)):
    # i番目の商品を使わない場合の最小値
    dp[i + 1][state] = min(dp[i + 1][state], dp[i][state])
    # i番目の商品を使う場合の最小値
    dp[i + 1][state | S[i]] = min(dp[i + 1][state | S[i]], dp[i][state] + C[i])

# from pprint import pprint
# pprint(dp)
print(-1 if dp[-1][-1] == INF else dp[-1][-1])
