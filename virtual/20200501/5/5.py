
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
# https://atcoder.jp/contests/arc093/tasks/arc093_b

# 条件を満たすものを出力しろ
# 連結成分が指定されている
# 100*100のマスに限定して考えれば簡単。
# 50*100のマスの中で499個の島を作ることはできるよね？(2行あたり25個作れるはず, 40行もあれば十分作れる)

A, B = read_ints()
A -= 1
B -= 1
print(100, 100)

# 黒の中に白の個島を作る
for _ in range(25):
    for _ in range(50):
        if A > 0:
            print('#.', end='')
            A -= 1
        else:
            print('##', end='')
    print()
    print('#' * 100)  # 隔離

# 白の中に黒の個島を作る
for _ in range(25):
    print('.' * 100)  # 隔離
    for _ in range(50):
        if B > 0:
            print('#.', end='')
            B -= 1
        else:
            print('..', end='')
    print()
