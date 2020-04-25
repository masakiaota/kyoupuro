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

# https://atcoder.jp/contests/abc100/tasks/abc100_d
# 定式化 M個のidxの集合idsに対して、
# max(|sum_ids (x_i)| +|sum_ids (y_i)| + |sum_ids (z_i)|) という問題
# 各項について正負を仮定すると、どのケーキを取ればいいのか優先度が決まる(ソート)
# bit全探索とソートの合わせ技
N, M = read_ints()
XYZ = read_tuple(N)

ans = -1 * 10**13 - 100
for is_mina_x, is_mina_y, is_mina_z in product(range(2), repeat=3):
    is_mina_x = -1 if is_mina_x else 1
    is_mina_y = -1 if is_mina_y else 1
    is_mina_z = -1 if is_mina_z else 1

    XYZ.sort(key=lambda x: is_mina_x *
             x[0] + is_mina_y * x[1] + is_mina_z * x[2], reverse=True)
    x_sum = 0
    y_sum = 0
    z_sum = 0
    for x, y, z in XYZ[:M]:
        x_sum += x
        y_sum += y
        z_sum += z
    ans = max(ans, abs(x_sum) + abs(y_sum) + abs(z_sum))

print(ans)
