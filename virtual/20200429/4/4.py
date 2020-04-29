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

# https://atcoder.jp/contests/abc048/tasks/arc064_a
N, x = read_ints()
A = read_ints()

# 前処理ですべてx以下にしてから
# 漸化式 a[i+1] <= x-a[i]を満たすようにa[i+1]を決定していく(等号が成り立つときが一番いい)
# 右からやったり左からやったときに結果って変わらないのかな...?(大丈夫、無駄に多く減らすことはない)

# 前処理
ans = 0
for i in range(N):
    ans += max(A[i] - x, 0)
    A[i] = min(A[i], x)

# 漸化式を解く
for i in range(N - 1):
    new = x - A[i]
    if A[i + 1] <= new:
        continue  # もともと条件は満たされている
    ans += A[i + 1] - new
    A[i + 1] = new  # x>=A[i]なので大丈夫
print(ans)
