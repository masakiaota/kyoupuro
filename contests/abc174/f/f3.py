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
# Python program to compute sum of ranges for different range queries

import math

# Function that accepts array and list of queries and print sum of each query


def queryResults(arr, Q):

    # Q.sort(): # Sort by L
    # sort all queries so that all queries in the increasing order of R values .
    Q.sort(key=lambda x: x[1])

    # Initialize current L, current R and current sum
    currL, currR, currSum = 0, 0, set()

    # Traverse through all queries
    for i in range(len(Q)):
        L, R = Q[i]  # L and R values of current range

        # Remove extra elements from previous range
        # if previous range is [0, 3] and current
        # range is [2, 5], then a[0] and a[1] are subtracted
        while currL < L:
            currSum -= {arr[currL]}
            currL += 1

        # Add elements of current range
        while currL > L:
            currSum |= {arr[currL - 1]}
            currL -= 1
        while currR <= R:
            currSum |= {arr[currR]}
            currR += 1

        # Remove elements of previous range
        # when previous range is [0, 10] and current range
        # is [3, 8], then a[9] and a[10] are subtracted
        while currR > R + 1:
            currSum -= {arr[currR - 1]}
            currR -= 1

        # Print the sum of current range
        print("Sum of", Q[i], "is", currSum)


N, Q = ints()
C = ints()
arr = C
Query = []
for _ in range(Q):
    l, r = ints()
    Query.append([l - 1, r - 1])
queryResults(arr, Query)
# This code is contributed by Shivam Singh
