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

# https://atcoder.jp/contests/abc113/tasks/abc113_d
# おっこの間やったばかり

H, W, K = read_ints()
if W == 1:
    print(1)
    exit()  # コーナーケース
# DP
# dp[h][k] ... 0からスタートしたときに,[0,h)の横線まで考慮して、k番目に到達できる通りの総数
# 繊維は2**8を全探索 ただし、条件を満たさないやつは注意する


dp = [[0] * W for _ in ra(H + 1)]
dp[0][0] = 1


# swapする要素のidxのペアを返す
def check_valid(p):
    # 1が連続してはいけない
    for pp, ppp in zip(p, p[1:]):
        if pp == 1 and ppp == 1:
            return False
    return True


def generate_ij_to_swap():
    ret = []
    for p in product(range(2), repeat=W - 1):
        if not check_valid(p):
            continue
        pre = False
        for i, pp in enu(p):
            if pp:
                ret.append((i, i + 1))
                ret.append((i + 1, i))
            elif pre == 0 and pp == 0:
                ret.append((i, i))  # つながってないやつ
            pre = pp
        if pre == 0:
            ret.append((i + 1, i + 1))
    return ret


swap_idx = generate_ij_to_swap()


for h in ra(H):
    for i, j in swap_idx:
        dp[h + 1][j] += dp[h][i] % MOD  # 遷移の更新

print(dp[H][K - 1] % MOD)
