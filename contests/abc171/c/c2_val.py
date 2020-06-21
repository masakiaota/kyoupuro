import sys
sys.setrecursionlimit(1 << 25)
read = sys.stdin.readline
ra = range
enu = enumerate


def mina(*argv, sub=1): return list(map(lambda x: x - sub, argv))
# 受け渡されたすべての要素からsubだけ引く.リストを*をつけて展開しておくこと


def read_a_int(): return int(read())


def read_ints(): return list(map(int, read().split()))


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
from operator import itemgetter
from itertools import product, permutations, combinations
from bisect import bisect_left, bisect_right  # , insort_left, insort_right
from math import gcd
# 文字の順番を扱うとき


def ord_from_a(char):
    return ord(char) - ord('a')


def chr_from_a(n: int):
    # nはaから何番あとかを示す
    if n == -1:
        n = 25
    return chr(n + ord('a'))


# N = read_a_int()
# 26進数のときにN番目の犬の名前は？
# 0の扱いが面倒

def solve(N):
    ans = ''
    for i in range(1, 10):
        if N < (26 ** i - 1) // 25:
            break  # 桁数の決定
    n = i - 1
    N -= (26 ** n - 1) // 25
    # print(n)
    ans = ''

    for i in range(n):
        N, r = divmod(N, 26)
        ans = chr_from_a(r) + ans

    return ans


import string


def correct(N):
    alphabet = list(string.ascii_lowercase)
    n = N
    idxs = []
    while n > 0:
        n -= 1
        idxs.append(n % 26)
        n //= 26
    ans = ''
    for i in idxs[::-1]:
        ans += alphabet[i]
    return ans


from random import randint

abc = 'abcdefghijklmnopqrstuvwxyz'
for _ in range(10 ** 7):
    n = randint(1, 10**15 + 1)
    print('verify for', n)
    myans = solve(n)
    correctans = correct(n)
    assert myans == correctans, (n, myans, correctans)
