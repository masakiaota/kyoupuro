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


def factorization(n: int):
    if n == 1:
        return []  # 1は素数ではない
    # 素因数分解
    arr = []
    temp = n
    for i in range(2, int(n**0.5) + 1):  # ここにバグがないか心配
        if temp % i == 0:
            cnt = 0
            while temp % i == 0:
                cnt += 1
                temp //= i
            arr.append((i, cnt))

    if temp != 1:
        arr.append((temp, 1))

    if arr == []:
        arr.append((n, 1))

    return arr


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
# https://atcoder.jp/contests/abc114/tasks/abc114_d

N = read_a_int()

# N!を素因数分解形式で表示する
# その結果ni>=74となる 因数の個数
# +
# ni >= 24 かつ nj >=2となる因数の組の個数
# +
# ni >= 14 かつ nj >=4となる因数の組の個数
# +
# ni >= 4 かつ nj >=4 かつ nk >=2 となる因数の組の個数 (ここで4以上は順序の区別なく選ぶ必要があるのに脳内でいろいろ混線してバグらせた)
# が答えのはず
# 素数は少ないので全探索できそう

N_fact = defaultdict(lambda: 0)
for i in ra(1, N + 1):
    tmp = factorization(i)
    for k, v in tmp:
        N_fact[k] += v

ans = 0
values = list(N_fact.values())
for v in values:
    if v >= 74:
        ans += 1

# print(ans)
for i, j in product(range(len(values)), repeat=2):
    if i == j:
        continue
    if values[i] >= 24 and values[j] >= 2:
        ans += 1
    if values[i] >= 14 and values[j] >= 4:
        ans += 1


# ここまで多分ok

# print(ans)
for i, j in combinations(range(len(values)), r=2):  # i,jはダブらないように選ぶ
    for k in range(len(values)):
        if i == k or j == k:
            continue
        if values[i] >= 4 and values[j] >= 4 and values[k] >= 2:
            ans += 1

print(ans)
