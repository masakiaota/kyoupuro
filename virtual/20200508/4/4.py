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


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right

# https://atcoder.jp/contests/abc153/tasks/abc153_e
# 個数無制限ナップサック問題

H, N = read_ints()
A, B = read_col(N)


'''
dp[i][j] ... 与えられるダメージの最大。[0,i)の魔法で、魔力j以下使うとき。
更新則
dp[i+1][j] = max(dp[i][j],dp[i+1,j-B[i]] + A[i])
∵i番目を使わないとき、i番目の魔法をもう一回使うとき でダメージの最大の方を選択する
初期条件
dp[0][0] = 0 #なくても大丈夫そう


これだとjが大きすぎてTLEする
'''

'''
dp[i][j] ... 魔力の最小値。[0,i)の魔法で、ちょうどダメージjにできるとき。ちょうどjにできなければINF
更新則 #配るDPのほうが実装が簡単だけど練習で
dp[i+1][j] = min(dp[i][j],dp[i+1,j-A[i]] + B[i]) で良さそうじゃない？
∵i番目を使わないとき、i番目の魔法をもう一回使うとき でダメージの最大の方を選択する
初期条件
dp[i][0] = 0
'''


J = max(2 * H + 1, 2 * max(B))
dp = [[INF] * (J) for _ in range(N + 1)]  # Hの二倍も取れば十分かな？
for i in range(N + 1):
    dp[i][0] = 0

for i in range(N):
    for j in range(J):
        dp[i + 1][j] = min(dp[i + 1][j], dp[i][j])
        if j - A[i] >= 0:
            dp[i + 1][j] = min(dp[i + 1][j], dp[i + 1][j - A[i]] + B[i])

ans = INF
for x in dp[N][H:]:
    ans = min(ans, x)

print(ans)
