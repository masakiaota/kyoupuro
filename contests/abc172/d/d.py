import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def exit(*argv, **kwarg):
    print(*argv, **kwarg)
    sys.exit()


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def a_int(): return int(read())


def ints(): return list(map(int, read().split()))


def read_col(H):
    '''H is number of rows
    A列、B列が与えられるようなとき
    ex1)A,B=read_col(H)    ex2) A,=read_col(H) #一列の場合'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return tuple(map(list, zip(*ret)))


def read_tuple(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(tuple(map(int, read().split())))
    return ret


def read_matrix(H):
    '''H is number of rows'''
    ret = []
    for _ in range(H):
        ret.append(list(map(int, read().split())))
    return ret
    # return [list(map(int, read().split())) for _ in range(H)] # 内包表記はpypyでは遅いため


MOD = 10**9 + 7
INF = 2**31  # 2147483648 > 10**9
# default import
from collections import defaultdict, Counter, deque
from operator import itemgetter, xor, add
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from functools import reduce
from math import gcd


def lcm(a, b):
    # 最小公倍数
    g = gcd(a, b)
    return a // g * b

# エラトステネスの篩の篩みたいに約数の個数をどんどん作っていくのは？


def ret_eratos(N: int):
    '''エラトステネスの篩'''
    '''is_primeは約数の個数'''
    is_prime = [0] * (N + 1)
    is_prime[0] = -2  # 0と1は素数ではない
    is_prime[1] = -1
    for i in range(2, N // 2 + 1):
        for j in range(i * 2, N + 1, i):  # iの倍数は素数でない
            is_prime[j] += 1
    for i in range(N + 1):
        is_prime[i] += 2
    return is_prime


N = a_int()
eratos = ret_eratos(N)
# print(eratos)
ans = 0
for k in range(1, N + 1):
    ans += k * eratos[k]
print(ans)

# でも多分TLEだなぁ
